use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc, Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use tauri::{ipc::Channel, AppHandle, Manager, State};

use crate::{store::AppState, tools, util};

#[derive(Default)]
pub struct LogcatState(Mutex<Option<Session>>);

struct Session {
    id: String,
    committed_bytes: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    worker: thread::JoinHandle<()>,
}

impl Session {
    fn stop(self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = self.worker.join();
    }
}

impl LogcatState {
    pub fn stop(&self) {
        if let Some(session) = self.0.lock().unwrap().take() {
            session.stop();
        }
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogcatEvent {
    lines: Vec<String>,
    finished: bool,
    error: Option<String>,
}

fn write_lines(writer: &mut impl Write, lines: &[String]) -> std::io::Result<()> {
    for line in lines {
        writeln!(writer, "{line}")?;
    }
    writer.flush()
}

fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?
        .join("logcat");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("创建日志缓存目录失败：{error}"))?;
    Ok(directory)
}

fn cache_file(app: &AppHandle, session_id: &str) -> Result<PathBuf, String> {
    crate::store::validate_id(session_id)?;
    Ok(cache_dir(app)?.join(format!("{session_id}.log")))
}

fn check_export_path(directory: &Path, path: &Path) -> Result<(), String> {
    let directory = directory
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let target = path
        .canonicalize()
        .or_else(|_| {
            path.parent()
                .unwrap_or(Path::new("."))
                .canonicalize()
                .map(|parent| parent.join(path.file_name().unwrap_or_default()))
        })
        .map_err(|error| format!("无法访问导出目录：{error}"))?;
    if target.starts_with(directory) {
        return Err("请选择日志缓存目录以外的位置导出。".into());
    }
    Ok(())
}

fn copy_snapshot(source: &Path, destination: &Path, bytes: u64) -> std::io::Result<u64> {
    let input = File::open(source)?;
    let mut output = BufWriter::new(File::create(destination)?);
    let copied = std::io::copy(&mut input.take(bytes), &mut output)?;
    output.flush()?;
    Ok(copied)
}

