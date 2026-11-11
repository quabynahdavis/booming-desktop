import { defineConfig } from "vite";

// The fixed port matches src-tauri/tauri.conf.json (build.devUrl) so the
// Tauri webview attaches to the Vite dev server without surprises.
export default defineConfig({
  clearScreen: false,
  base: '/',
  server: {
    port: 1420,
    strictPort: true,
  },
  publicDir: 'src',
  build: {
    target: "es2022",
    rollupOptions: {
      input: {
        main: 'index.html',
      },
      output: {
        assetFileNames: (assetInfo) => {
          if (assetInfo.name?.endsWith('.json') && assetInfo.name?.includes('locales')) {
            return 'i18n/locales/[name][extname]';
          }
          return 'assets/[name]-[hash][extname]';
        },
      },
    },
  },
});
