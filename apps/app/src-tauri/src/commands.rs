use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};

use crate::adb;
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
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

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
pub async fn decode_package(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<PackageInfo, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || apk::decode(&app, &state, &id))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn build_jar(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<PackageInfo, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || apk::build_jar(&app, &state, &id))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn decompile_package(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<PackageInfo, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || apk::decompile(&app, &state, &id))
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

#[tauri::command]
pub async fn adb_devices(
    state: State<'_, AppState>,
) -> Result<Vec<crate::model::AndroidDevice>, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || adb::devices(&state))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn adb_connect(state: State<'_, AppState>, address: String) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || adb::connect(&state, &address))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn adb_disconnect(state: State<'_, AppState>, address: String) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || adb::disconnect(&state, &address))
        .await
        .map_err(join_error)?
}

#[tauri::command]
pub async fn adb_install(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
    apk_path: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        adb::install(&app, &state, &device_id, &PathBuf::from(apk_path))
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub async fn adb_install_package(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
    id: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let dir = store::package_dir(&state, &id)?;
        adb::install(&app, &state, &device_id, &store::apk_path(&dir))
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub async fn adb_push_obb(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
    file_path: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        adb::push_obb(&app, &state, &device_id, &PathBuf::from(file_path))
    })
    .await
    .map_err(join_error)?
}

#[tauri::command]
pub async fn adb_uninstall(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
    package_name: String,
) -> Result<String, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        adb::uninstall(&app, &state, &device_id, &package_name)
    })
    .await
    .map_err(join_error)?
}
