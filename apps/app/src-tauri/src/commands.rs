use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};

use crate::apk;
use crate::model::{
    DirectoryListing, EnvironmentInfo, FileEntry, FileKind, ImportOutcome, PackageImportError,
    PackageInfo, TextPreview,
};
use crate::store::{self, AppState};
use crate::tools;
use crate::util;

const MAX_DIRECTORY_ENTRIES: usize = 2000;
const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePaths {
    work_dir: String,
    tools_dir: String,
}

#[tauri::command]
pub fn runtime_paths(state: State<'_, AppState>) -> RuntimePaths {
    RuntimePaths {
        work_dir: state.work_dir.display().to_string(),
        tools_dir: state.tools_dir.display().to_string(),
    }
}

async fn execute_tool(
    app: AppHandle,
    program: PathBuf,
    args: Vec<String>,
    stream: Option<bool>,
    source: String,
    cwd: Option<String>,
) -> Result<util::Captured, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut command = util::command(program);
        command.args(args);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        if stream.unwrap_or(false) {
            util::run_streaming(&app, &source, &mut command)
        } else {
            util::run_quiet(&source, &mut command)
        }
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub async fn adb_command(
    app: AppHandle,
    state: State<'_, AppState>,
    args: Vec<String>,
    stream: Option<bool>,
    source: Option<String>,
    cwd: Option<String>,
) -> Result<util::Captured, String> {
    execute_tool(
        app,
        tools::require_adb(&state)?,
        args,
        stream,
        source.unwrap_or_else(|| "adb".into()),
        cwd,
    )
    .await
}

#[tauri::command]
pub async fn java_command(
    app: AppHandle,
    args: Vec<String>,
    stream: Option<bool>,
    source: Option<String>,
    cwd: Option<String>,
) -> Result<util::Captured, String> {
    execute_tool(
        app,
        tools::require_java()?,
        args,
        stream,
        source.unwrap_or_else(|| "java".into()),
        cwd,
    )
    .await
}

#[tauri::command]
pub fn get_package_info(state: State<'_, AppState>, id: String) -> Result<PackageInfo, String> {
    store::load_info(&store::package_dir(&state, &id)?)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackageFileAction {
    IsFile,
    HasEntries,
    Remove,
    Rename,
}

// Filesystem primitives are confined to a package; workflows and tool arguments live in TS.
#[tauri::command]
pub async fn package_file(
    state: State<'_, AppState>,
    id: String,
    action: PackageFileAction,
    path: String,
    destination: Option<String>,
) -> Result<bool, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        package_file_impl(&state, &id, action, &path, destination.as_deref())
    })
    .await
    .map_err(join_error)?
}

fn package_file_impl(
    state: &AppState,
    id: &str,
    action: PackageFileAction,
    path: &str,
    destination: Option<&str>,
) -> Result<bool, String> {
    let dir = store::package_dir(state, id)?;
    store::validate_relative_path(path)?;
    let target = dir.join(path.replace('\\', "/"));
    if !target.exists() {
        return Ok(false);
    }
    let target = store::safe_join(&dir, &path)?;
    match action {
        PackageFileAction::IsFile => Ok(target.is_file()),
        PackageFileAction::HasEntries => Ok(store::has_entries(&target)),
        PackageFileAction::Remove => {
            if target.is_dir() {
                fs::remove_dir_all(target)
            } else {
                fs::remove_file(target)
            }
            .map_err(|error| format!("删除文件失败：{error}"))?;
            Ok(true)
        }
        PackageFileAction::Rename => {
            let destination = destination.ok_or("缺少目标路径")?;
            store::validate_relative_path(&destination)?;
            let relative = PathBuf::from(destination.replace('\\', "/"));
            let parent = relative.parent().and_then(Path::to_str).unwrap_or("");
            let parent = store::safe_join(&dir, parent)?;
            let name = relative.file_name().ok_or("目标路径不能为空")?;
            let destination = parent.join(name);
            if destination.exists() {
                return Err("目标路径已存在".into());
            }
            fs::rename(target, destination).map_err(|error| format!("移动文件失败：{error}"))?;
            Ok(true)
        }
    }
}
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::{package_file_impl, PackageFileAction};
    use crate::store::AppState;
    use std::fs;

    #[test]
    fn package_files_support_staging_and_reject_path_escape() {
        let root = std::env::temp_dir().join(format!(
            "mobiletoolbox-files-{}-{}",
            std::process::id(),
            crate::util::now_ms()
        ));
        let dir = root.join("sample");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("source.tmp.jar"), b"new jar").unwrap();
        let state = AppState {
            work_dir: root.clone(),
            tools_dir: root.clone(),
        };
        assert!(package_file_impl(
            &state,
            "sample",
            PackageFileAction::IsFile,
            "source.tmp.jar",
            None
        )
        .unwrap());
        assert!(!package_file_impl(
            &state,
            "sample",
            PackageFileAction::IsFile,
            "missing/file",
            None
        )
        .unwrap());
        assert!(package_file_impl(
            &state,
            "sample",
            PackageFileAction::Remove,
            "../outside",
            None
        )
        .is_err());
        assert!(package_file_impl(
            &state,
            "sample",
            PackageFileAction::Rename,
            "source.tmp.jar",
            Some("../outside")
        )
        .is_err());
        assert!(package_file_impl(
            &state,
            "sample",
            PackageFileAction::Rename,
            "source.tmp.jar",
            Some("source.jar")
        )
        .unwrap());
        assert_eq!(fs::read(dir.join("source.jar")).unwrap(), b"new jar");
        assert!(package_file_impl(
            &state,
            "sample",
            PackageFileAction::Remove,
            "source.jar",
            None
        )
        .unwrap());
        assert!(!dir.join("source.jar").exists());
        fs::remove_dir_all(root).unwrap();
    }
}

