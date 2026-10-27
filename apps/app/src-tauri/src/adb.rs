use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use tauri::AppHandle;

use crate::model::AndroidDevice;
use crate::store::AppState;
use crate::tools;
use crate::util;

fn valid_device_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':'))
}

fn valid_address(address: &str) -> bool {
    !address.is_empty()
        && address.len() <= 128
        && address.contains(':')
        && !address.contains("..")
        && address
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | ':' | '-' | '_' | '[' | ']'))
}

pub fn valid_package_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 255 {
        return false;
    }
    let parts: Vec<&str> = name.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        })
}

fn base_command(adb: &Path, device_id: &str) -> Command {
    let mut cmd = util::command(adb);
    cmd.arg("-s").arg(device_id);
    cmd
}

fn getprop(adb: &Path, device_id: &str) -> HashMap<String, String> {
    let mut cmd = base_command(adb, device_id);
    cmd.arg("shell").arg("getprop");
    let mut props = HashMap::new();
    let Ok(captured) = util::run_quiet("adb getprop", &mut cmd) else {
        return props;
    };
    for line in captured.stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('[') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once("]: [") {
            let key = key.trim_start_matches('[').trim().to_string();
            let value = value.trim_end_matches(']').trim().to_string();
            if !key.is_empty() {
                props.insert(key, value);
            }
        }
    }
    props
}

pub fn devices(state: &AppState) -> Result<Vec<AndroidDevice>, String> {
    let adb = tools::require_adb(state)?;
    let captured = util::run_quiet("adb devices", util::command(&adb).arg("devices").arg("-l"))?;

    let mut result = Vec::new();
    for (index, line) in captured.stdout.lines().enumerate() {
        let trimmed = line.trim();
        if index == 0 || trimmed.is_empty() || trimmed.starts_with('*') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(id) = parts.next() else {
            continue;
        };
        let Some(state_text) = parts.next() else {
            continue;
        };
        if !valid_device_id(id) {
            continue;
        }

        let mut extra: HashMap<String, String> = HashMap::new();
        for token in parts {
            if let Some((key, value)) = token.split_once(':') {
                extra.insert(key.to_string(), value.to_string());
            }
        }

        let mut device = AndroidDevice {
            id: id.to_string(),
            state: state_text.to_string(),
            model: extra.get("model").cloned(),
            manufacturer: extra.get("manufacturer").cloned(),
            android_release: None,
            sdk_int: None,
            product: extra.get("product").cloned(),
        };

        if state_text == "device" {
            let props = getprop(&adb, id);
            device.model = props.get("ro.product.model").cloned().or(device.model);
            device.manufacturer = props
                .get("ro.product.manufacturer")
                .cloned()
                .or_else(|| props.get("ro.product.brand").cloned())
                .or(device.manufacturer);
            device.android_release = props.get("ro.build.version.release").cloned();
            device.sdk_int = props.get("ro.build.version.sdk").cloned();
            device.product = props.get("ro.product.name").cloned().or(device.product);
        }

        result.push(device);
    }
    Ok(result)
}

pub fn connect(state: &AppState, address: &str) -> Result<String, String> {
    if !valid_address(address) {
        return Err("连接地址格式不正确，应为 host:port，例如 127.0.0.1:7555".into());
    }
    let adb = tools::require_adb(state)?;
    let captured = util::run_quiet(
        "adb connect",
        util::command(&adb).arg("connect").arg(address),
    )?;
    let text = captured.combined();
    let lower = text.to_lowercase();
    if lower.contains("failed") || lower.contains("cannot") || lower.contains("unable") {
        return Err(if text.trim().is_empty() {
            "adb connect 失败".into()
        } else {
            text.trim().to_string()
        });
    }
    Ok(text)
}

