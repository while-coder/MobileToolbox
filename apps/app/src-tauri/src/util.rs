use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::model::{FileKind, LogEvent, ProgressEvent, ProgressState};

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis() as u64)
        .unwrap_or(0)
}

/// 统一构造命令：Windows 下隐藏控制台窗口。
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[derive(Serialize)]
pub struct Captured {
    pub stdout: String,
    pub stderr: String,
}

impl Captured {
    pub fn combined(&self) -> String {
        match (self.stdout.trim().is_empty(), self.stderr.trim().is_empty()) {
            (false, false) => format!("{}\n{}", self.stdout.trim(), self.stderr.trim()),
            (false, true) => self.stdout.trim().to_string(),
            (true, false) => self.stderr.trim().to_string(),
            (true, true) => String::new(),
        }
    }
}

/// 安静地运行命令，仅返回输出；用于环境探测等不需要刷日志的场景。
pub fn run_quiet(source: &str, cmd: &mut Command) -> Result<Captured, String> {
    let output = cmd
        .output()
        .map_err(|error| format!("无法启动 {}：{}", source, error))?;
    let captured = Captured {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    };
    if output.status.success() {
        Ok(captured)
    } else {
        let detail = captured.combined();
        let detail = if detail.is_empty() {
            "没有输出".to_string()
        } else {
            tail(&detail, 12)
        };
        Err(format!(
            "{} 退出码 {}：{}",
            source,
            output.status.code().unwrap_or(-1),
            detail
        ))
    }
}

/// 运行长任务并逐行推送到日志面板，分别返回 stdout 和 stderr。
pub fn run_streaming(app: &AppHandle, source: &str, cmd: &mut Command) -> Result<Captured, String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|error| format!("无法启动 {}：{}", source, error))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("无法读取 {} 的输出", source))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("无法读取 {} 的错误输出", source))?;

    let out_app = app.clone();
    let out_source = source.to_string();
    let out_thread = thread::spawn(move || collect_lines(stdout, out_app, out_source, "stdout"));

    let err_app = app.clone();
    let err_source = source.to_string();
    let err_thread = thread::spawn(move || collect_lines(stderr, err_app, err_source, "stderr"));

    let status = child.wait();
    let stdout = out_thread
        .join()
        .map_err(|_| "stdout 读取任务失败".to_string())??;
    let stderr = err_thread
        .join()
        .map_err(|_| "stderr 读取任务失败".to_string())??;

    match status {
        Ok(status) if status.success() => Ok(Captured { stdout, stderr }),
        Ok(status) => {
            let detail = if stderr.trim().is_empty() {
                tail(&stdout, 12)
            } else {
                tail(&stderr, 12)
            };
            Err(format!(
                "{} 退出码 {}：{}",
                source,
                status.code().unwrap_or(-1),
                if detail.is_empty() {
                    "没有输出".into()
                } else {
                    detail
                }
            ))
        }
        Err(error) => Err(format!("{} 执行失败：{}", source, error)),
    }
}

fn collect_lines<R: Read>(
    reader: R,
    app: AppHandle,
    source: String,
    stream: &'static str,
) -> Result<String, String> {
    let mut reader = BufReader::new(reader);
    let mut output = String::new();
    let mut buffer = String::new();
    loop {
        buffer.clear();
        let count = reader
            .read_line(&mut buffer)
            .map_err(|error| format!("读取 {source} 输出失败：{error}"))?;
        if count == 0 {
            break;
        }
        output.push_str(&buffer);
        let line = buffer.trim_end_matches(['\r', '\n']);
        if !line.trim().is_empty() {
            emit_log(&app, &source, stream, line);
        }
    }
    Ok(output)
}

fn tail(text: &str, lines: usize) -> String {
    let collected: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let start = collected.len().saturating_sub(lines);
    collected[start..].join("\n")
}

pub fn emit_log(app: &AppHandle, source: &str, stream: &str, text: &str) {
    let _ = app.emit(
        "mobiletoolbox://log",
        LogEvent {
            time: now_ms(),
            source: source.to_string(),
            stream: stream.to_string(),
            text: text.to_string(),
        },
    );
}

#[allow(clippy::too_many_arguments)]
pub fn emit_progress(
    app: &AppHandle,
    id: Option<&str>,
    file: Option<&str>,
    stage: &str,
    message: &str,
    state: ProgressState,
    ratio: Option<f64>,
) {
    let _ = app.emit(
        "mobiletoolbox://progress",
        ProgressEvent {
            id: id.map(str::to_string),
            file: file.map(str::to_string),
            stage: stage.to_string(),
            message: message.to_string(),
            state,
            ratio,
        },
    );
}

pub struct FileHashes {
    pub size: u64,
    pub sha256: String,
}

