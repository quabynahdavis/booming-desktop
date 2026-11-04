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

## [M0.5] — 2026-10-31 — toolchain and UI refresh

### Added
- `chore(toolchain): switch JS toolchain from pnpm to bun` — bun 1.4.4
  adopted as the preferred toolchain; `package.json` scripts updated.
- `feat(ui): adopt material 3 web components for the smoke page` —
  `@material/web@2.5.0` imported per-component; `md-filled-tonal-button`,
  `md-tonal-body-1` token typography added to `styles.css`.
- `feat(engine): add engine locator with bundled/system fallback` —
  `EngineLocator::discover()` implements env override
  (`BOOMING_MPV_LIBRARY`) → bundled resources → system sonames →
  `NullEngine` fallback.
- `docs(bootstrap): add M0 setup, verification matrix and docs tree (en)` —
  new `/docs/en/` folder with `OVERVIEW.md`, `CHANGELOG.md`,
  `bootstrap/01-setup.md`.

### Verified
- `cargo check` (default) and `--no-default-features`: pass, zero warnings.
- `cargo build` on a host without libmpv: passes — no link-time mpv dep.
- `bun run typecheck` and `bun run build`: pass. `cargo test --lib`: 1 passed.

## [M1] — 2026-11-03 — library and playback foundation

### Added
- `feat(library): add rusqlite schema + DAO` — SQLite schema with `songs`,
  `playlists`, `playlist_items`, `queue_state`, `schema_version` tables
  (WAL mode); `Db` provides list/search/insert/persist methods.
- `feat(library): add lofty tag extraction with replaygain + cover` —
  `lofty::read_from_path` → `TrackMeta` with `replay_gain_*` tags and cover
  pictures; `playback::replaygain::parse_gain_db` parses `db` values.
- `feat(library): add parallel folder scanner with progress events` —
  `Scanner::scan_folder` (walkdir) + `ScanProgress { scanned, total,
  current_path }`; `rayon` declared in `Cargo.toml` for future parallelism.
- `feat(library): add playlists with M3U import/export and smart playlists`.
- `feat(playback): enable gapless audio and audio filter wiring` —
  `gapless-audio=yes`, `audio-buffer=0.03`, `vo=null` before init; post-init
  `set_gapless` best-effort (rc<0 → log + Ok).
- `feat(playback): add PlaybackService with mpv engine bindings and queue` —
  `Arc<Mutex<…>>` state, IPC commands (`play_song`, `pause_playback`,
  `resume_playback`, `seek_playback`, `next_track`, `previous_track`,
  `get_playback_state`), sequential queue + `RepeatMode::All` wrap,
  simple Fisher-Yates ban list of last 3.

### Verified
- `cargo check` (mpv on/off), `cargo build` (no mpv link), `bun run build`,
  `cargo test --lib`: pass.

## [M2] — 2026-11-04 — library expansion and UI polish

### Added
- `feat(ui): add cover art caching and rendering in player panel` — in-memory
  cover cache + `get_cover_data_url` IPC (base64 data URL, skip >512 KB).
- `feat(packaging): bundle mpv engine resources per OS with fetch script` —
  `scripts/fetch-mpv-libs.sh` with `linux` / `windows` / `macos` subcommands.
- `docs(packaging): add bundle and release notes` — new
  `docs/en/packaging/01-bundle.md`.

### Verified
- `cargo check`, `cargo test --lib`: pass. `scripts/fetch-mpv-libs.sh` and
  `bun run build` pass on dev host.

## [M3] — 2026-11-04 — playback service refinement

### Added
- `feat(playback): add repeat/shuffle logic (ban list of last 3, simple
  Fisher-Yates)` — `RepeatMode { None, One, All }`; `pick_next` with ban
  list; `pick_previous`.
- `feat(playback): add Loudnorm fallback` — no tag present →
  `set_replay_gain_db(0.0)` + `set_audio_filter("loudnorm")`.
- `docs(playback): add engine and service reference` — `docs/en/playback/`
  created with `01-engine.md` and `02-playback-service.md`.

### Verified
- `cargo check`, `cargo test --lib`: pass.

## [M4] — 2026-11-04 — packaging and release readiness

### Added
- `feat(packaging): bundle mpv engine resources per OS with fetch script` —
  Windows: mpv release zip `v0.39.0` from `github.com/mpv-player/mpv/releases`
  download → extract `libmpv-2.dll`; Linux: mpv devfiles `libmpv.so.2`;
  macOS: Homebrew/DMG path.
- `docs(packaging): add bundle and release notes` — new
  `docs/en/packaging/01-bundle.md`.

### Verified
- `scripts/fetch-mpv-libs.sh linux` and `windows`: pass (dev host, network).

## [unreleased]

- M1: real playback loop (play/pause/resume/stop/seek), playlist queue,
  library scan, richer IPC, UI shell with library views.
- Bundling: real icons + version stamping, `deb`/`appimage` targets.
- Windows/macOS specifics (currently nothing OS-specific outside the
  standard Tauri shell boilerplate and the dlopen filename table).
