use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use apk_info::structs::{
    ResTableConfig, ResTableEntry, ResourceValue, ResourceValueType, StringPool,
};
use apk_info::{Apk, ARSC};
use tauri::AppHandle;
use zip::ZipArchive;

use crate::model::{PackageInfo, ProgressState};
use crate::store::{self, AppState};
use crate::util;

const MAX_ICON_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug)]
struct ParsedApk {
    package_name: Option<String>,
    version_name: Option<String>,
    version_code: Option<String>,
    min_sdk: Option<String>,
    target_sdk: Option<String>,
    compile_sdk: Option<String>,
    launchable_activity: Option<String>,
    install_location: Option<String>,
    debuggable: bool,
    permissions: Vec<String>,
    native_code: Vec<String>,
    labels: BTreeMap<String, String>,
    default_label: Option<String>,
    application_label: Option<String>,
    icon_candidates: Vec<(u32, String)>,
    fallback_icon: Option<String>,
}

/// 导入单个 APK：统一收尾错误日志，并把失败同步给前端的任务列表。
pub fn import_one(app: &AppHandle, state: &AppState, source: &str) -> Result<PackageInfo, String> {
    let source_path = PathBuf::from(source);
    let Some(file_name) = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string)
    else {
        return Err("无法读取文件名".into());
    };

    match import_package(app, state, &source_path, file_name.clone()) {
        Ok(info) => Ok(info),
        Err(error) => {
            util::emit_log(
                app,
                "导入",
                "error",
                &format!("{} 导入失败：{}", file_name, error),
            );
            util::emit_progress(
                app,
                None,
                Some(&file_name),
                "import",
                &error,
                ProgressState::Error,
                None,
            );
            Err(error)
        }
    }
}

