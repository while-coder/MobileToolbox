import { Channel, invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { open, save } from "@tauri-apps/plugin-dialog"
import type {
  BrowserRoot,
  DirectoryListing,
  EnvironmentInfo,
  ImportOutcome,
  LogEvent,
  PackageInfo,
  ProgressEvent,
  TextPreview,
} from "@mobiletoolbox/protocol"

export interface CommandOutput { stdout: string; stderr: string }
export interface CommandOptions { stream?: boolean; source?: string; cwd?: string }
export interface LogcatEvent { lines: string[]; finished: boolean; error: string | null }

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window

async function pickFiles(filters: { name: string; extensions: string[] }[], multiple: boolean): Promise<string[]> {
  if (!isTauri) return []
  const result = await open({ multiple, directory: false, filters })
  if (!result) return []
  return Array.isArray(result) ? result : [result]
}

/**
 * 所有 Tauri 命令的唯一入口。
 * 命令参数按 Tauri 约定使用 camelCase，Rust 侧为 snake_case。
 */
export const native = {
  isTauri,

  environment: () => invoke<EnvironmentInfo>("get_environment"),
  listPackages: () => invoke<PackageInfo[]>("list_packages"),
  importPackages: (paths: string[]) => invoke<ImportOutcome>("import_packages", { paths }),
  removePackage: (id: string) => invoke<void>("remove_package", { id }),
  clearPackages: () => invoke<void>("clear_packages"),

  getPackageIcon: (id: string) => invoke<string | null>("get_package_icon", { id }),
  listPackageDirectory: (id: string, root: BrowserRoot, relativePath: string) =>
    invoke<DirectoryListing>("list_package_dir", { id, root, relativePath }),
  readPackageText: (id: string, root: BrowserRoot, relativePath: string) =>
    invoke<TextPreview>("read_package_text", { id, root, relativePath }),
  readPackageImage: (id: string, root: BrowserRoot, relativePath: string) =>
    invoke<string>("read_package_image", { id, root, relativePath }),

  openPackageFolder: (id: string, kind: "root" | "source" | "java") =>
    invoke<void>("open_package_folder", { id, kind }),
  revealPackage: (id: string) => invoke<void>("reveal_package", { id }),
  openPath: (path: string) => invoke<void>("open_path", { path }),

  runtimePaths: () => invoke<{ workDir: string; toolsDir: string }>("runtime_paths"),
  adbCommand: (args: string[], options: CommandOptions = {}) => invoke<CommandOutput>("adb_command", { args, ...options }),
  startLogcat: (deviceId: string, sessionId: string, handler: (event: LogcatEvent) => void) => {
    const onEvent = new Channel<LogcatEvent>()
    onEvent.onmessage = handler
    return invoke<string>("start_logcat", { deviceId, sessionId, onEvent })
  },
  stopLogcat: (sessionId: string) => invoke<void>("stop_logcat", { sessionId }),
  exportLogcat: (path: string, text: string) => invoke<void>("export_logcat", { path, text }),
  exportLogcatSession: (sessionId: string, path: string) => invoke<number>("export_logcat_session", { sessionId, path }),
  openLogcatCache: () => invoke<void>("open_logcat_cache"),
  clearLogcatCache: () => invoke<void>("clear_logcat_cache"),
  pickLogcatFile: () => save({ defaultPath: `logcat-${Date.now()}.log`, filters: [{ name: "日志文件", extensions: ["log", "txt"] }] }),
  javaCommand: (args: string[], options: CommandOptions = {}) => invoke<CommandOutput>("java_command", { args, ...options }),
  packageFile: (id: string, action: "isFile" | "hasEntries" | "remove" | "rename", path: string, destination?: string) =>
    invoke<boolean>("package_file", { id, action, path, destination }),
  getPackageInfo: (id: string) => invoke<PackageInfo>("get_package_info", { id }),

  onLog: (handler: (event: LogEvent) => void): Promise<UnlistenFn> =>
    listen<LogEvent>("mobiletoolbox://log", (event) => handler(event.payload)),
  onProgress: (handler: (event: ProgressEvent) => void): Promise<UnlistenFn> =>
    listen<ProgressEvent>("mobiletoolbox://progress", (event) => handler(event.payload)),

  pickApkFiles: () => pickFiles([{ name: "Android 安装包", extensions: ["apk"] }], true),
  pickApkFile: () => pickFiles([{ name: "Android 安装包", extensions: ["apk"] }], false),
  pickObbFile: () => pickFiles([{ name: "OBB 数据包", extensions: ["obb"] }], false),
}