fn join_error(error: impl std::fmt::Display) -> String {
    format!("后台任务失败：{}", error)
}

#[tauri::command]
pub async fn get_environment(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<EnvironmentInfo, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || tools::build_environment(&app, &state))
        .await
        .map_err(join_error)
}

#[tauri::command]
pub fn list_packages(state: State<'_, AppState>) -> Vec<PackageInfo> {
    store::list_infos(&state)
}

#[tauri::command]
pub async fn import_packages(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<ImportOutcome, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut outcome = ImportOutcome::default();
        for path in paths.into_iter().take(200) {
            match apk::import_one(&app, &state, &path) {
                Ok(info) => outcome.packages.push(info),
                Err(message) => outcome.errors.push(PackageImportError {
                    file: Path::new(&path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or(&path)
                        .to_string(),
                    message,
                }),
            }
        }
        outcome
    })
    .await
    .map_err(join_error)
}

#[tauri::command]
pub async fn remove_package(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let dir = store::package_dir(&state, &id)?;
        if !dir.exists() {
            return Err("安装包不存在".into());
        }
        fs::remove_dir_all(&dir).map_err(|error| format!("删除失败：{}", error))?;
        util::emit_log(&app, "安装包", "info", &format!("已删除 {}", id));
        Ok(())
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub async fn clear_packages(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let entries = fs::read_dir(&state.work_dir)
            .map_err(|error| format!("读取工作目录失败：{}", error))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(&path)
                    .map_err(|error| format!("删除 {} 失败：{}", path.display(), error))?;
            } else if path.is_file() {
                fs::remove_file(&path)
                    .map_err(|error| format!("删除 {} 失败：{}", path.display(), error))?;
            }
        }
        util::emit_log(&app, "安装包", "info", "安装包库已清空");
        Ok(())
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub fn get_package_icon(state: State<'_, AppState>, id: String) -> Result<Option<String>, String> {
    let dir = store::package_dir(&state, &id)?;
    let info = store::load_info(&dir)?;
    let Some(icon) = store::icon_path(&dir, &info) else {
        return Ok(None);
    };
    let bytes = fs::read(&icon).map_err(|error| format!("读取图标失败：{}", error))?;
    let extension = icon
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("png");
    Ok(Some(util::data_url(
        util::mime_for_extension(extension),
        &bytes,
    )))
}

