import { fileURLToPath, URL } from "node:url";
import { defineConfig, searchForWorkspaceRoot } from "vite";
import vue from "@vitejs/plugin-vue";

const uiKitPath = fileURLToPath(new URL("../../../ui-kit", import.meta.url));

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],

  resolve: {
    dedupe: ["vue"],
  },

  // Tauri 期望固定端口，端口被占用时直接失败，避免打开错误的前端
  clearScreen: false,
  server: {
    port: 1431,
    strictPort: true,
    host: process.env.TAURI_DEV_HOST || false,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
    fs: {
      // @qingfeng346/ui-kit 以 link: 方式位于工作区之外
      allow: [searchForWorkspaceRoot(process.cwd()), uiKitPath],
    },
  },

  build: {
    target: "chrome105",
    minify: "esbuild",
    sourcemap: false,
  },
});