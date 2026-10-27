use std::fs;
use std::path::{Path, PathBuf};

use crate::model::PackageInfo;
use crate::util;

/// 应用的持久状态。工作目录里一个子目录对应一个已导入的 APK。
#[derive(Clone, Debug)]
pub struct AppState {
    pub work_dir: PathBuf,
    pub tools_dir: PathBuf,
}

pub fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 128 {
        return Err("非法的安装包标识".into());
    }
    if id.starts_with('.') || id.contains("..") {
        return Err("非法的安装包标识".into());
    }
    if !id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err("非法的安装包标识".into());
    }
    Ok(())
}

pub fn package_dir(state: &AppState, id: &str) -> Result<PathBuf, String> {
    validate_id(id)?;
    Ok(state.work_dir.join(id))
}

pub fn apk_path(dir: &Path) -> PathBuf {
    dir.join("original.apk")
}

pub fn decoded_dir(dir: &Path) -> PathBuf {
    dir.join("decoded")
}

pub fn java_dir(dir: &Path) -> PathBuf {
    dir.join("java")
}

pub fn jar_path(dir: &Path) -> PathBuf {
    dir.join("source.jar")
}

pub fn icon_path(dir: &Path, info: &PackageInfo) -> Option<PathBuf> {
    let extension = info.icon_ext.as_deref()?;
    if extension.is_empty() {
        return None;
    }
    let path = dir.join(format!("icon.{}", extension));
    path.is_file().then_some(path)
}

pub fn refresh_flags(info: &mut PackageInfo, dir: &Path) {
    info.decoded = decoded_dir(dir).join("AndroidManifest.xml").is_file();
    info.has_jar = jar_path(dir).is_file();
    info.decompiled = has_entries(&java_dir(dir));
    if info.has_icon && icon_path(dir, info).is_none() {
        info.has_icon = false;
        info.icon_ext = None;
    }
}

pub fn load_info(dir: &Path) -> Result<PackageInfo, String> {
    let mut info: PackageInfo = util::read_json(&dir.join("meta.json"))?;
    refresh_flags(&mut info, dir);
    Ok(info)
}

pub fn save_info(dir: &Path, info: &PackageInfo) -> Result<(), String> {
    util::write_json(&dir.join("meta.json"), info)
}

pub fn list_infos(state: &AppState) -> Vec<PackageInfo> {
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir(&state.work_dir) else {
        return result;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || !path.join("meta.json").is_file() {
            continue;
        }
        if let Ok(info) = load_info(&path) {
            result.push(info);
        }
    }
    result.sort_by(|left, right| right.imported_at.cmp(&left.imported_at));
    result
}

pub fn has_entries(path: &Path) -> bool {
    fs::read_dir(path)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

pub fn root_dir(dir: &Path, root: &str) -> Result<PathBuf, String> {
    match root {
        "source" => Ok(decoded_dir(dir)),
        "java" => Ok(java_dir(dir)),
        _ => Err("未知的浏览根目录".into()),
    }
}

/// 把前端传来的相对路径限制在浏览根目录内，并使用 canonicalize 阻断符号链接越界。
pub fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if !root.exists() {
        return Err(format!("目录不存在：{}", root.display()));
    }
    if relative.len() > 4096 {
        return Err("路径过长".into());
    }
    let root_canonical = root
        .canonicalize()
        .map_err(|error| format!("无法解析目录：{}", error))?;

    let normalized = relative.replace('\\', "/");
    if normalized.starts_with('/') || normalized.starts_with("//") {
        return Err("不允许使用绝对路径".into());
    }

    let mut target = root_canonical.clone();
    for part in normalized.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err("不允许访问上级目录".into());
        }
        if part.contains(':') || part.contains('\0') {
            return Err("非法路径".into());
        }
        target.push(part);
    }

    let canonical = target
        .canonicalize()
        .map_err(|_| format!("路径不存在：{}", relative))?;
    if !canonical.starts_with(&root_canonical) {
        return Err("路径越界".into());
    }
    Ok(canonical)
}
