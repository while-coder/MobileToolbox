use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use tauri::AppHandle;
use zip::ZipArchive;

use crate::model::{EnvironmentInfo, ToolStatus};
use crate::store::AppState;
use crate::util;

pub fn java_binary() -> PathBuf {
    if let Some(home) = env::var_os("JAVA_HOME") {
        let candidate =
            PathBuf::from(home)
                .join("bin")
                .join(if cfg!(windows) { "java.exe" } else { "java" });
        if candidate.is_file() {
            return candidate;
        }
    }
    which("java").unwrap_or_else(|| PathBuf::from("java"))
}

pub fn which(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let names: Vec<String> = if cfg!(windows) {
        vec![
            name.to_string(),
            format!("{}.exe", name),
            format!("{}.cmd", name),
        ]
    } else {
        vec![name.to_string()]
    };
    for directory in env::split_paths(&path) {
        for candidate in &names {
            let file = directory.join(candidate);
            if file.is_file() {
                return Some(file);
            }
        }
    }
    None
}

pub fn adb_binary(state: &AppState) -> Option<PathBuf> {
    let bundled = if cfg!(windows) {
        state.tools_dir.join("adb/adb.exe")
    } else {
        state.tools_dir.join("adb/adb")
    };
    if bundled.is_file() {
        Some(bundled)
    } else {
        which("adb")
    }
}

pub fn apktool_jar(state: &AppState) -> PathBuf {
    state.tools_dir.join("apktool/apktool.jar")
}

pub fn cfr_jar(state: &AppState) -> PathBuf {
    state.tools_dir.join("cfr.jar")
}

pub fn dex_tools_dir(state: &AppState) -> PathBuf {
    state.tools_dir.join("dex-tools")
}

pub fn dex_tools_jar(state: &AppState) -> Option<PathBuf> {
    let directory = dex_tools_dir(state).join("lib");
    let entries = fs::read_dir(directory).ok()?;
    entries.flatten().find_map(|entry| {
        let name = entry.file_name().to_string_lossy().to_string();
        (name.starts_with("dex-tools-") && name.ends_with(".jar")).then(|| entry.path())
    })
}

pub fn require_adb(state: &AppState) -> Result<PathBuf, String> {
    adb_binary(state).ok_or_else(|| "未找到 adb，请检查工具链或系统 PATH。".to_string())
}

pub fn require_java() -> Result<PathBuf, String> {
    let java = java_binary();
    let exists = if java.is_absolute() {
        java.is_file()
    } else {
        which("java").is_some()
    };
    if exists {
        Ok(java)
    } else {
        Err("未找到 Java 运行时，请安装 JDK 17+ 并配置 JAVA_HOME。".to_string())
    }
}

pub fn build_environment(app: &AppHandle, state: &AppState) -> EnvironmentInfo {
    let java = detect_java();
    let adb = detect_binary(adb_binary(state), "adb", "version");

    let apktool_path = apktool_jar(state);
    let mut apktool = jar_status(&apktool_path, "apktool.jar", java.available);
    // apktool 的 MANIFEST 不带版本号，可用时再向命令行要一次。
    if apktool.version.is_none() && apktool.available {
        apktool.version = jar_cli_version(&apktool_path, &["--version"]);
    }
    let cfr_path = cfr_jar(state);
    let cfr = jar_status(&cfr_path, "cfr.jar", java.available);

    let dex_dir = dex_tools_dir(state);
    let dex_jar = dex_tools_jar(state);
    let dex2jar = if let Some(jar) = &dex_jar {
        let version = jar
            .file_stem()
            .and_then(|name| name.to_str())
            .map(|name| name.trim_start_matches("dex-tools-").to_string());
        ToolStatus {
            available: java.available,
            path: Some(dex_dir.display().to_string()),
            version,
            detail: if java.available {
                None
            } else {
                Some("需要 Java 运行时".into())
            },
        }
    } else {
        ToolStatus {
            available: false,
            path: Some(dex_dir.display().to_string()),
            version: None,
            detail: Some("未找到 dex-tools 运行库".into()),
        }
    };

    EnvironmentInfo {
        app_version: app.package_info().version.to_string(),
        platform: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        work_dir: state.work_dir.display().to_string(),
        tools_dir: state.tools_dir.display().to_string(),
        java,
        adb,
        apktool,
        dex2jar,
        cfr,
    }
}

fn detect_java() -> ToolStatus {
    let java = java_binary();
    let path = Some(java.display().to_string());
    match util::run_quiet("java -version", util::command(&java).arg("-version")) {
        Ok(captured) => {
            let raw = captured.combined();
            let line = raw
                .lines()
                .find(|line| line.to_lowercase().contains("version"))
                .or_else(|| raw.lines().next())
                .unwrap_or("")
                .trim()
                .replace('"', "");
            ToolStatus {
                available: true,
                path,
                version: (!line.is_empty()).then_some(line),
                detail: None,
            }
        }
        Err(error) => ToolStatus {
            available: false,
            path,
            version: None,
            detail: Some(error),
        },
    }
}

fn detect_binary(path: Option<PathBuf>, label: &str, version_arg: &str) -> ToolStatus {
    let Some(path) = path else {
        return ToolStatus {
            available: false,
            path: None,
            version: None,
            detail: Some(format!("未找到 {}", label)),
        };
    };
    let display = path.display().to_string();
    match util::run_quiet(label, util::command(&path).arg(version_arg)) {
        Ok(captured) => {
            let version = captured
                .combined()
                .lines()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_string());
            ToolStatus {
                available: true,
                path: Some(display),
                version,
                detail: None,
            }
        }
        Err(error) => ToolStatus {
            available: false,
            path: Some(display),
            version: None,
            detail: Some(error),
        },
    }
}

fn jar_status(path: &Path, label: &str, java_ready: bool) -> ToolStatus {
    let exists = path.is_file();
    let mut detail = None;
    if !exists {
        detail = Some(format!("未找到 {}", label));
    } else if !java_ready {
        detail = Some("需要 Java 运行时".into());
    }
    ToolStatus {
        available: exists && java_ready,
        path: Some(path.display().to_string()),
        version: if exists {
            jar_manifest_version(path)
        } else {
            None
        },
        detail,
    }
}

fn jar_manifest_version(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    let mut entry = archive.by_name("META-INF/MANIFEST.MF").ok()?;
    let mut raw = String::new();
    entry.read_to_string(&mut raw).ok()?;
    for line in raw.lines() {
        for key in ["Implementation-Version:", "Bundle-Version:"] {
            if let Some(value) = line.strip_prefix(key) {
                let value = value.trim();
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// 对于 MANIFEST 里没有版本的 jar，退回到命令行参数查询。
fn jar_cli_version(path: &Path, args: &[&str]) -> Option<String> {
    let java = java_binary();
    let mut cmd = util::command(&java);
    cmd.arg("-jar").arg(path);
    cmd.args(args);
    let captured = util::run_quiet("apktool --version", &mut cmd).ok()?;
    captured
        .combined()
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
}