pub fn disconnect(state: &AppState, address: &str) -> Result<String, String> {
    if !valid_address(address) {
        return Err("连接地址格式不正确".into());
    }
    let adb = tools::require_adb(state)?;
    let captured = util::run_quiet(
        "adb disconnect",
        util::command(&adb).arg("disconnect").arg(address),
    )?;
    Ok(captured.combined())
}

pub fn install(
    app: &AppHandle,
    state: &AppState,
    device_id: &str,
    apk: &Path,
) -> Result<String, String> {
    if !valid_device_id(device_id) {
        return Err("非法的设备标识".into());
    }
    if !apk.is_file() {
        return Err(format!("APK 不存在：{}", apk.display()));
    }
    let adb = tools::require_adb(state)?;
    let mut cmd = base_command(&adb, device_id);
    cmd.arg("install").arg("-r").arg(apk);
    let output = util::run_streaming(app, "adb install", &mut cmd)?;
    let lower = output.to_lowercase();
    if lower.contains("failure") || lower.contains("error") {
        return Err(output.trim().to_string());
    }
    if !lower.contains("success") {
        return Err(format!("安装结果未确认成功：{}", output.trim()));
    }
    Ok(output.trim().to_string())
}

pub fn uninstall(
    app: &AppHandle,
    state: &AppState,
    device_id: &str,
    package: &str,
) -> Result<String, String> {
    if !valid_device_id(device_id) {
        return Err("非法的设备标识".into());
    }
    if !valid_package_name(package) {
        return Err("包名格式不正确".into());
    }
    let adb = tools::require_adb(state)?;
    let mut cmd = base_command(&adb, device_id);
    cmd.arg("uninstall").arg(package);
    let output = util::run_streaming(app, "adb uninstall", &mut cmd)?;
    let lower = output.to_lowercase();
    if lower.contains("failure") || lower.contains("error") {
        return Err(output.trim().to_string());
    }
    if !lower.contains("success") {
        return Err(format!("卸载结果未确认成功：{}", output.trim()));
    }
    Ok(output.trim().to_string())
}

fn obb_package(file_name: &str) -> Option<String> {
    let parts: Vec<&str> = file_name.split('.').collect();
    if parts.len() < 4 || !matches!(parts[0], "main" | "patch") {
        return None;
    }
    if !parts[1].chars().all(|ch| ch.is_ascii_digit()) || parts.last() != Some(&"obb") {
        return None;
    }
    let package = parts[2..parts.len() - 1].join(".");
    valid_package_name(&package).then_some(package)
}

pub fn push_obb(
    app: &AppHandle,
    state: &AppState,
    device_id: &str,
    file: &Path,
) -> Result<String, String> {
    if !valid_device_id(device_id) {
        return Err("非法的设备标识".into());
    }
    let file_name = file
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "无法读取 OBB 文件名".to_string())?
        .to_string();
    if !file_name.to_lowercase().ends_with(".obb") {
        return Err("只支持 .obb 文件".into());
    }
    if !file.is_file() {
        return Err(format!("OBB 文件不存在：{}", file.display()));
    }
    let package = obb_package(&file_name).ok_or_else(|| {
        "OBB 文件名应为 main.<版本号>.<包名>.obb 或 patch.<版本号>.<包名>.obb".to_string()
    })?;

    let adb = tools::require_adb(state)?;
    let remote_dir = format!("/sdcard/Android/obb/{}", package);
    let mut mkdir = base_command(&adb, device_id);
    mkdir.arg("shell").arg("mkdir").arg("-p").arg(&remote_dir);
    util::run_quiet("adb shell mkdir", &mut mkdir)?;

    let remote = format!("{}/{}", remote_dir, file_name);
    let mut push = base_command(&adb, device_id);
    push.arg("push").arg(file).arg(&remote);
    let output = util::run_streaming(app, "adb push", &mut push)?;
    if !output.to_lowercase().contains("pushed") && !output.to_lowercase().contains("skipped") {
        return Err(format!("推送结果未确认成功：{}", output.trim()));
    }
    Ok(format!("{} → {}", file_name, remote))
}
