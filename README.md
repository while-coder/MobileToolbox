# MobileToolbox

MobileToolbox 是一个本地运行的移动应用与设备工具箱：当前支持导入 APK、查看清单与权限、解包资源、生成 JAR、反编译 Java 源码，并通过 adb 安装到真机或模拟器。所有解析和处理都在本机完成，不上传任何文件。

项目结构参考了 cliproam 的 pnpm monorepo + Vue 3 + Vite + Tauri 2 组织方式，功能对应 ScorpioPackageTool（APK 解析、解包、反编译、设备管理），界面与交互重新设计。

## 功能

- **安装包库**：多选导入或拖拽 APK，计算 SHA-256，按内容去重，持久化到工作目录。
- **清单分析**：解析包名、版本、应用名、多语言 label、SDK 区间、启动 Activity、安装位置、是否 debuggable、权限、native ABI。
- **图标提取**：优先提取最高密度的 PNG / WebP / JPG 位图；只有自适应 XML 图标时使用名称渐变头像兜底。
- **资源解包**：调用 apktool 解包到 `decoded/`，内置资源浏览器支持懒加载目录、文本预览和图片预览。
- **字节码与源码**：dex2jar 生成 `source.jar`，CFR 反编译为 Java 源码；源码树同样可以在应用内浏览。
- **设备管理**：读取 adb 设备，支持网络连接模拟器（MuMu / 夜神 / 蓝叠快捷入口）、安装安装包库中的 APK、推送 OBB、卸载应用。
- **运行日志**：底部常驻的全局日志面板，支持级别筛选、关键词搜索、复制与最大化；折叠时只占 34px 状态条，展开时向上覆盖内容区，不挤压主界面。导入、解包、反编译等长任务会显示实时进度。
- **环境自检**：启动时检测 Java、adb、apktool、dex2jar、CFR，设置页可随时重新检测。

## 路线图

当前版本聚焦 Android 安装包分析与基础设备操作，后续会沿着「移动设备工作台」继续扩展：

| 方向 | 计划能力 |
| --- | --- |
| Android 设备 | 实时 logcat、设备文件目录浏览、应用引用/依赖查看与导出 |
| iOS 设备 | 设备识别与连接、IPA 安装、设备日志、应用容器与文件浏览 |
| 通用能力 | 跨平台设备会话、任务历史与可复用的操作记录 |

iOS 能力预计通过可选的 `libimobiledevice` / `ideviceinstaller` 工具链接入，不会影响当前 Android 功能的使用。

名称选 **MobileToolbox**，是为了不再把产品限定在 APK 上：安装包只是入口，设备、日志、文件和应用引用都属于同一个观察范围。

## 界面

三块主视图：

| 视图 | 内容 |
| --- | --- |
| 安装包 | 左侧安装包列表 + 右侧详情（概览 / 文件资源 / Java 源码） |
| 设备 | 左侧 ADB 设备与连接入口 + 右侧安装、推送 OBB、卸载 |
| 设置 | 工作目录、工具链状态、数据清理、关于 |

底部常驻运行日志：折叠时保留最新一条日志与运行中任务的状态条，展开后提供级别筛选、搜索、复制和最大化，覆盖在主内容之上而不改变布局。

## 技术栈

- **桌面端**：Tauri 2 + Rust 2021
- **前端**：Vue 3.5 `<script setup>` + TypeScript + Vite 6
- **UI**：本地链接的 `@qingfeng346/ui-kit` + `lucide-vue-next`
- **共享协议**：`@mobiletoolbox/protocol`，Rust / 前端字段使用 camelCase 对齐
- **包管理**：pnpm workspace（`apps/*` + `packages/*`）
- **工具链**：内置 APK 解析器（apk-info，进程内调用）；apktool、dex2jar、CFR、adb 随应用打包在 `src-tauri/resources/tools`

## 目录结构

```text
mobiletoolbox/
├─ apps/
│  ├─ app/                    # Vue 应用 + Tauri 外壳
│  │  ├─ src/
│  │  │  ├─ platform/native.ts    # 所有 Tauri command 的唯一入口
│  │  │  ├─ stores/               # 应用 / 安装包 / 设备状态
│  │  │  ├─ features/updater/     # 应用内更新状态与弹窗
│  │  │  ├─ App.vue
│  │  │  └─ main.ts
│  │  └─ src-tauri/
│  │     ├─ src/                  # Rust 后端
│  │     ├─ native/               # 各平台图标生成脚本与说明
│  │     ├─ resources/tools/      # 内置工具链
│  │     └─ tauri.conf.json
│  └─ icon/                   # 统一图标源图与 Tauri 图标清单
├─ packages/
│  ├─ protocol/               # 前后端共享类型、常量、格式化函数
│  └─ ui/                     # 无原生依赖的展示组件
├─ scripts/                   # 图标、版本与发版脚本
├─ .github/workflows/         # Release 构建与 updater 清单发布
├─ package.json
└─ pnpm-workspace.yaml
```