/// 实际导入流程。进度事件全程使用源文件名作为 key（不传 id），
/// 这样在安装包 id 生成之前界面也能显示进度，并且各阶段的 key 始终一致。
fn import_package(
    app: &AppHandle,
    state: &AppState,
    source_path: &Path,
    file_name: String,
) -> Result<PackageInfo, String> {
    if !source_path.is_file() {
        return Err("文件不存在".into());
    }
    if source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| !ext.eq_ignore_ascii_case("apk"))
        .unwrap_or(true)
    {
        return Err("只支持 .apk 文件".into());
    }

    let size = fs::metadata(source_path)
        .map(|meta| meta.len())
        .unwrap_or(0);
    util::emit_log(
        app,
        "导入",
        "info",
        &format!("开始导入 {}（{}）", file_name, util::format_size(size)),
    );
    util::emit_progress(
        app,
        None,
        Some(&file_name),
        "import",
        "准备导入",
        ProgressState::Running,
        Some(0.0),
    );

    util::emit_progress(
        app,
        None,
        Some(&file_name),
        "hash",
        "正在计算摘要",
        ProgressState::Running,
        Some(0.02),
    );
    util::emit_log(app, "导入", "info", "计算 SHA-256…");
    let hashes = util::hash_file(source_path, |done, total| {
        let ratio = if total > 0 {
            done as f64 / total as f64
        } else {
            0.0
        };
        util::emit_progress(
            app,
            None,
            Some(&file_name),
            "hash",
            &format!(
                "正在计算摘要 {:.0}% · {} / {}",
                ratio * 100.0,
                util::format_size(done),
                util::format_size(total)
            ),
            ProgressState::Running,
            Some(0.02 + ratio * 0.43),
        );
    })?;
    if hashes.size == 0 {
        return Err("文件为空".into());
    }
    util::emit_log(
        app,
        "导入",
        "info",
        &format!(
            "摘要完成 · SHA-256 {}… · {}",
            &hashes.sha256[..10],
            util::format_size(hashes.size)
        ),
    );

    // 以内容为准去重：同一个 APK 换名后再次导入，应复用已有条目。
    if let Some(info) = store::list_infos(state)
        .into_iter()
        .find(|info| info.sha256 == hashes.sha256)
    {
        util::emit_log(
            app,
            "导入",
            "info",
            &format!("{} 已在库中：{}", file_name, info.app_name),
        );
        util::emit_progress(
            app,
            None,
            Some(&file_name),
            "import",
            "安装包已在库中",
            ProgressState::Done,
            Some(1.0),
        );
        return Ok(info);
    }

    let stem = source_path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("package");
    let id = format!("{}-{}", util::sanitize_stem(stem), &hashes.sha256[..10]);
    let dir = store::package_dir(state, &id)?;
    let apk = store::apk_path(&dir);

    if dir.join("meta.json").is_file() && apk.is_file() {
        let info = store::load_info(&dir)?;
        util::emit_log(
            app,
            "导入",
            "info",
            &format!("{} 已在库中：{}", file_name, info.app_name),
        );
        util::emit_progress(
            app,
            None,
            Some(&file_name),
            "import",
            "安装包已在库中",
            ProgressState::Done,
            Some(1.0),
        );
        return Ok(info);
    }

    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|error| format!("清理旧目录失败：{}", error))?;
    }
    fs::create_dir_all(&dir).map_err(|error| format!("创建目录失败：{}", error))?;

    let result = (|| -> Result<PackageInfo, String> {
        util::emit_progress(
            app,
            None,
            Some(&file_name),
            "copy",
            "正在复制 APK",
            ProgressState::Running,
            Some(0.45),
        );
        util::emit_log(
            app,
            "导入",
            "info",
            &format!("复制 APK 到工作目录（{}）", util::format_size(hashes.size)),
        );
        util::copy_file(&source_path, &apk, |done, total| {
            let ratio = if total > 0 {
                done as f64 / total as f64
            } else {
                0.0
            };
            util::emit_progress(
                app,
                None,
                Some(&file_name),
                "copy",
                &format!(
                    "正在复制 APK {:.0}% · {} / {}",
                    ratio * 100.0,
                    util::format_size(done),
                    util::format_size(total)
                ),
                ProgressState::Running,
                Some(0.45 + ratio * 0.2),
            );
        })?;

        util::emit_progress(
            app,
            None,
            Some(&file_name),
            "parse",
            "正在解析 AndroidManifest.xml",
            ProgressState::Running,
            Some(0.66),
        );
        util::emit_log(app, "导入", "info", "解析 AndroidManifest.xml…");
        let parsed = parse_apk(&apk)?;

        util::emit_progress(
            app,
            None,
            Some(&file_name),
            "icon",
            "正在提取应用图标",
            ProgressState::Running,
            Some(0.9),
        );
        let icon_ext = extract_icon(&apk, &dir, &parsed);
        let app_name = parsed
            .default_label
            .clone()
            .or_else(|| parsed.application_label.clone())
            .or_else(|| parsed.package_name.clone())
            .unwrap_or_else(|| {
                source_path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Android 应用")
                    .to_string()
            });

        util::emit_progress(
            app,
            None,
            Some(&file_name),
            "save",
            "正在写入安装包信息",
            ProgressState::Running,
            Some(0.97),
        );
        let info = PackageInfo {
            id: id.clone(),
            file_name: file_name.clone(),
            app_name,
            labels: parsed.labels,
            package_name: parsed.package_name,
            version_name: parsed.version_name,
            version_code: parsed.version_code,
            min_sdk: parsed.min_sdk,
            target_sdk: parsed.target_sdk,
            compile_sdk: parsed.compile_sdk,
            launchable_activity: parsed.launchable_activity,
            install_location: parsed.install_location,
            debuggable: parsed.debuggable,
            permissions: parsed.permissions,
            native_code: parsed.native_code,
            has_icon: icon_ext.is_some(),
            icon_ext,
            size_bytes: hashes.size,
            sha256: hashes.sha256,
            imported_at: util::now_ms(),
            decoded: false,
            has_jar: false,
            decompiled: false,
        };
        store::save_info(&dir, &info)?;
        Ok(info)
    })();

    match result {
        Ok(info) => {
            util::emit_progress(
                app,
                None,
                Some(&file_name),
                "import",
                &format!("已导入 {}", info.app_name),
                ProgressState::Done,
                Some(1.0),
            );
            util::emit_log(
                app,
                "导入",
                "info",
                &format!(
                    "{} → {}（{}）",
                    file_name,
                    info.app_name,
                    info.package_name
                        .clone()
                        .unwrap_or_else(|| "未知包名".into())
                ),
            );
            Ok(info)
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&dir);
            Err(error)
        }
    }
}

