# PlaybackService & queue state

`PlaybackService` (`src-tauri/src/playback/mod.rs`) is the process-wide
controller of the audio pipeline. It wraps the `Engine` (`SharedEngine =
Arc<Mutex<Box<dyn Engine>>>`) together with queue, repeat, shuffle and
replay-gain bookkeeping, and exposes it over Tauri IPC.

## Service layout

```rust
pub struct PlaybackService {
    engine: SharedEngine,                       // Arc<Mutex<Box<dyn Engine>>>
    current_song_id: Mutex<Option<i64>>,       // in-memory queue position
    queue: Mutex<Vec<i64>>,                    // queue of song IDs (in memory)
    repeat: Mutex<RepeatMode>,                 // None | One | All
    shuffle: Mutex<bool>,
    last_played_ids: Mutex<Vec<i64>>,        // ban list (last 3 plays)
    app_handle: Mutex<Option<AppHandle>>,    // for emitting state changes
    replay_gain_mode: Mutex<GainMode>,        // Off | Track | Album
}
```

Managed by Tauri via `tauri::Builder::manage(...)` in `src-tauri/src/lib.rs`.

## IPC commands exposed (src-tauri/src/commands.rs)

| Command | Args | Behaviour |
|---|---|---|
| `engine_probe` | — | async; returns `EngineStatus` |
| `scan_start` | `root: String` | opens `<app_data>/booming.db`, runs `Scanner::scan_folder` |
| `scan_cancel` | `id: String` | no-op stub |
| `scan_status` | `id: String` | returns `"idle"` |
| `play_song` | `song_id: i64` | db lookup + `playback.play_song(&db, id)` |
| `pause_playback` | — | `playback.pause()` |
| `resume_playback` | — | `playback.resume()` |
| `seek_playback` | `secs: f64` | `playback.seek(secs)` |
| `next_track` | — | `playback.next(&db)` |
| `previous_track` | — | `playback.previous(&db)` |
| `get_playback_state` | — | returns `PlaybackStateInfo` |
| `set_replay_gain_mode` | `mode: GainMode` | Off / Track / Album |
| `list_playlists` | — | returns `Vec<Playlist>` |
| `list_playlist_songs` | `playlist_id: i64` | returns `Vec<SongPath>` |
| `create_playlist` | `name: String, playlist_type: String` | returns playlist id |
| `add_song_to_playlist` | `playlist_id: i64, song_id: i64` | adds song |
| `remove_from_playlist` | `playlist_id: i64, song_id: i64` | removes song |
| `reorder_playlist` | `playlist_id: i64, order: Vec<i64>` | reorders items |
| `import_m3u` | `path: String, playlist_id: i64` | imports M3U |
| `export_m3u` | `playlist_id: i64, out_path: String` | exports M3U |
| `get_cover_data_url` | `song_id: i64` | returns base64 cover data URL |

## `play_song` flow

```text
play_song(song_id)
  → db.get_song(song_id)                    // lookup title/path
  → current_song_id = Some(song_id)        // set before load
  → db.increment_play_count(song_id); db.update_last_played(song_id)
  → ReplayGainManager::apply_for_track(&engine, &song, mode)
  → engine.load(&path); engine.play()
  → emit_state()   // "playback-state-changed"
```

The ReplayGain mode is read from `self.replay_gain_mode` before applying.

## State transitions and events

`emit_state()` calls `AppHandle::emit("playback-state-changed", ())` — the
frontend listens for this Tauri event to refresh the player panel and queue.

`get_state(&db) -> PlaybackStateInfo` reads the engine state, position, duration,
current song and queue; it then loads the current song record via
`db.get_song(id)`.

`PlaybackStateInfo` shape:

```json
{
  "state": "playing",        // "playing" | "paused" | "stopped"
  "position": 12.5,
  "duration": 234.0,
  "current_song_id": 3,
  "current_song": { "id": 3, ... },
  "queue": [3, 7, 11]
}
```

## Queue

- The queue lives in memory as `Mutex<Vec<i64>>` (list of `songs.id` values).
- `set_queue` / `get_queue` manage it.
- The `queue_state` table in the SQLite schema is present but not yet wired
  to the in-memory queue for persistence in the current milestone.
- `pick_next` consumes the queue to produce the next song id; `pick_previous`
  backs it with the in-memory position.

## Repeat / shuffle

`RepeatMode` enum: `None`, `One`, `All`.

| Behaviour | Implementation |
|---|---|
| `Repeat: None` | next after last → no next. |
| `Repeat: One` | (handled via mpv `playlist-repeat` option set at engine init). |
| `Repeat: All` | wraps to `queue[0]`. |
| `Shuffle` | uses ban list instead of true random. |

### Shuffle ban list (last 3)

```rust
fn pick_next(&self, queue: &mut Vec<i64>, cur_id: Option<i64>) -> Result<Option<i64>, EngineError> {
    let shuffle = *self.shuffle.lock().unwrap();
    let repeat = *self.repeat.lock().unwrap();
    if shuffle {
        // simplified shuffle with ban list
        let mut last = self.last_played_ids.lock().unwrap();
        let mut candidates: Vec<i64> = queue.iter().cloned().collect();
        candidates.retain(|id| !last.contains(id));
        if candidates.is_empty() { candidates = queue.clone(); }
        if !candidates.is_empty() {
            let chosen = candidates[0];
            last.push(chosen);
            if last.len() > 3 { last.remove(0); }
            return Ok(Some(chosen));
        }
    }
    // sequential
    // ...position +1 handling with repeat All wrapping to queue[0]
}
```

The shuffle is a **simple deterministic pick**: first candidate not recently
played. Not a Fisher-Yates shuffle yet. The ban list keeps the last 3 plays
(`last_played_ids`, capped at 3 entries).

### `previous`

- Looks up current song position in queue.
- If `pos > 0`, returns `queue[pos - 1]`.
- Does not implement "restart current track" semantics yet.

## Verification

```sh
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --lib --manifest-path src-tauri/Cargo.toml
```

Frontend: the player panel polls `get_playback_state` every 500 ms and
listens for `playback-state-changed` events.

## Known gaps / deferrals

- True Fisher-Yates shuffle for the ban list.
- Shuffle should exclude the currently-playing track more robustly.
- `previous` does not restart the current track at position 0.
- `queue_state` persistence not yet wired to the in-memory queue.
- Scan cancel/status are stubs.
- Events: only `playback-state-changed` is emitted today.
