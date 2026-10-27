import { invoke } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { open } from "@tauri-apps/plugin-dialog"
import type {
  AndroidDevice,
  BrowserRoot,
  DirectoryListing,
  EnvironmentInfo,
  ImportOutcome,
  LogEvent,
  PackageInfo,
  ProgressEvent,
  TextPreview,
} from "@mobiletoolbox/protocol"

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

  decodePackage: (id: string) => invoke<PackageInfo>("decode_package", { id }),
  buildJar: (id: string) => invoke<PackageInfo>("build_jar", { id }),
  decompilePackage: (id: string) => invoke<PackageInfo>("decompile_package", { id }),

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

  adbDevices: () => invoke<AndroidDevice[]>("adb_devices"),
  adbConnect: (address: string) => invoke<string>("adb_connect", { address }),
  adbDisconnect: (address: string) => invoke<string>("adb_disconnect", { address }),
  adbInstall: (deviceId: string, apkPath: string) =>
    invoke<string>("adb_install", { deviceId, apkPath }),
  adbInstallPackage: (deviceId: string, id: string) =>
    invoke<string>("adb_install_package", { deviceId, id }),
  adbPushObb: (deviceId: string, filePath: string) =>
    invoke<string>("adb_push_obb", { deviceId, filePath }),
  adbUninstall: (deviceId: string, packageName: string) =>
    invoke<string>("adb_uninstall", { deviceId, packageName }),

  onLog: (handler: (event: LogEvent) => void): Promise<UnlistenFn> =>
    listen<LogEvent>("mobiletoolbox://log", (event) => handler(event.payload)),
  onProgress: (handler: (event: ProgressEvent) => void): Promise<UnlistenFn> =>
    listen<ProgressEvent>("mobiletoolbox://progress", (event) => handler(event.payload)),

  pickApkFiles: () => pickFiles([{ name: "Android 安装包", extensions: ["apk"] }], true),
  pickApkFile: () => pickFiles([{ name: "Android 安装包", extensions: ["apk"] }], false),
  pickObbFile: () => pickFiles([{ name: "OBB 数据包", extensions: ["obb"] }], false),
}