fn parse_apk(apk: &Path) -> Result<ParsedApk, String> {
    let package = Apk::new(apk).map_err(|error| format!("解析 APK 失败：{}", error))?;
    let package_name = package.get_package_name();
    if package_name.is_none() {
        return Err("未能从 AndroidManifest.xml 解析出包名，文件可能不是有效的 APK".into());
    }

    let arsc = load_resource_table(&package);
    let application_label = package.get_application_label();

    let mut permissions = Vec::new();
    let mut permission_set = HashSet::new();
    for permission in package
        .get_permissions()
        .chain(package.get_permissions_sdk23())
    {
        if permission_set.insert(permission.to_string()) {
            permissions.push(permission.to_string());
        }
    }

    let mut native_code = package.get_supported_abis();
    native_code.sort();
    native_code.dedup();

    let (labels, icon_candidates) = match &arsc {
        Some(table) => (
            localized_labels(&package, table),
            icon_resources(&package, table),
        ),
        None => (BTreeMap::new(), Vec::new()),
    };

    Ok(ParsedApk {
        package_name: package_name.clone(),
        version_name: package.get_version_name(),
        version_code: package.get_version_code(),
        min_sdk: package.get_min_sdk_version(),
        target_sdk: package.get_attribute_value("uses-sdk", "targetSdkVersion"),
        compile_sdk: package.get_compile_sdk_version(),
        launchable_activity: package.get_main_activity().map(|activity| {
            qualify_activity(package_name.as_deref().unwrap_or_default(), activity)
        }),
        install_location: package.get_install_location(),
        debuggable: package
            .get_application_debuggable()
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
        permissions,
        native_code,
        labels,
        default_label: application_label.clone(),
        application_label,
        icon_candidates,
        fallback_icon: package.get_application_icon(),
    })
}

fn load_resource_table(package: &Apk) -> Option<ARSC> {
    let (bytes, _) = package.read("resources.arsc").ok()?;
    let mut input = bytes.as_slice();
    ARSC::new(&mut input).ok()
}

/// 读取 `<application android:label>` 的原始引用，再遍历资源表取全部语言版本。
fn localized_labels(package: &Apk, arsc: &ARSC) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    let Some(reference) = package
        .get_all_attribute_values("application", "label")
        .next()
    else {
        return labels;
    };

    for_each_reference_resource(arsc, reference, |config, value| {
        let Some(locale) = locale_key(config) else {
            return;
        };
        let text = resolve_value(arsc, value);
        if !text.is_empty() {
            labels.entry(locale).or_insert(text);
        }
    });
    labels
}

/// 遍历 `android:icon` 在资源表里的全部配置，保留引用文件路径，供后续挑最高密度。
fn icon_resources(package: &Apk, arsc: &ARSC) -> Vec<(u32, String)> {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let Some(reference) = package
        .get_all_attribute_values("application", "icon")
        .next()
    else {
        return candidates;
    };

    for_each_reference_resource(arsc, reference, |config, value| {
        let path = resolve_value(arsc, value);
        if !path.starts_with("res/") || !seen.insert(path.clone()) {
            return;
        }
        let density = u32::from(config.get_orientation_touchscreen_density().2);
        candidates.push((density, path));
    });
    candidates
}

enum ResourceReference {
    Id(u32),
    Name(String, String),
}

/// 清单里的引用有两种写法：`@string/app_name` 和资源混淆后的 `@0x7f0f0023`。
fn parse_resource_reference(reference: &str) -> Option<ResourceReference> {
    let name = reference.strip_prefix('@')?;
    if let Some(hex) = name.strip_prefix("0x").or_else(|| name.strip_prefix("0X")) {
        return u32::from_str_radix(hex, 16).ok().map(ResourceReference::Id);
    }
    let (type_name, key) = name.split_once('/')?;
    Some(ResourceReference::Name(
        type_name.to_string(),
        key.to_string(),
    ))
}

