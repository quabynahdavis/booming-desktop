# Library scanning & SQLite persistence

Folder scan → `lofty` tag extraction → SQLite via `rusqlite`. This document
is a quick reference for the data pipeline implemented in
`src-tauri/src/library/`.

## Pipeline overview

```
folder ──► WalkDir ──► filter by extension ──► extract_tags (lofty)
                                       │
                                       ▼
                                TrackMeta (tags)
                                       │
                                       ▼
                           Db::insert_or_update_song
                                       │
                                       ▼
                         SQLite (rusqlite, WAL mode)
```

1. **File discovery** — `walkdir::WalkDir::new(root)` walks the tree, filters
   for files whose lowercase extension is in the allow-list, and collects the
   paths.
2. **Tag extraction** — `library::scan::tags::extract_tags(&Path)` reads the
   file with `lofty::read_from_path`, pulls the primary tag, and maps it onto
   `TrackMeta`.
3. **Cover** — `PictureType::CoverFront` picture bytes are stored in the
   `songs.cover` BLOB column (rejected if larger than 512 KiB).
4. **Persist** — `Db::insert_or_update_song` uses an `ON CONFLICT(path) DO
   UPDATE` clause, so re-scanning the same folder refreshes metadata in place.
5. **Progress** — `ScanProgress { scanned, total, current_path }` is the
   progress carrier returned by `Scanner::scan_folder`.

Allowed extensions: `mp3`, `flac`, `ogg`, `opus`, `m4a`, `aac`, `wav`, `wma`,
`ape`, `wv`.

## Current implementation status

- **Scanner is sequential.** `Scanner::scan_folder` discovers files, then
  iterates them one by one inserting rows. `rayon` is declared as a dependency
  in `Cargo.toml` but the scan loop is not yet parallelized.
- **Progress events are not emitted to the frontend yet.** The IPC command
  `scan_start` is intentionally simplified: it opens the DB at
  `<app_data_dir>/booming.db`, runs the scan synchronously, and returns
  `"scan_started"`. The `scan-progress` Tauri event is not yet wired; the
  scanner returns a `Vec<ScanProgress>` internally that is not currently
  forwarded to the UI.
- **Failures are swallowed.** `if let Ok(meta) = extract_tags(&path)` — files
  that fail tag extraction are skipped silently and counted in `scanned`.
  `insert_or_update_song` errors are ignored.

These points are tracked as future work (parallel scanning, event emission,
error reporting UI).

## SQLite schema

Created on `Db::open` with `PRAGMA journal_mode=WAL`.

```sql
CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);

CREATE TABLE IF NOT EXISTS songs (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  path              TEXT UNIQUE NOT NULL,
  title             TEXT,
  artist            TEXT,
  album             TEXT,
  album_artist      TEXT,
  track             INTEGER,
  disc              INTEGER,
  year              INTEGER,
  genre             TEXT,
  duration          INTEGER,
  cover             BLOB,
  replay_gain_track TEXT,
  replay_gain_album TEXT,
  replay_peak_track TEXT,
  replay_peak_album TEXT,
  date_added        TEXT NOT NULL,
  last_played       TEXT,
  play_count        INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS playlists (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  name         TEXT NOT NULL,
  type         TEXT NOT NULL,   -- 'user' | 'recently_played' | 'most_played'
  date_created TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS playlist_items (
  playlist_id INTEGER NOT NULL,
  song_id     INTEGER NOT NULL,
  position    INTEGER NOT NULL,
  PRIMARY KEY (playlist_id, song_id)
);

CREATE TABLE IF NOT EXISTS queue_state (
  id       INTEGER PRIMARY KEY,
  song_id  INTEGER,
  position INTEGER
);

CREATE INDEX IF NOT EXISTS idx_songs_artist ON songs(artist);
CREATE INDEX IF NOT EXISTS idx_songs_album  ON songs(album);
CREATE INDEX IF NOT EXISTS idx_songs_path   ON songs(path);
```

`schema_version` starts at `1` when the table is empty.

## `TrackMeta` fields (lofty → SQLite)

| Field | lofty source | SQLite column |
| --- | --- | --- |
| `title` | `t.title()` | `songs.title` |
| `artist` | `t.artist()` | `songs.artist` |
| `album` | `t.album()` | `songs.album` |
| `album_artist` | `t.get_string(ItemKey::AlbumArtist)` | `songs.album_artist` |
| `track` | `t.track()` | `songs.track` |
| `disc` | `t.disk()` | `songs.disc` |
| `year` | `t.get_string(ItemKey::Year)` parsed as `u32` | `songs.year` |
| `genre` | `t.genre()` | `songs.genre` |
| `duration` | `tagged.properties().duration()` → `as_secs()` | `songs.duration` |
| `cover` | `t.get_picture_type(PictureType::CoverFront)` | `songs.cover` |
| `replay_gain_track` | (not read yet — see ReplayGain section) | `songs.replay_gain_track` |
| `replay_gain_album` | (not read yet — see ReplayGain section) | `songs.replay_gain_album` |

## ReplayGain parsing

`playback::replaygain::parse_gain_db` is the shared parser used at apply time:

```rust
pub fn parse_gain_db(s: &str) -> Option<f64> {
    let val = s.split_whitespace().next()?;
    val.parse::<f64>().ok()
}
```

So `"-3.10 dB"` → `Some(-3.10)` and `"+1.5"` → `Some(1.5)`. Both `REPLAYGAIN_TRACK_GAIN` and `REPLAYGAIN_ALBUM_GAIN` tags follow this format.

`ReplayGainManager::apply_for_track` routes by mode:

- `Off` → `engine.set_replay_gain_db(0.0)` + clear `af` filter.
- `Track` → parse `songs.replay_gain_track`, apply via `set_replay_gain_db`,
  else fall back to `loudnorm`.
- `Album` → parse `songs.replay_gain_album` (falling back to track gain),
  apply, else fall back to `loudnorm`.

Note: the scanner does **not** yet populate the `replay_gain_*` columns; the
apply path exists but the tag → column wiring is a future milestone.

## Loudnorm fallback

When no gain tag is present (or parsing fails), Booming Desktop sets:

- `engine.set_replay_gain_db(0.0)`
- `engine.set_audio_filter("loudnorm")`

`loudnorm` is an mpv audio filter (EBU R128). The engine's dlopen wrapper passes the string to mpv's `af` option.

## Cover cache

`library::cover_cache` keeps decoded cover bytes in an in-memory
`HashMap<i64, Vec<u8>>` keyed by song id, skipping blobs larger than
~512 KB. The frontend gets covers via `get_cover_data_url` (base64 data URL)
instead of reading the SQLite BLOB directly.

## Quick reference: database DAO methods

`Db` (rusqlite) exposes `list_songs`, `search_songs`, `get_song`,
`increment_play_count`, `update_last_played`, `get_cover`, plus
`connection()` for playlists and M3U import/export. Playlist creation is
tracked in `library::playlist` (types `user` / `recently_played` /
`most_played`), and `reorder_playlist` rewrites `playlist_items.position`.

## Verification

```sh
cargo check --manifest-path src-tauri/Cargo.toml
cargo test  --lib --manifest-path src-tauri/Cargo.toml
```

Note: the scanner skips unreadable files; `insert_or_update_song` failures
are ignored by design (scan is best-effort). If you need strict error
reporting, add a logger/result collector in a future milestone.