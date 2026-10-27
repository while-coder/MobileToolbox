/**
 * MobileToolbox 前后端共享协议。
 *
 * 这里只放纯数据类型、常量和格式化函数，不依赖 Vue、Node 或 Tauri，
 * 桌面端（apps/app）、共享 UI（packages/ui）和将来的 Web 端共用同一份定义。
 */

/** 与 Rust `PackageInfo` 一一对应（serde rename_all = camelCase）。 */
export interface PackageInfo {
  /** 工作目录中的唯一目录名：<文件名>-<sha256 前 10 位> */
  id: string;
  /** 导入时的原始文件名，例如 WeChat.apk */
  fileName: string;
  /** 应用显示名；优先取默认语言 label，取不到时回退包名/文件名 */
  appName: string;
  /** 内置解析器提取的全部本地化 label，键为语言标签（zh-CN、en-GB…） */
  labels: Record<string, string>;
  packageName: string | null;
  versionName: string | null;
  versionCode: string | null;
  minSdk: string | null;
  targetSdk: string | null;
  compileSdk: string | null;
  launchableActivity: string | null;
  installLocation: string | null;
  debuggable: boolean;
  permissions: string[];
  nativeCode: string[];
  /** 是否成功从 APK 中提取到可显示的图标位图 */
  hasIcon: boolean;
  /** 图标文件扩展名（png/webp/jpg），未提取时为 null */
  iconExt: string | null;
  sizeBytes: number;
  sha256: string;
  /** 导入时间，Unix 毫秒 */
  importedAt: number;
  /** apktool 是否已解包 */
  decoded: boolean;
  /** dex2jar 是否已生成 source.jar */
  hasJar: boolean;
  /** CFR 是否已生成 Java 源码 */
  decompiled: boolean;
}

export interface PackageImportError {
  file: string;
  message: string;
}

export interface ImportOutcome {
  packages: PackageInfo[];
  errors: PackageImportError[];
}

/** 任务阶段，用于按钮 spinner 与进度文案。 */
export type PackageStage =
  | "idle"
  | "importing"
  | "decoding"
  | "jar"
  | "decompiling"
  | "removing";

export type ProgressState = "running" | "done" | "error";

/** 后端通过 mobiletoolbox://progress 推送的进度事件。 */
export interface ProgressEvent {
  /** 安装包 id；导入尚未完成时为 null */
  id: string | null;
  /** 导入过程中的源文件名 */
  file: string | null;
  stage: string;
  message: string;
  state: ProgressState;
  /** 0~1，未知时为 null */
  ratio: number | null;
}

export type LogStream = "stdout" | "stderr" | "info" | "error";

/** 后端通过 mobiletoolbox://log 推送的日志行。 */
export interface LogEvent {
  time: number;
  source: string;
  stream: LogStream;
  text: string;
}

export interface ToolStatus {
  available: boolean;
  path: string | null;
  version: string | null;
  detail: string | null;
}

export interface EnvironmentInfo {
  appVersion: string;
  platform: string;
  workDir: string;
  toolsDir: string;
  java: ToolStatus;
  adb: ToolStatus;
  apktool: ToolStatus;
  dex2jar: ToolStatus;
  cfr: ToolStatus;
}

export interface AndroidDevice {
  id: string;
  state: string;
  model: string | null;
  manufacturer: string | null;
  androidRelease: string | null;
  sdkInt: string | null;
  product: string | null;
}

export interface InstalledApp {
  packageName: string;
  appName: string;
  system: boolean;
}

export interface DeviceFileEntry {
  name: string;
  directory: boolean;
  symbolicLink: boolean;
  size: number;
  modifiedAt: number;
}

export type FileKind = "text" | "image" | "binary";

export interface FileEntry {
  name: string;
  /** 相对根目录的路径，使用 / 分隔；根目录本身为空串 */
  relativePath: string;
  directory: boolean;
  size: number;
  kind: FileKind;
  extension: string;
}

export interface DirectoryListing {
  entries: FileEntry[];
  /** 条目过多时只返回前 limit 个 */
  truncated: boolean;
}

export interface TextPreview {
  content: string;
  truncated: boolean;
  size: number;
}

export type BrowserRoot = "source" | "java";

/** 文件浏览器由应用层注入的读取函数，UI 包不直接调用 Tauri。 */
export type LoadDirectory = (root: BrowserRoot, relativePath: string) => Promise<DirectoryListing>;
export type LoadText = (root: BrowserRoot, relativePath: string) => Promise<TextPreview>;
export type LoadImage = (root: BrowserRoot, relativePath: string) => Promise<string>;