### 分层约定

- `packages/ui` 只做展示，通过 props 注入 `loadDirectory` / `loadText` / `loadImage`，不直接调用 Tauri。
- `apps/app/src/platform/native.ts` 集中封装 `invoke` 与事件订阅，页面和 store 不出现裸 command 字符串。
- Rust 后端按职责拆分：`tools.rs` 环境探测、`apk.rs` 导入/解包/反编译、`adb.rs` 设备操作、`store.rs` 工作目录与路径安全、`commands.rs` Tauri 边界。

## 环境要求

- Node.js 20+，pnpm 10+
- Rust stable（1.93+）与 Tauri 2 的系统依赖
- **Java 17+**（apktool、dex2jar、CFR 需要；建议设置 `JAVA_HOME`）
- Windows 10/11（主要开发与验证平台）；macOS / Linux 需要放置对应平台的 adb 二进制

## 开发与构建

```bash
# 安装依赖
pnpm install

# 桌面端热更新开发（Vite: http://localhost:1431）
pnpm dev

# 仅启动 Web 预览（没有原生能力，导入/设备操作不可用）
pnpm dev:web

# 类型检查
pnpm check

# 构建桌面安装包
pnpm build

# 仅构建前端
pnpm build:web
```

`pnpm dev` 会先构建 `@mobiletoolbox/protocol`，然后运行 `tauri dev`。Tauri 开发与打包前会调用 `native/sync-icons.cjs` 生成当前平台图标；`build.rs` 也会在直接执行 `cargo build/check` 时做同样的事。

### 图标生成

源图与 Tauri 图标清单位于 `apps/icon/`（`icon.png` + `icons.json`）。图标按平台生成到 `apps/app/src-tauri/native/<平台>/icons/`，这些输出不纳入 Git。

```bash
# 刷新全部桌面平台图标和窗口 favicon
pnpm sync:icons

# 只生成某个平台（在 apps/app/src-tauri 下执行）
node native/windows/sync-icons.cjs
node native/macos/sync-icons.cjs
node native/linux/sync-icons.cjs
```

生成缓存位于 `.tmp/native-icons/`；源图或脚本变化后会自动重新生成。详细说明见 [apps/app/src-tauri/native/README.md](apps/app/src-tauri/native/README.md)。

## 应用内更新

桌面端通过 `tauri-plugin-updater` 从 GitHub Release 的 `updater` 标签检查更新：

- 启动后静默检查，发现新版本时弹出更新对话框，可查看更新说明、下载进度并重启生效。
- 设置页的「应用更新」卡片可手动检查，并显示当前状态。
- 更新清单地址为 `https://github.com/while-coder/MobileToolbox/releases/download/updater/latest.json`，公钥写在 `tauri.conf.json` 的 `plugins.updater` 中。

本地生成签名密钥（私钥不要提交）：

```bash
cd apps/app
pnpm tauri signer generate -w ~/.tauri/mobiletoolbox.key
```

把私钥内容写入仓库 Secrets 的 `TAURI_SIGNING_PRIVATE_KEY`（无密码时 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` 留空），并使用生成的公钥替换 `tauri.conf.json` 中的 `pubkey`。也可以在 `BUILD_SIGNING_JSON` 中提供同名的 JSON 字段。

## 发版

```bash
# 升级 patch / minor / major 版本并推送 tag
pnpm release patch
pnpm release minor
pnpm release major

# 使用指定版本
pnpm release 0.2.0

# 只同步各包版本到根 package.json（不提交）
pnpm sync:versions
```

`pnpm release` 会统一根目录、各 workspace 包与应用 Rust 的版本，提交后创建并推送 `vX.Y.Z` tag。运行前需要工作区干净；已有 tag 会在确认后删除重建。`.github/workflows/release.yml` 随后：

1. 从 `package.json` 与 `CHANGELOG.md` 解析版本和发布说明，重建 GitHub Release；
2. 在 Windows / macOS / Linux 上构建安装包并上传；
3. 生成并发布 `updater/latest.json`，供应用内更新使用。

发布流程复用 `while-coder/workflows` 的桌面构建与更新清单工作流。本地保留 ui-kit 的 `link:` 依赖；CI 通过两个包的 `ciDependencies` 字段切换到 GitHub 固定版本，无需相邻的 ui-kit 工作区。

`CHANGELOG.md` 中的版本标题必须与版本号完全一致（例如 `## 0.2.0`），否则 Release notes 会回退为默认文案。

