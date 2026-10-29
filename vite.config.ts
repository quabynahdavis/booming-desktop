import { defineConfig } from "vite";

// The fixed port matches src-tauri/tauri.conf.json (build.devUrl) so the
// Tauri webview attaches to the Vite dev server without surprises.
export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: "es2022",
  },
});
