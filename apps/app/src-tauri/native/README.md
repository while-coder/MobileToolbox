# 原生工程与平台资源

`native/` 保存需要维护的图标生成脚本和各平台安装包配置。各平台的 `icons/` 是自动生成内容，不纳入 Git。

| 目录 | 用途 |
| --- | --- |
| `sync-icons.cjs` | 图标生成入口：按平台分发、缓存指纹并调用 Tauri CLI。 |
| `windows/sync-icons.cjs` | 输出 `windows/icons/icon.ico`。 |
| `macos/sync-icons.cjs` | 输出 `macos/icons/icon.icns` 与 Tauri 窗口所需的 `32x32.png`。 |
| `linux/sync-icons.cjs` | 输出 32、128、256 像素 PNG。 |
| `linux/linux.desktop` | Debian 安装包的桌面启动器模板。 |
| `macos/Info.plist`、`macos/locales/` | macOS 应用语言与 adb 局域网权限说明。 |

`tauri.windows.conf.json`、`tauri.macos.conf.json`、`tauri.linux.conf.json` 仍在 `src-tauri/` 根目录，供 Tauri CLI 按平台自动加载，`bundle.icon` 指向这里生成的图标。

## 图标生成

源图位于 `apps/icon/`：

- `icon.png`：1024×1024 源图。
- `icons.json`：Tauri 图标清单，`default` 指向源图。

生成流程为 `native/sync-icons.cjs → <平台>/sync-icons.cjs → Tauri CLI`。入口负责平台分发，共用函数负责缓存和 CLI 调用，各平台脚本负责输出处理。各平台脚本也可以直接运行：

```bash
cd apps/app/src-tauri
node native/windows/sync-icons.cjs
```

输入没有变化且输出完整时复用已有结果；脚本或源图变化后会重新生成。缓存位于仓库 `.tmp/native-icons/`。

Tauri 开发和打包前会通过 `beforeDevCommand` / `beforeBuildCommand` 自动运行生成脚本；`build.rs` 也会在 `tauri_build::build()` 前按 Cargo 目标平台生成图标，因此 `cargo build/check` 不依赖预先提交的图标。运行前需安装 Node.js 和 pnpm 依赖。

根目录 `pnpm sync:icons` 会刷新全部桌面平台资源，并更新 `apps/app/public/mobiletoolbox-icon.png`。

MobileToolbox 是桌面端工具（依赖 adb 与本地 Java 工具链），因此只维护 Windows、macOS、Linux 三个平台的原生资源。