#[tauri::command]
pub async fn export_logcat_session(
    app: AppHandle,
    logcat: State<'_, LogcatState>,
    session_id: String,
    path: String,
) -> Result<u64, String> {
    let source = cache_file(&app, &session_id)?;
    check_export_path(&cache_dir(&app)?, Path::new(&path))?;
    let bytes = {
        let current = logcat.0.lock().map_err(|_| "logcat 状态不可用")?;
        match current.as_ref().filter(|session| session.id == session_id) {
            Some(session) => session.committed_bytes.load(Ordering::Acquire),
            None => std::fs::metadata(&source)
                .map_err(|error| format!("读取日志缓存失败：{error}"))?
                .len(),
        }
    };
    tauri::async_runtime::spawn_blocking(move || {
        copy_snapshot(&source, Path::new(&path), bytes)
            .map_err(|error| format!("导出完整日志失败：{error}"))
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn open_logcat_cache(app: AppHandle) -> Result<(), String> {
    util::open_directory(&cache_dir(&app)?)
}

#[tauri::command]
pub fn clear_logcat_cache(app: AppHandle, logcat: State<'_, LogcatState>) -> Result<(), String> {
    let current = logcat.0.lock().map_err(|_| "logcat 状态不可用")?;
    if current
        .as_ref()
        .is_some_and(|session| !session.worker.is_finished())
    {
        return Err("请先停止采集，再清理日志缓存。".into());
    }
    for entry in std::fs::read_dir(cache_dir(&app)?).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
            && path.extension().is_some_and(|extension| extension == "log")
        {
            std::fs::remove_file(path).map_err(|error| format!("清理日志缓存失败：{error}"))?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn export_logcat(app: AppHandle, path: String, text: String) -> Result<(), String> {
    check_export_path(&cache_dir(&app)?, Path::new(&path))?;
    std::fs::write(path, text).map_err(|error| format!("导出日志失败：{error}"))
}

fn read_lines(reader: impl Read, sender: mpsc::SyncSender<(bool, String)>, stderr: bool) {
    for line in BufReader::new(reader).split(b'\n') {
        match line {
            Ok(line) => {
                let line = String::from_utf8_lossy(&line)
                    .trim_end_matches('\r')
                    .to_string();
                if sender.send((stderr, line)).is_err() {
                    break;
                }
            }
            Err(error) => {
                let _ = sender.send((true, format!("读取 logcat 失败：{error}")));
                break;
            }
        }
    }
}

#[tauri::command]
pub fn start_logcat(
    app: AppHandle,
    state: State<'_, AppState>,
    logcat: State<'_, LogcatState>,
    device_id: String,
    session_id: String,
    on_event: Channel<LogcatEvent>,
) -> Result<String, String> {
    if device_id.is_empty()
        || device_id.len() > 128
        || !device_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '-'))
    {
        return Err("非法的设备标识".into());
    }
    let mut current = logcat.0.lock().map_err(|_| "logcat 状态不可用")?;
    if let Some(previous) = current.take() {
        previous.stop();
    }
    let save_path = cache_file(&app, &session_id)?;
    let mut output = BufWriter::new(
        File::options()
            .write(true)
            .create_new(true)
            .open(&save_path)
            .map_err(|error| format!("无法创建日志缓存：{error}"))?,
    );
    let mut child = util::command(tools::require_adb(&state)?)
        .args(["-s", &device_id, "logcat", "-v", "threadtime", "-T", "500"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("无法启动 logcat：{error}"))?;
    let committed_bytes = Arc::new(AtomicU64::new(0));
    let worker_bytes = committed_bytes.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_cancel = cancel.clone();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let worker = thread::spawn(move || {
        let (sender, receiver) = mpsc::sync_channel(2048);
        let errors = sender.clone();
        thread::spawn(move || read_lines(stdout, sender, false));
        thread::spawn(move || read_lines(stderr, errors, true));
        let mut detail = String::new();
        let mut exit_status = None;
        let mut failure = None;
        let mut cancelled = false;
        let mut connected = true;
        loop {
            if !cancelled && worker_cancel.load(Ordering::Relaxed) {
                cancelled = true;
                let _ = child.kill();
                exit_status = child.wait().ok();
            }
            let mut lines = Vec::new();
            let mut disconnected = false;
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(line) => {
                    let mut batch = vec![line];
                    batch.extend(receiver.try_iter().take(255));
                    for (stderr, line) in batch {
                        if stderr {
                            detail = line;
                        } else {
                            lines.push(line);
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => disconnected = true,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if let Err(error) = write_lines(&mut output, &lines) {
                failure = Some(format!("保存日志失败：{error}"));
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            worker_bytes.fetch_add(
                lines.iter().map(|line| line.len() as u64 + 1).sum::<u64>(),
                Ordering::Release,
            );
            if connected
                && !lines.is_empty()
                && on_event
                    .send(LogcatEvent {
                        lines,
                        finished: false,
                        error: None,
                    })
                    .is_err()
            {
                connected = false;
                worker_cancel.store(true, Ordering::Relaxed);
            }
            if exit_status.is_none() {
                match child.try_wait() {
                    Ok(status) => exit_status = status,
                    Err(error) => {
                        failure = Some(format!("读取 logcat 状态失败：{error}"));
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                }
            }
            if disconnected && exit_status.is_some() {
                break;
            }
        }
        let error = failure.or_else(|| {
            exit_status
                .filter(|status| !cancelled && !status.success())
                .map(|status| format!("logcat 已退出（{}）：{}", status, detail))
        });
        let _ = on_event.send(LogcatEvent {
            lines: vec![],
            finished: true,
            error,
        });
    });
    *current = Some(Session {
        id: session_id,
        committed_bytes,
        cancel,
        worker,
    });
    Ok(save_path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_copies_all_committed_lines_without_later_appends() {
        let directory = std::env::temp_dir().join(format!(
            "mobiletoolbox-logcat-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let source = directory.join("session.log");
        let destination = directory.join("export.log");
        let mut writer = BufWriter::new(File::create(&source).unwrap());
        let lines: Vec<String> = (0..6001).map(|index| format!("日志 {index}")).collect();
        write_lines(&mut writer, &lines).unwrap();
        let committed = std::fs::metadata(&source).unwrap().len();
        write_lines(&mut writer, &["later line".into()]).unwrap();
        assert_eq!(
            copy_snapshot(&source, &destination, committed).unwrap(),
            committed
        );
        let exported = std::fs::read_to_string(&destination).unwrap();
        assert_eq!(exported.lines().count(), 6001);
        assert!(exported.ends_with("日志 6000\n"));
        assert!(check_export_path(&directory, &source).is_err());
        assert!(check_export_path(&directory, &directory.join("new.log")).is_err());
        drop(writer);
        std::fs::remove_file(source).unwrap();
        std::fs::remove_file(destination).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn file_output_preserves_more_than_preview_limit_and_appends_batches() {
        let mut output = Vec::new();
        let lines: Vec<String> = (0..6001).map(|index| format!("日志 {index}")).collect();
        write_lines(&mut output, &lines[..3000]).unwrap();
        write_lines(&mut output, &lines[3000..]).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert_eq!(text.lines().count(), 6001);
        assert!(text.starts_with("日志 0\n"));
        assert!(text.ends_with("日志 6000\n"));
    }

    #[test]
    fn file_write_failures_are_reported() {
        struct Failing;
        impl Write for Failing {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert!(write_lines(&mut Failing, &["line".into()]).is_err());
    }
}

#[tauri::command]
pub fn stop_logcat(logcat: State<'_, LogcatState>, session_id: String) {
    let mut current = logcat.0.lock().unwrap();
    if current
        .as_ref()
        .is_some_and(|session| session.id == session_id)
    {
        if let Some(session) = current.take() {
            session.stop();
        }
    }
}
