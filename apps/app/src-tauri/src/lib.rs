mod apk;
mod commands;
mod logcat;
mod model;
mod store;
mod tools;
mod util;

use std::fs;
use std::path::PathBuf;

use tauri::Manager;

use store::AppState;

/// Tauri 在 Windows 上可能返回 verbatim 路径（`\\?\E:\...`），
/// Java 启动器无法访问这种路径，需要在进入工具链之前还原成普通路径。
fn normalize_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy().to_string();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{}", rest));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
}

/// 开发时用 CARGO_MANIFEST_DIR，打包后用资源目录；两处布局都兼容。
fn resolve_tools_dir(app: &tauri::AppHandle) -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(resource) = app.path().resource_dir() {
        candidates.push(resource.join("resources/tools"));
        candidates.push(resource.join("tools"));
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("resources/tools"));
            candidates.push(directory.join("tools"));
        }
    }
    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/tools"));

    candidates
        .iter()
        .find(|path| path.join("apktool/apktool.jar").is_file())
        .cloned()
        .unwrap_or_else(|| candidates.first().cloned().unwrap_or_default())
}

pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());

    // 应用内更新与重启只在桌面端注册。
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build());

    builder
        .manage(logcat::LogcatState::default())
        .setup(|app| {
            let base = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
            let base = normalize_path(base);
            let work_dir = base.join("packages");
            if let Err(error) = fs::create_dir_all(&work_dir) {
                eprintln!("创建安装包目录失败：{}", error);
            }
            let tools_dir = normalize_path(resolve_tools_dir(app.handle()));
            app.manage(AppState {
                work_dir,
                tools_dir,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_environment,
            commands::runtime_paths,
            commands::adb_command,
            logcat::start_logcat,
            logcat::stop_logcat,
            logcat::export_logcat,
            logcat::export_logcat_session,
            logcat::open_logcat_cache,
            logcat::clear_logcat_cache,
            commands::java_command,
            commands::package_file,
            commands::get_package_info,
            commands::list_packages,
            commands::import_packages,
            commands::remove_package,
            commands::clear_packages,
            commands::get_package_icon,
            commands::list_package_dir,
            commands::read_package_text,
            commands::read_package_image,
            commands::open_package_folder,
            commands::reveal_package,
            commands::open_path,
        ])
        .build(tauri::generate_context!())
        .expect("MobileToolbox 启动失败")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<logcat::LogcatState>().stop();
            }
        });
}