## 调试

在 VS Code 的运行与调试面板按平台选择 **Launch App · Windows** 或 **Launch App · macOS**。两者都会依次：

1. 清理可能占用 `1431` 端口的旧 Vite 进程；
2. 构建 `@mobiletoolbox/protocol` 并编译 Rust 调试版；
3. 启动 Vite；
4. 启动 `mobiletoolbox.exe`（macOS 为 `mobiletoolbox`）。

Windows 调试依赖 VS Code 的 **C/C++** 扩展（`cppvsdbg`），macOS 依赖 **CodeLLDB** 扩展（`lldb`）。相关任务在 [.vscode/tasks.json](.vscode/tasks.json)，调试入口在 [.vscode/launch.json](.vscode/launch.json)。

## 数据目录

默认工作目录（Windows）：

```text
%APPDATA%\com.whilecode.mobiletoolbox\packages\
```

每个安装包一个目录：

```text
<文件名>-<sha256 前 10 位>/
├─ original.apk     # 导入时的 APK 副本
├─ meta.json        # 解析结果与处理状态
├─ icon.png         # 提取的图标（如可用）
├─ decoded/         # apktool 解包结果
├─ source.jar       # dex2jar 输出
└─ java/            # CFR 反编译结果
```

设置页可以打开工作目录或清空全部数据。

## 工具链说明

| 工具 | 用途 | 调用方式 |
| --- | --- | --- |
| apk-info | 解析 `AndroidManifest.xml` 与资源表 | 进程内调用（Rust crate） |
| apktool | 解包资源与 Smali | `java -jar apktool.jar d -f -o <dir> <apk>` |
| dex2jar | DEX → JAR | `java -cp lib/* com.googlecode.dex2jar.tools.Dex2jarCmd -f <apk> -o <jar>` |
| CFR | JAR → Java 源码 | `java -jar cfr.jar <jar> --outputdir <dir> --silent true` |
| adb | 设备、安装、推送 | `adb devices -l` / `install -r` / `push` / `uninstall` |

设备应用名称由随包附带的 `app-info/app-labels.dex` 通过 ADB 临时运行，使用设备当前语言读取；运行结束会删除设备上的临时文件，读取失败时回退为包名。此功能不需要在设备安装 APK，也不依赖电脑上的 Java。

修改 `apps/app/android-tools/AppLabels.java` 后，使用 JDK 重新生成随包资源：`pnpm build:android-tools`。构建脚本复用内置的 dx，不需要 Android SDK。

工具执行接口只提供 `native.adbCommand(args, options)` 和 `native.javaCommand(args, options)`，由 Rust 统一定位 ADB / Java 并执行参数数组，返回独立的 stdout / stderr；`stream: true` 会同时推送实时日志。设备操作在 `apps/app/src/platform/adb.ts`，apktool / dex2jar / CFR 流程在 `packageCommands.ts` 中组装。临时输出的清理、结果检查和替换由 TS 调用限定在安装包目录内的文件操作完成。

命令编排回归测试：`pnpm test:commands`。设置 `MOBILETOOLBOX_TEST_DEVICE` 为已授权设备序列号后，会额外验证实际设备列表与应用名称读取。

与原版 ScorpioPackageTool 使用 jd-gui 打开 JAR 不同，MobileToolbox 内置 **CFR 0.152** 直接在应用内浏览反编译源码，避免额外窗口，也兼容 Java 21。原版 jd-gui 在 Java 21 下会因初始化失败而无法启动。

## 安全与限制

- 所有文件浏览请求都会校验相对路径，阻止 `..`、绝对路径和符号链接越界；目录条目最多返回 2000 个，文本最多读取 2 MB，图片最多 10 MB。
- 导入时使用 SHA-256 前 10 位作为目录标识，避免文件名冲突和路径注入。
- APK 清单与资源表由进程内的 apk-info 解析，不再依赖 aapt；遇到无法解析的清单会报错并清理临时目录。
- 自适应图标（adaptive-icon XML）无法在没有 Android 渲染环境的情况下转成位图，会回退为字母渐变头像。
- 反编译大型 APK 可能需要数分钟，具体取决于 dex2jar 与 CFR 的速度。
- OBB 推送要求文件名符合 `main.<版本号>.<包名>.obb` 或 `patch.<版本号>.<包名>.obb`。

## License

[MIT](LICENSE)
