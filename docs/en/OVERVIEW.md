# Booming Desktop — Documentation (English)

Offline music player for **Windows, macOS and Linux**. Rust backend
(Tauri v2) + vanilla TypeScript frontend (Vite, no framework). Audio engine:
libmpv, loaded at **runtime only** (dlopen) so the whole stack compiles and
links without mpv installed.

## File index

| File | Scope |
| --- | --- |
| [`architecture/01-overview.md`](architecture/01-overview.md) | High-level architecture: Tauri shell, Rust backend modules, IPC surface, frontend, Android feature parity mapping, ASCII Module → IPC → Frontend diagram |
| [`bootstrap/01-setup.md`](bootstrap/01-setup.md) | Dev toolchain, verification matrix, runtime libmpv install, bun + Material 3 |
| [`library/01-scan.md`](library/01-scan.md) | Folder scan → lofty tags → SQLite (`rusqlite`), schema, scanner, `TrackMeta` quick reference, ReplayGain, cover cache, playlist API |
| [`playback/01-engine.md`](playback/01-engine.md) | `Engine` trait façade, `MpvEngine` dlopen strategy, gapless audio, ReplayGain, Loudnorm fallback, `BOOMING_MPV_LIBRARY` discovery order |
| [`playback/02-playback-service.md`](playback/02-playback-service.md) | `PlaybackService` (Arc<Mutex>), IPC commands, queue, repeat/shuffle ban list, `playback-state-changed` event |
| [`packaging/01-bundle.md`](packaging/01-bundle.md) | Per-OS bundling strategy, `tauri.conf.json` `bundle.resources`, placeholder stubs, `scripts/fetch-mpv-libs.sh`, release build order |
| [`CHANGELOG.md`](CHANGELOG.md) | Project and docs revision history (this language) |

## Domain map

- **architecture** — system shape, module decomposition, IPC surface, frontend design, Android parity, key architecture decisions.
- **bootstrap** — environment and toolchain facts, verification matrix, runtime library installation, bun + Material 3 adoption.
- **library** — SQLite schema, `lofty` tag extraction, `Scanner`, `scan_folder`, `Playlist`, `cover_cache`, `replaygain`.
- **playback** — `Engine` trait façade, `NullEngine`, `MpvEngine`, `PlaybackService`, ReplayGain application, queue and repeat/shuffle logic.
- **packaging** — per-OS release builds, Tauri bundle config, fetch-mpv-libs script, placeholder stubs, release step order.

## Key project rules

- Engine selection must **never crash** on a machine without libmpv: it
  degrades to the null engine and logs the reason.
- Nothing links against libmpv at build time; mpv is resolved via dlopen
  (env override: `BOOMING_MPV_LIBRARY`).
- All documentation lives under `/docs/{lang}/…`; commit subject format is
  Conventional Commits, atomic per related-file group.