/** API Level → Android 版本。新 SDK 继续往下补即可。 */
export const ANDROID_VERSIONS: Readonly<Record<string, string>> = {
  "1": "Android 1.0",
  "2": "Android 1.1",
  "3": "Android 1.5 Cupcake",
  "4": "Android 1.6 Donut",
  "5": "Android 2.0 Eclair",
  "6": "Android 2.0.1",
  "7": "Android 2.1",
  "8": "Android 2.2 Froyo",
  "9": "Android 2.3 Gingerbread",
  "10": "Android 2.3.3",
  "11": "Android 3.0 Honeycomb",
  "12": "Android 3.1",
  "13": "Android 3.2",
  "14": "Android 4.0 Ice Cream Sandwich",
  "15": "Android 4.0.3",
  "16": "Android 4.1 Jelly Bean",
  "17": "Android 4.2",
  "18": "Android 4.3",
  "19": "Android 4.4 KitKat",
  "20": "Android 4.4W",
  "21": "Android 5.0 Lollipop",
  "22": "Android 5.1",
  "23": "Android 6.0 Marshmallow",
  "24": "Android 7.0 Nougat",
  "25": "Android 7.1",
  "26": "Android 8.0 Oreo",
  "27": "Android 8.1",
  "28": "Android 9 Pie",
  "29": "Android 10",
  "30": "Android 11",
  "31": "Android 12",
  "32": "Android 12L",
  "33": "Android 13",
  "34": "Android 14",
  "35": "Android 15",
  "36": "Android 16",
  "37": "Android 17",
};

export function androidName(api: string | null | undefined): string {
  if (!api) return "未知";
  return ANDROID_VERSIONS[api.trim()] ?? `API ${api}`;
}

/** "Android 13 (API 33)" */
export function androidLabel(api: string | null | undefined): string {
  if (!api) return "未知";
  const name = ANDROID_VERSIONS[api.trim()];
  return name ? `${name} (API ${api})` : `API ${api}`;
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "-";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value >= 100 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
}

export function formatDateTime(timestamp: number): string {
  if (!Number.isFinite(timestamp) || timestamp <= 0) return "-";
  const date = new Date(timestamp);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function formatTime(timestamp: number): string {
  const date = new Date(timestamp);
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

const TEXT_EXTENSIONS = new Set([
  "txt", "md", "markdown", "log", "csv", "tsv", "ini", "cfg", "conf", "properties",
  "xml", "json", "json5", "yml", "yaml", "toml", "gradle", "pro", "map",
  "java", "kt", "kts", "smali", "js", "mjs", "cjs", "ts", "tsx", "jsx", "vue",
  "html", "htm", "css", "scss", "less", "sql", "sh", "bat", "cmd", "ps1",
  "c", "h", "cpp", "hpp", "cs", "go", "rs", "py", "rb", "php", "lua", "dart",
  "gitignore", "gitattributes", "nomedia", "version",
]);

const IMAGE_EXTENSIONS = new Set(["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "avif", "svg"]);

export function extensionOf(name: string): string {
  const index = name.lastIndexOf(".");
  return index >= 0 ? name.slice(index + 1).toLowerCase() : "";
}

export function fileKind(name: string): FileKind {
  const extension = extensionOf(name);
  if (IMAGE_EXTENSIONS.has(extension)) return "image";
  if (TEXT_EXTENSIONS.has(extension)) return "text";
  return "binary";
}

export interface PackageStatus {
  label: string;
  tone: "default" | "success" | "warning" | "info";
}

/** 列表/详情共用的状态标签。 */
export function packageStatus(pkg: PackageInfo): PackageStatus {
  if (pkg.decompiled) return { label: "已反编译", tone: "success" };
  if (pkg.hasJar) return { label: "已生成 JAR", tone: "info" };
  if (pkg.decoded) return { label: "已解包", tone: "info" };
  return { label: "仅解析", tone: "default" };
}

export function isPackageBusy(stage: PackageStage | undefined): boolean {
  return (
    stage === "importing" ||
    stage === "decoding" ||
    stage === "jar" ||
    stage === "decompiling" ||
    stage === "removing"
  );
}

/** 按 locale 在资源表给出的 label 表中挑一个最合适的结果。 */
export function localizedLabel(pkg: PackageInfo, locale?: string): string {
  const labels = pkg.labels ?? {};
  const normalized = (locale || "").replace("_", "-").toLowerCase();
  if (normalized) {
    const exact = Object.keys(labels).find((key) => key.toLowerCase() === normalized);
    if (exact) return labels[exact];
    const language = normalized.split("-")[0];
    const sameLanguage = Object.keys(labels).find((key) => key.toLowerCase().split("-")[0] === language);
    if (sameLanguage) return labels[sameLanguage];
  }
  return pkg.appName || pkg.packageName || pkg.fileName;
}

export function matchesQuery(pkg: PackageInfo, query: string): boolean {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return true;
  return [pkg.appName, pkg.packageName, pkg.fileName, pkg.versionName, pkg.versionCode]
    .filter((value): value is string => Boolean(value))
    .some((value) => value.toLowerCase().includes(normalized));
}

/** 设备列表的展示辅助。 */
export function deviceLabel(device: AndroidDevice): string {
  const model = device.model?.trim();
  const release = device.androidRelease?.trim();
  if (model && release) return `${model} · Android ${release}`;
  return model || device.id;
}

export function adbStateLabel(state: string): string {
  switch (state) {
    case "device": return "在线";
    case "unauthorized": return "未授权";
    case "offline": return "离线";
    case "connecting": return "连接中";
    default: return state || "未知";
  }
}