fn for_each_reference_resource<F>(arsc: &ARSC, reference: &str, visit: F)
where
    F: FnMut(&ResTableConfig, &ResourceValue),
{
    match parse_resource_reference(reference) {
        Some(ResourceReference::Id(id)) => for_each_resource_by_id(arsc, id, visit),
        Some(ResourceReference::Name(type_name, key)) => {
            for_each_resource(arsc, &type_name, &key, visit)
        }
        None => {}
    }
}

/// 按类型名 + 资源名遍历所有配置；同名资源可能在不同 `values-*`、`mipmap-*` 下各有一份。
fn for_each_resource<F>(arsc: &ARSC, type_name: &str, key: &str, mut visit: F)
where
    F: FnMut(&ResTableConfig, &ResourceValue),
{
    for package in arsc.packages() {
        for (config, type_map) in &package.resources {
            for (type_id, entries) in type_map {
                let matches_type = package
                    .type_strings
                    .get(u32::from(*type_id).saturating_sub(1))
                    .map(String::as_str)
                    == Some(type_name);
                if !matches_type {
                    continue;
                }
                for entry in &entries.entries {
                    visit_entry(entry, &package.key_strings, Some(key), config, &mut visit);
                }
            }
        }
    }
}

/// 资源名被混淆/抹掉时，按数字 ID 直接定位条目。
fn for_each_resource_by_id<F>(arsc: &ARSC, id: u32, mut visit: F)
where
    F: FnMut(&ResTableConfig, &ResourceValue),
{
    let package_id = (id >> 24) as u8;
    let type_id = ((id >> 16) & 0xff) as u8;
    let entry_id = (id & 0xffff) as u16;

    for package in arsc.packages() {
        if package.header.id as u8 != package_id {
            continue;
        }
        for (config, type_map) in &package.resources {
            let Some(entries) = type_map.get(&type_id) else {
                continue;
            };
            let Some(entry) = entries.find(entry_id) else {
                continue;
            };
            visit_entry(entry, &package.key_strings, None, config, &mut visit);
        }
    }
}

fn visit_entry<F>(
    entry: &ResTableEntry,
    keys: &StringPool,
    key: Option<&str>,
    config: &ResTableConfig,
    visit: &mut F,
) where
    F: FnMut(&ResTableConfig, &ResourceValue),
{
    match entry {
        ResTableEntry::Default(e) => {
            let matches = key.is_none_or(|key| keys.get(e.index).map(String::as_str) == Some(key));
            if matches {
                visit(config, &e.value);
            }
        }
        ResTableEntry::Compact(e) => {
            let matches =
                key.is_none_or(|key| keys.get(u32::from(e.key)).map(String::as_str) == Some(key));
            if matches {
                // 紧凑条目把数据类型放在 flags 高字节，值本身放在 data。
                let value = ResourceValue {
                    size: 8,
                    res: 0,
                    data_type: ResourceValueType::from((e.flags >> 8) as u8),
                    data: e.data,
                };
                visit(config, &value);
            }
        }
        _ => {}
    }
}

fn resolve_value(arsc: &ARSC, value: &ResourceValue) -> String {
    let text = arsc.value_to_string(value);
    match value.data_type {
        ResourceValueType::Reference | ResourceValueType::DynamicReference => {
            arsc.get_resource_value(value.data).unwrap_or(text)
        }
        _ => text,
    }
}

/// 把资源限定符里的 locale 转成 aapt 风格：`en-rUS` → `en-US`，`b+zh+Hans+CN` → `zh-Hans-CN`。
fn locale_key(config: &ResTableConfig) -> Option<String> {
    let text = config.as_string();
    let mut tokens = text.split('-');
    while let Some(token) = tokens.next() {
        if let Some(tag) = token.strip_prefix("b+") {
            return Some(tag.replace('+', "-"));
        }
        let is_language =
            (2..=3).contains(&token.len()) && token.chars().all(|ch| ch.is_ascii_lowercase());
        if !is_language {
            continue;
        }
        let mut key = token.to_string();
        if let Some(region) = tokens.next().and_then(|part| part.strip_prefix('r')) {
            if (2..=3).contains(&region.len())
                && region.chars().all(|ch| ch.is_ascii_alphanumeric())
            {
                key.push('-');
                key.push_str(region);
            }
        }
        return Some(key);
    }
    None
}