/// 计算 SHA-256；去重与目录标识都以它为准，因此不再额外计算 MD5 / SHA-1。
/// 每处理约 8 MB 回调一次进度，调用方可以据此推送事件而不刷屏。
pub fn hash_file<F: FnMut(u64, u64)>(
    path: &Path,
    mut on_progress: F,
) -> Result<FileHashes, String> {
    let total = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    let mut file = File::open(path).map_err(|error| format!("无法读取文件：{}", error))?;
    let mut sha256 = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut size = 0u64;
    let mut reported = 0u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("读取文件失败：{}", error))?;
        if read == 0 {
            break;
        }
        size += read as u64;
        sha256.update(&buffer[..read]);
        if size - reported >= 8 * 1024 * 1024 || size == total {
            on_progress(size, total);
            reported = size;
        }
    }
    Ok(FileHashes {
        size,
        sha256: hex::encode(sha256.finalize()),
    })
}

/// 复制文件并按 8 MB 粒度汇报进度，避免大文件复制期间界面完全静止。
pub fn copy_file<F: FnMut(u64, u64)>(
    source: &Path,
    target: &Path,
    mut on_progress: F,
) -> Result<u64, String> {
    let total = fs::metadata(source).map(|meta| meta.len()).unwrap_or(0);
    let mut input = File::open(source).map_err(|error| format!("无法读取文件：{}", error))?;
    let mut output = File::create(target).map_err(|error| format!("无法写入文件：{}", error))?;
    let mut buffer = vec![0u8; 1024 * 1024];
    let mut copied = 0u64;
    let mut reported = 0u64;
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| format!("读取文件失败：{}", error))?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .map_err(|error| format!("写入文件失败：{}", error))?;
        copied += read as u64;
        if copied - reported >= 8 * 1024 * 1024 || copied == total {
            on_progress(copied, total);
            reported = copied;
        }
    }
    output
        .flush()
        .map_err(|error| format!("写入文件失败：{}", error))?;
    Ok(copied)
}

pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{} B", bytes);
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if value >= 100.0 {
        format!("{:.0} {}", value, UNITS[unit])
    } else {
        format!("{:.1} {}", value, UNITS[unit])
    }
}

pub fn sanitize_stem(name: &str) -> String {
    let mut stem: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    while stem.len() > 48 {
        stem.pop();
    }
    let stem = stem.trim_matches(['.', '_', '-']).to_string();
    if stem.is_empty() {
        "package".to_string()
    } else {
        stem
    }
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败：{}", path.display(), error))?;
    serde_json::from_str(&raw).map_err(|error| format!("解析 {} 失败：{}", path.display(), error))
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let raw = serde_json::to_vec_pretty(value).map_err(|error| format!("序列化失败：{}", error))?;
    let temp = path.with_file_name(format!(
        "{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("meta.json")
    ));
    {
        let mut file = File::create(&temp)
            .map_err(|error| format!("写入 {} 失败：{}", temp.display(), error))?;
        file.write_all(&raw)
            .map_err(|error| format!("写入 {} 失败：{}", temp.display(), error))?;
        file.sync_all().ok();
    }
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("覆盖 {} 失败：{}", path.display(), error))?;
    }
    fs::rename(&temp, path).map_err(|error| format!("保存 {} 失败：{}", path.display(), error))
}

pub fn data_url(mime: &str, bytes: &[u8]) -> String {
    format!(
        "data:{};base64,{}",
        mime,
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
}

pub fn mime_for_extension(extension: &str) -> &'static str {
    match extension.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}

const TEXT_EXTENSIONS: &[&str] = &[
    "txt",
    "md",
    "markdown",
    "log",
    "csv",
    "tsv",
    "ini",
    "cfg",
    "conf",
    "properties",
    "xml",
    "json",
    "json5",
    "yml",
    "yaml",
    "toml",
    "gradle",
    "pro",
    "map",
    "java",
    "kt",
    "kts",
    "smali",
    "js",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "jsx",
    "vue",
    "html",
    "htm",
    "css",
    "scss",
    "less",
    "sql",
    "sh",
    "bat",
    "cmd",
    "ps1",
    "c",
    "h",
    "cpp",
    "hpp",
    "cs",
    "go",
    "rs",
    "py",
    "rb",
    "php",
    "lua",
    "dart",
    "gitignore",
    "gitattributes",
    "nomedia",
    "version",
];

const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "avif", "svg",
];

pub fn file_kind(extension: &str) -> FileKind {
    let extension = extension.to_ascii_lowercase();
    if IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        FileKind::Image
    } else if TEXT_EXTENSIONS.contains(&extension.as_str()) {
        FileKind::Text
    } else {
        FileKind::Binary
    }
}

pub fn open_directory(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("目录不存在：{}", path.display()));
    }
    #[cfg(windows)]
    {
        command("explorer")
            .arg(path)
            .spawn()
            .map_err(|error| format!("打开资源管理器失败：{}", error))?;
    }
    #[cfg(target_os = "macos")]
    {
        command("open")
            .arg(path)
            .spawn()
            .map_err(|error| format!("打开 Finder 失败：{}", error))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        command("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|error| format!("打开文件管理器失败：{}", error))?;
    }
    Ok(())
}

pub fn reveal_path(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("文件不存在：{}", path.display()));
    }
    #[cfg(windows)]
    {
        command("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
            .map_err(|error| format!("定位文件失败：{}", error))?;
    }
    #[cfg(not(windows))]
    {
        let parent = path.parent().unwrap_or(path);
        open_directory(parent)?;
    }
    Ok(())
}
