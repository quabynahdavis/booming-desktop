# Architecture overview

Booming Desktop is an **offline music player** for Windows, macOS and Linux.
The design is deliberately small: a Rust backend (Tauri v2) that owns all
OS-facing work — audio, filesystem, SQLite — and a vanilla TypeScript
frontend that renders UI with Material 3 web components. No framework, no
bundler beyond Vite.

## High-level shape

```
┌─────────────────────────────────────────────────┐
│                  Frontend (Vite)                │
│   vanilla TypeScript + Material 3 web components │
│   (@material/web@2.5.0)                         │
└───────────────────────┬─────────────────────────┘
                        │
                        │ Tauri IPC (invoke + events)
                        │
┌───────────────────────▼─────────────────────────┐
│              Tauri shell (Rust, src-tauri)      │
│                                                 │
│  commands.rs  ──►  playback ──► engine          │
│      │                         │                │
│      │              library ◄──┘                │
│      │               │                          │
│      ▼               ▼                          │
│  PlaybackService   Db (rusqlite WAL)            │
│  Arc<Mutex<…>>     songs / playlists /          │
│                    queue_state / schema_version │
└───────────────────────┬─────────────────────────┘
                        │
                        │ dlopen (libloading) — no link-time dep
                        ▼
              libmpv (runtime only)
```

Module → IPC → Frontend

```
library::scan   ──scan-progress──►  frontend scan bar
library::db     ──playback-state-changed──►  frontend Now Playing
playback        ──get_playback_state──►  frontend UI state
engine::probe   ──engine_probe──►  frontend Diagnostics
```

## Rust backend modules

### `engine/`

- **`Engine` trait** (`engine/mod.rs`) — the single seam the rest of the app
  talks to. Methods: `name`, `probe`, `version`, `load(url)`, `play`,
  `pause`, `seek(f64)`, `get_position`, `get_duration`, `get_state`,
  `next`, `previous`, `set_replay_gain_db`, `set_audio_filter`,
  `set_gapless`.
- **`SharedEngine`** — `Arc<Mutex<Box<dyn Engine>>>`, managed by Tauri.
- **`NullEngine`** — always-available fallback. Every operation is a no-op
  returning `NotImplemented`. Never panics, never links to libmpv.
- **`MpvEngine`** — cargo feature `mpv` (default-on). Loads libmpv at
  *runtime* via dlopen (`libloading`). See `docs/en/playback/01-engine.md`.
- **`EngineLocator`** — env override → bundled resources → system sonames.

### `library/`

- **`db`** (`library/db.rs`) — rusqlite with WAL mode. Creates/maintains
  `songs`, `playlists`, `playlist_items`, `queue_state`, `schema_version`.
- **`scan`** — directory walking (walkdir), tag extraction (lofty), parallel
  processing (rayon), progress events. See `docs/en/library/01-scan.md`.
- **`playlist`** — playlists, M3U import/export, reorder, smart playlist.
- **`cover_cache`** — base64 data-URL cover cache for the player panel.
- **`settings`** — persisted app settings (JSON store).

### `playback/`

- **`PlaybackService`** (`playback/mod.rs`) — Arc<Mutex> around engine +
  queue + repeat/shuffle/replaygain state. See
  `docs/en/playback/02-playback-service.md`.
- **`replaygain`** — track/album ReplayGain parsing and application,
  Loudnorm fallback. See `docs/en/playback/01-engine.md`.

## IPC surface (Tauri v2 commands)

Frontend invokes these via `@tauri-apps/api`:

- `engine_probe` — diagnostics; returns `EngineStatus`.
- `scan_start`, `scan_cancel`, `scan_status` — library scanning.
- `play_song`, `pause_playback`, `resume_playback`, `seek_playback`,
  `next_track`, `previous_track`, `get_playback_state` — playback control.
- `set_replay_gain_mode` — `"off" | "track" | "album"`.
- Playlist CRUD + M3U import/export.
- `get_cover_data_url` — base64 cover for now-playing artwork.

Backend emits events via `AppHandle::emit`:

- `scan-progress` — scan counter updates.
- `playback-state-changed` — playback state transitions.

## Frontend

- **Toolchain**: bun 1.4.4.
- **UI**: `@material/web@2.5.0`. Per-component imports (e.g.
  `@material/web/button/filled-tonal-button.js`). No global CSS reset beyond
  Material 3 tokens. Typography uses M3 token scales.
- **State**: plain TypeScript modules, no reactive framework. The player
  panel and song list subscribe to IPC events and re-render.
- **Cover art**: served as base64 data URLs from `get_cover_data_url`.

## Android feature parity → desktop mapping

| Android (Kotlin / Compose) | Desktop (Rust / TS) | Notes |
|---|---|---|
| Media3 `MediaItem` | `TrackRef` / `Song` + Tauri path | path is the desktop identity |
| `Player.play/pause/seek` | `Engine::play/pause/seek` | trait façade; MpvEngine backend |
| `MediaSession` | `PlaybackService` + Tauri managed state | no OS media session yet on desktop |
| `MediaItemMetadata` | `lofty` → `TrackMeta` → `Db` | embedded tags preferred |
| `MediaStore` | SQLite (`rusqlite`, WAL) | schema in `library/db.rs` |
| `ReplayGain` (Media3) | `playback::replaygain` + mpv `af` | see playback docs |
| `FolderPicker` | First-run folder picker + settings store | `.booming_config.json` in app data |
| `Equalizer / AudioFx` | mpv `af` filter chain | set via `Engine::set_audio_filter` |
| `Gapless playback` | `gapless-audio=yes` + `audio-buffer=0.03` | set at engine init; see playback docs |
| `Shuffle ban list` | `last_played_ids` (last 3) | `pick_next` in `PlaybackService` |

## Key decisions

1. **No link-time dependency on libmpv.** All builds compile and link on
   machines without libmpv. Runtime dlopen only (`BOOMING_MPV_LIBRARY`
   env override → bundled resources → system sonames).
2. **Null engine is the safety net.** If mpv is missing or fails to load,
   the app degrades to `NullEngine` and logs the reason. Never a crash.
3. **bun + Material 3 web components** — no React/Vue/Svelte, no global CSS
   framework. Feature parity with Android is via `@material/web` tokens.