/// `aapt badging` 输出的是展开后的类名，`apk-info` 保留清单里的原样写法。
fn qualify_activity(package: &str, activity: &str) -> String {
    if let Some(rest) = activity.strip_prefix('.') {
        format!("{}{}", package, rest)
    } else if activity.contains('.') {
        activity.to_string()
    } else if package.is_empty() {
        activity.to_string()
    } else {
        format!("{}.{}", package, activity)
    }
}

/// 旧条目可能记录了提取失败；加载图标时从原 APK 补齐缓存。
pub fn ensure_icon(dir: &Path, info: &mut PackageInfo) -> Result<Option<PathBuf>, String> {
    if let Some(path) = store::icon_path(dir, info) {
        return Ok(Some(path));
    }
    let apk = store::apk_path(dir);
    let parsed = parse_apk(&apk)?;
    let Some(extension) = extract_icon(&apk, dir, &parsed) else {
        return Ok(None);
    };
    info.has_icon = true;
    info.icon_ext = Some(extension);
    store::save_info(dir, info)?;
    Ok(store::icon_path(dir, info))
}

fn extract_icon(apk: &Path, dir: &Path, parsed: &ParsedApk) -> Option<String> {
    let mut candidates: Vec<(u32, String)> = parsed
        .icon_candidates
        .iter()
        .filter(|(_, path)| is_raster(path))
        .cloned()
        .collect();
    candidates.sort_by(|left, right| right.0.cmp(&left.0));
    if let Some(fallback) = &parsed.fallback_icon {
        if is_raster(fallback) {
            candidates.push((0, fallback.clone()));
        }
    }

    let mut seen = HashSet::new();
    candidates.retain(|(_, path)| seen.insert(path.clone()));
    if candidates.is_empty() {
        return None;
    }

    let file = File::open(apk).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    for (_, path) in candidates {
        let Some(extension) = icon_extension(&path) else {
            continue;
        };
        let Ok(mut entry) = archive.by_name(&path) else {
            continue;
        };
        if entry.size() == 0 || entry.size() > MAX_ICON_BYTES {
            continue;
        }
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        if entry.read_to_end(&mut bytes).is_err() {
            continue;
        }
        let target = dir.join(format!("icon.{}", extension));
        if fs::write(&target, &bytes).is_ok() {
            return Some(extension.to_string());
        }
    }
    None
}

fn is_raster(path: &str) -> bool {
    matches!(icon_extension(path), Some("png" | "jpg" | "webp"))
}

fn icon_extension(path: &str) -> Option<&'static str> {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())?
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some("png"),
        "jpg" | "jpeg" => Some("jpg"),
        "webp" => Some("webp"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires MOBILETOOLBOX_TEST_APK pointing to a local APK"]
    fn extracts_icon_from_apk() {
        let apk = PathBuf::from(std::env::var("MOBILETOOLBOX_TEST_APK").unwrap());
        let parsed = parse_apk(&apk).unwrap();
        let dir = std::env::temp_dir().join(format!("mobiletoolbox-icon-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let extension = extract_icon(&apk, &dir, &parsed);
        let valid = extension.as_ref().is_some_and(|ext| {
            fs::metadata(dir.join(format!("icon.{ext}"))).unwrap().len() > 0
        });
        // 模拟旧版导入的无图标记录，确保加载时能够恢复并持久化。
        let mut info = PackageInfo::default();
        if let Some(ext) = &extension {
            fs::remove_file(dir.join(format!("icon.{ext}"))).unwrap();
        }
        fs::copy(&apk, dir.join("original.apk")).unwrap();
        store::save_info(&dir, &info).unwrap();
        let recovered = ensure_icon(&dir, &mut info).unwrap();
        assert!(recovered.is_some());
        assert!(store::load_info(&dir).unwrap().has_icon);
        fs::remove_dir_all(&dir).unwrap();
        assert!(valid, "APK launcher icon was not extracted");
    }
}
