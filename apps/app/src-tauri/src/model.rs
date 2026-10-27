use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 与前端 `PackageInfo` 一一对应。`meta.json` 也直接存这个结构。
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PackageInfo {
    pub id: String,
    pub file_name: String,
    pub app_name: String,
    pub labels: BTreeMap<String, String>,
    pub package_name: Option<String>,
    pub version_name: Option<String>,
    pub version_code: Option<String>,
    pub min_sdk: Option<String>,
    pub target_sdk: Option<String>,
    pub compile_sdk: Option<String>,
    pub launchable_activity: Option<String>,
    pub install_location: Option<String>,
    pub debuggable: bool,
    pub permissions: Vec<String>,
    pub native_code: Vec<String>,
    pub has_icon: bool,
    pub icon_ext: Option<String>,
    pub size_bytes: u64,
    pub sha256: String,
    pub imported_at: u64,
    pub decoded: bool,
    pub has_jar: bool,
    pub decompiled: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageImportError {
    pub file: String,
    pub message: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub packages: Vec<PackageInfo>,
    pub errors: Vec<PackageImportError>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentInfo {
    pub app_version: String,
    pub platform: String,
    pub work_dir: String,
    pub tools_dir: String,
    pub java: ToolStatus,
    pub adb: ToolStatus,
    pub apktool: ToolStatus,
    pub dex2jar: ToolStatus,
    pub cfr: ToolStatus,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Text,
    Image,
    Binary,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub relative_path: String,
    pub directory: bool,
    pub size: u64,
    pub kind: FileKind,
    pub extension: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryListing {
    pub entries: Vec<FileEntry>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextPreview {
    pub content: String,
    pub truncated: bool,
    pub size: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    /// Unix 毫秒
    pub time: u64,
    pub source: String,
    pub stream: String,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProgressState {
    Running,
    Done,
    Error,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub id: Option<String>,
    pub file: Option<String>,
    pub stage: String,
    pub message: String,
    pub state: ProgressState,
    pub ratio: Option<f64>,
}
