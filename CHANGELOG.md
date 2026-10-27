# Changelog

本文件记录 MobileToolbox 的发版说明。发版 workflow 会自动提取对应 `## <version>` 段落作为 GitHub Release notes，请保持版本标题与 `package.json` 中的版本号完全一致。

## Unreleased

- aapt 替换为内置的 apk-info 解析器（纯 Rust），移除三平台 aapt 二进制，安装包体积减少约 5.5 MB
- 解析器直接读取 AXML/ARSC：支持数字资源 ID 混淆的 APK，按配置枚举多语言 label 与图标密度
- 桌面图标统一由 `apps/icon/` 源图生成，新增 `native/` 平台图标脚本与 `pnpm sync:icons`
- 新增应用内更新：检查更新、下载安装、重启生效，更新包由 GitHub Release 分发
- 新增 GitHub Actions 发布流程：构建 Windows / macOS / Linux 安装包并发布 updater 清单
- 新增 `pnpm release`、`pnpm sync:versions` 发版脚本

## 0.1.0

2026-10-26

首个版本。

- pnpm monorepo：`@mobiletoolbox/protocol`、`@mobiletoolbox/ui`、`@mobiletoolbox/app`
- 安装包导入、SHA-256 去重、aapt 清单解析、图标提取
- apktool 资源解包，应用内文件树与文本/图片预览
- dex2jar + CFR 反编译，Java 源码浏览
- adb 设备列表、网络连接、安装 APK、推送 OBB、卸载应用
- 实时运行日志、任务进度、拖拽导入、环境自检
- 深色界面与响应式侧边栏
- 命名定为 MobileToolbox，为后续 Android 设备日志/文件浏览和 iOS 设备管理预留扩展方向
