#!/usr/bin/env node

const { execFileSync } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

const rootDirectory = path.resolve(__dirname, "..");
const appDirectory = path.join(rootDirectory, "apps", "app");
const iconSourceDirectory = path.join(rootDirectory, "apps", "icon");
const sourceIcon = path.join(iconSourceDirectory, "icon.png");
const iconManifest = path.join(iconSourceDirectory, "icons.json");
const generatedIconsDirectory = path.join(rootDirectory, ".tmp", "app-icons");
const favicon = path.join(appDirectory, "public", "mobiletoolbox-icon.png");
const { syncNativeIcons } = require("../apps/app/src-tauri/native/sync-icons.cjs");

for (const input of [sourceIcon, iconManifest]) {
  if (!fs.existsSync(input)) {
    console.error(`图标输入不存在：${input}`);
    process.exit(1);
  }
}

const tauriCli = require.resolve("@tauri-apps/cli/tauri.js", {
  paths: [appDirectory],
});

execFileSync(
  process.execPath,
  [tauriCli, "icon", iconManifest, "--output", generatedIconsDirectory],
  { cwd: rootDirectory, stdio: "inherit" },
);

fs.mkdirSync(path.dirname(favicon), { recursive: true });
fs.copyFileSync(path.join(generatedIconsDirectory, "128x128.png"), favicon);

// 逐平台生成原生图标；各平台脚本只负责从 Tauri 输出中挑选需要的文件。
for (const platform of ["windows", "macos", "linux"]) syncNativeIcons(platform);

console.log(`已从 ${path.relative(rootDirectory, sourceIcon)} 刷新主窗口图标：${path.relative(rootDirectory, favicon)}`);
console.log("已刷新 Windows、macOS、Linux 原生图标");