#[tauri::command]
pub fn list_package_dir(
    state: State<'_, AppState>,
    id: String,
    root: String,
    relative_path: String,
) -> Result<DirectoryListing, String> {
    let dir = store::package_dir(&state, &id)?;
    let root_dir = store::root_dir(&dir, &root)?;
    if !root_dir.is_dir() {
        return Err(if root == "java" {
            "尚未生成 Java 源码".into()
        } else {
            "尚未解包资源".into()
        });
    }
    let target = store::safe_join(&root_dir, &relative_path)?;
    if !target.is_dir() {
        return Err("路径不是目录".into());
    }

    let parts: Vec<String> = relative_path
        .replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .map(str::to_string)
        .collect();
    let base = parts.join("/");

    let entries = fs::read_dir(&target).map_err(|error| format!("读取目录失败：{}", error))?;
    let mut result = Vec::new();
    let mut truncated = false;
    for entry in entries.flatten() {
        if result.len() >= MAX_DIRECTORY_ENTRIES {
            truncated = true;
            break;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        let directory = file_type.is_dir();
        let size = if directory {
            0
        } else {
            entry.metadata().map(|meta| meta.len()).unwrap_or(0)
        };
        let extension = if directory {
            String::new()
        } else {
            Path::new(&name)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
        };
        let relative = if base.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", base, name)
        };
        result.push(FileEntry {
            name,
            relative_path: relative,
            directory,
            size,
            kind: if directory {
                FileKind::Binary
            } else {
                util::file_kind(&extension)
            },
            extension,
        });
    }

    result.sort_by(|left, right| {
        right
            .directory
            .cmp(&left.directory)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    Ok(DirectoryListing {
        entries: result,
        truncated,
    })
}

#[tauri::command]
pub fn read_package_text(
    state: State<'_, AppState>,
    id: String,
    root: String,
    relative_path: String,
) -> Result<TextPreview, String> {
    let dir = store::package_dir(&state, &id)?;
    let root_dir = store::root_dir(&dir, &root)?;
    let path = store::safe_join(&root_dir, &relative_path)?;
    if !path.is_file() {
        return Err("路径不是文件".into());
    }
    let size = path
        .metadata()
        .map_err(|error| format!("读取文件信息失败：{}", error))?
        .len();
    let mut file = File::open(&path).map_err(|error| format!("打开文件失败：{}", error))?;
    let mut bytes = Vec::with_capacity(size.min(MAX_TEXT_BYTES) as usize);
    file.by_ref()
        .take(MAX_TEXT_BYTES)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("读取文件失败：{}", error))?;
    Ok(TextPreview {
        content: String::from_utf8_lossy(&bytes).to_string(),
        truncated: size > MAX_TEXT_BYTES,
        size,
    })
}

#[tauri::command]
pub fn read_package_image(
    state: State<'_, AppState>,
    id: String,
    root: String,
    relative_path: String,
) -> Result<String, String> {
    let dir = store::package_dir(&state, &id)?;
    let root_dir = store::root_dir(&dir, &root)?;
    let path = store::safe_join(&root_dir, &relative_path)?;
    if !path.is_file() {
        return Err("路径不是文件".into());
    }
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = util::mime_for_extension(&extension);
    if mime == "application/octet-stream" {
        return Err("不支持的图片格式".into());
    }
    let size = path
        .metadata()
        .map_err(|error| format!("读取文件信息失败：{}", error))?
        .len();
    if size > MAX_IMAGE_BYTES {
        return Err(format!("图片过大（{} 字节），请直接在解包目录中查看", size));
    }
    let bytes = fs::read(&path).map_err(|error| format!("读取图片失败：{}", error))?;
    Ok(util::data_url(mime, &bytes))
}

#[tauri::command]
pub fn open_package_folder(
    state: State<'_, AppState>,
    id: String,
    kind: String,
) -> Result<(), String> {
    let dir = store::package_dir(&state, &id)?;
    let target = match kind.as_str() {
        "root" => dir,
        "source" => store::decoded_dir(&dir),
        "java" => store::java_dir(&dir),
        _ => return Err("未知的目录类型".into()),
    };
    if !target.exists() {
        return Err(match kind.as_str() {
            "java" => "尚未生成 Java 源码".into(),
            "source" => "尚未解包资源".into(),
            _ => "安装包目录不存在".into(),
        });
    }
    util::open_directory(&target)
}

#[tauri::command]
pub fn reveal_package(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let dir = store::package_dir(&state, &id)?;
    util::reveal_path(&store::apk_path(&dir))
}

#[tauri::command]
pub fn open_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let root = state
        .work_dir
        .canonicalize()
        .map_err(|error| format!("工作目录不可用：{}", error))?;
    let target = PathBuf::from(&path)
        .canonicalize()
        .map_err(|error| format!("路径不存在：{}", error))?;
    if !target.starts_with(&root) {
        return Err("只能打开工作目录内的路径".into());
    }
    if target.is_dir() {
        util::open_directory(&target)
    } else {
        util::reveal_path(&target)
    }
}
