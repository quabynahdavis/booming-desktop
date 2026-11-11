import { defineConfig } from "vite";
import { resolve } from "path";

// The fixed port matches src-tauri/tauri.conf.json (build.devUrl) so the
// Tauri webview attaches to the Vite dev server without surprises.
export default defineConfig({
  clearScreen: false,
  base: '/',
  server: {
    port: 1420,
    strictPort: true,
  },
  publicDir: resolve(__dirname, 'src'),
  build: {
    target: "es2022",
    rollupInputOptions: {
      external: ['@fontsource/material-symbols-outlined'],
    },
    assetsInclude: ["**/locales/**/*.json"],
  },
});
