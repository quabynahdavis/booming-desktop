# Changelog (English)

All notable changes to Booming Desktop. Dates in ISO-8601 (UTC).

## [0.1.0] — 2026-10-29 — M0: scaffold

### Added
- Node/toolchain bootstrap: `package.json` (bun 1.4.4), Vite 8 + TypeScript
  5.9 frontend scaffold, ignore rules, lockfile.
- Tauri v2 shell (`src-tauri`): config (`tauri.conf.json`, targets
  `deb` + `appimage`, Linux-first), capabilities (`core:default`), minimal
  generated app icons, run entry (`bin`/`lib`).
- Engine facade (`src-tauri/src/engine`):
  - `Engine` trait — `name`, `probe`, `version`, `play(path)`, `pause`,
    `resume`, `stop`, `seek(status)`, `enqueue(track)`; M0 returns
    `NotImplemented` errors from the playback stubs.
  - `NullEngine` — always-available fallback (probe reports
    `engine: "null", loaded: false`).
  - `MpvEngine` behind cargo feature `mpv` (default-on): resolves libmpv at
    **runtime** via dlopen (`libloading`), queries `mpv-version` through a
    throwaway `vo = null` handle, and degrades to `NullEngine` with a clear
    log when the library is missing.
  - `BOOMING_MPV_LIBRARY` env override for the shared-library path.
- IPC command `engine_probe` returning
  `{ engine, loaded, version, error }` (async, off the UI thread).
- Frontend smoke test: single page with a "Check engine" button rendering
  the JSON probe + human summary.
- Headless regression test: `engine::tests::mount_engine_falls_back_safely`.

### Verified
- `cargo check` (default) and `cargo check --no-default-features`: pass,
  zero warnings.
- `cargo build` on a host **without** libmpv installed: passes — proof that
  nothing links against libmpv at build time.
- `bun run typecheck` and `bun run build` (Vite 8.3.5): pass.
- `cargo test --lib`: 1 passed / 0 failed.

### Notes / decisions
- The maintained Rust bindings (`libmpv2` 6.0.0; ancestor `libmpv` 2.0.1)
  do **not** expose a `dlopen` feature in any current release — their
  `*-sys` build scripts unconditionally emit `cargo:rustc-link-lib=mpv`.
  Since M0 must compile/link on machines without libmpv, the backend ships
  a minimal, typed hand-rolled ABI table (`engine/mpv/ffi.rs`, `libloading`)
  aliasing the same function subset, abstracted behind `MpvSymbols`.
  Migration to the full `libmpv2` safe binding is an M1+ task localized to
  `engine/mpv/` once target machines can link libmpv.

## [unreleased]

- M1: real playback loop (play/pause/resume/stop/seek), playlist queue,
  library scan, richer IPC, UI shell with library views.
- Bundling: real icons + version stamping, `deb`/`appimage` targets.
- Windows/macOS specifics (currently nothing OS-specific outside the
  standard Tauri shell boilerplate and the dlopen filename table).
