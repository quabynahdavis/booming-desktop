# Booming Desktop

Offline music player for **Windows, macOS and Linux**. Rust backend (Tauri v2)
with a vanilla TypeScript frontend — no frontend framework.

- Product brief, milestones and environment setup: [`docs/en/`](docs/en/).
- Frontend: Vite + TypeScript (`pnpm install`, then `pnpm dev` / `pnpm build`).
- Backend: Tauri v2 crate under [`src-tauri/`](src-tauri/) (`cargo check` there
  to avoid linking system libraries during quick verification).

## Status: M0 (scaffold)

Headless-friendly verification:

```sh
pnpm install && pnpm build     # frontend (writes dist/, needed by the Rust macro)
cargo check --manifest-path src-tauri/Cargo.toml                 # with mpv feature
cargo check --manifest-path src-tauri/Cargo.toml --no-default-features  # without
```

The mpv audio backend loads `libmpv` at **runtime only** (dlopen); nothing
links against it at build time, so this repo compiles on machines without
mpv installed and simply falls back to the null engine, with a clear log.
