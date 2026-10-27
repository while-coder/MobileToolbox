#!/usr/bin/env node
const fs = require("node:fs");
const path = require("node:path");
const { generateNativeIcons } = require("../sync-icons.cjs");

function syncIcons() {
  return generateNativeIcons("macos", {
    inputs: [__filename],
    generate({ generated, output }) {
      // Tauri codegen also embeds a PNG as the default window icon on macOS.
      fs.copyFileSync(path.join(generated, "32x32.png"), path.join(output, "32x32.png"));
      fs.copyFileSync(path.join(generated, "icon.icns"), path.join(output, "icon.icns"));
    },
  });
}

module.exports = { syncIcons };
if (require.main === module) syncIcons();
