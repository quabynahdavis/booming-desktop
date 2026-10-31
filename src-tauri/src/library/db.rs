use std::path::PathBuf;
use rusqlite::{params, Connection, Result as SqlResult};

#[derive(Debug, Clone, serde::Serialize)]
pub struct Song {
    pub id: i64,
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub track: Option<u32>,
    pub disc: Option<u32>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub duration: Option<u32>,
    pub cover: Option<Vec<u8>>,
    pub replay_gain_track: Option<String>,
    pub replay_gain_album: Option<String>,
    pub replay_peak_track: Option<String>,
    pub replay_peak_album: Option<String>,
    pub date_added: String,
    pub last_played: Option<String>,
    pub play_count: i64,
}

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: PathBuf) -> SqlResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);
             CREATE TABLE IF NOT EXISTS songs (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 path TEXT UNIQUE NOT NULL,
                 title TEXT,
                 artist TEXT,
                 album TEXT,
                 album_artist TEXT,
                 track INTEGER,
                 disc INTEGER,
                 year INTEGER,
                 genre TEXT,
                 duration INTEGER,
                 cover BLOB,
                 replay_gain_track TEXT,
                 replay_gain_album TEXT,
                 replay_peak_track TEXT,
                 replay_peak_album TEXT,
                 date_added TEXT NOT NULL,
                 last_played TEXT,
                 play_count INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS playlists (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 type TEXT NOT NULL,
                 date_created TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS playlist_items (
                 playlist_id INTEGER NOT NULL,
                 song_id INTEGER NOT NULL,
                 position INTEGER NOT NULL,
                 PRIMARY KEY (playlist_id, song_id)
             );
             CREATE TABLE IF NOT EXISTS queue_state (
                 id INTEGER PRIMARY KEY,
                 song_id INTEGER,
                 position INTEGER
             );
             CREATE INDEX IF NOT EXISTS idx_songs_artist ON songs(artist);
             CREATE INDEX IF NOT EXISTS idx_songs_album ON songs(album);
             CREATE INDEX IF NOT EXISTS idx_songs_path ON songs(path);
            ",
        )?;
        // Initialize schema version
        let version: i64 = conn.query_row("SELECT COUNT(*) FROM schema_version", [], |r| r.get(0)).unwrap_or(0);
        if version == 0 {
            conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
        }
        Ok(Self { conn })
    }

    pub fn insert_or_update_song(&self, meta: &crate::library::scan::TrackMeta) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO songs (path, title, artist, album, album_artist, track, disc, year, genre, duration, cover, replay_gain_track, replay_gain_album, replay_peak_track, replay_peak_album, date_added, play_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, 0)
             ON CONFLICT(path) DO UPDATE SET
                 title=excluded.title,
                 artist=excluded.artist,
                 album=excluded.album,
                 album_artist=excluded.album_artist,
                 track=excluded.track,
                 disc=excluded.disc,
                 year=excluded.year,
                 genre=excluded.genre,
                 duration=excluded.duration,
                 cover=excluded.cover,
                 replay_gain_track=excluded.replay_gain_track,
                 replay_gain_album=excluded.replay_gain_album,
                 replay_peak_track=excluded.replay_peak_track,
                 replay_peak_album=excluded.replay_peak_album",
            params![
                meta.path,
                meta.title,
                meta.artist,
                meta.album,
                meta.album_artist,
                meta.track,
                meta.disc,
                meta.year,
                meta.genre,
                meta.duration,
                meta.cover,
                meta.replay_gain_track,
                meta.replay_gain_album,
                meta.replay_peak_track,
                meta.replay_peak_album,
                now
            ],
        )?;
        Ok(())
    }

    pub fn list_songs(&self, page: usize, page_size: usize, sort: &str) -> SqlResult<Vec<Song>> {
        let offset = page * page_size;
        let order = match sort {
            "artist" => "artist",
            "album" => "album",
            "title" => "title",
            _ => "date_added",
        };
        let mut stmt = self.conn.prepare(&format!(
            "SELECT id, path, title, artist, album, album_artist, track, disc, year, genre, duration, cover, replay_gain_track, replay_gain_album, replay_peak_track, replay_peak_album, date_added, last_played, play_count FROM songs ORDER BY {} DESC LIMIT ? OFFSET ?",
            order
        ))?;
        let rows = stmt.query_map(params![page_size as i64, offset as i64], |r| {
            Ok(Song {
                id: r.get(0)?,
                path: r.get(1)?,
                title: r.get(2)?,
                artist: r.get(3)?,
                album: r.get(4)?,
                album_artist: r.get(5)?,
                track: r.get(6)?,
                disc: r.get(7)?,
                year: r.get(8)?,
                genre: r.get(9)?,
                duration: r.get(10)?,
                cover: r.get(11)?,
                replay_gain_track: r.get(12)?,
                replay_gain_album: r.get(13)?,
                replay_peak_track: r.get(14)?,
                replay_peak_album: r.get(15)?,
                date_added: r.get(16)?,
                last_played: r.get(17)?,
                play_count: r.get(18)?,
            })
        })?;
        rows.collect()
    }

    pub fn search_songs(&self, query: &str) -> SqlResult<Vec<Song>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, artist, album, album_artist, track, disc, year, genre, duration, cover, replay_gain_track, replay_gain_album, replay_peak_track, replay_peak_album, date_added, last_played, play_count FROM songs WHERE title LIKE ? OR artist LIKE ? OR album LIKE ? LIMIT 100"
        )?;
        let rows = stmt.query_map(params![&pattern, &pattern, &pattern], |r| {
            Ok(Song {
                id: r.get(0)?,
                path: r.get(1)?,
                title: r.get(2)?,
                artist: r.get(3)?,
                album: r.get(4)?,
                album_artist: r.get(5)?,
                track: r.get(6)?,
                disc: r.get(7)?,
                year: r.get(8)?,
                genre: r.get(9)?,
                duration: r.get(10)?,
                cover: r.get(11)?,
                replay_gain_track: r.get(12)?,
                replay_gain_album: r.get(13)?,
                replay_peak_track: r.get(14)?,
                replay_peak_album: r.get(15)?,
                date_added: r.get(16)?,
                last_played: r.get(17)?,
                play_count: r.get(18)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_song(&self, id: i64) -> SqlResult<Option<Song>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, artist, album, album_artist, track, disc, year, genre, duration, cover, replay_gain_track, replay_gain_album, replay_peak_track, replay_peak_album, date_added, last_played, play_count FROM songs WHERE id = ?"
        )?;
        let mut rows = stmt.query_map(params![id], |r| {
            Ok(Song {
                id: r.get(0)?,
                path: r.get(1)?,
                title: r.get(2)?,
                artist: r.get(3)?,
                album: r.get(4)?,
                album_artist: r.get(5)?,
                track: r.get(6)?,
                disc: r.get(7)?,
                year: r.get(8)?,
                genre: r.get(9)?,
                duration: r.get(10)?,
                cover: r.get(11)?,
                replay_gain_track: r.get(12)?,
                replay_gain_album: r.get(13)?,
                replay_peak_track: r.get(14)?,
                replay_peak_album: r.get(15)?,
                date_added: r.get(16)?,
                last_played: r.get(17)?,
                play_count: r.get(18)?,
            })
        })?;
        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    }

    pub fn increment_play_count(&self, id: i64) -> SqlResult<()> {
        self.conn.execute("UPDATE songs SET play_count = play_count + 1 WHERE id = ?", params![id])?;
        Ok(())
    }

    pub fn update_last_played(&self, id: i64) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        self.conn.execute("UPDATE songs SET last_played = ? WHERE id = ?", params![now, id])?;
        Ok(())
    }

    pub fn get_cover(&self, id: i64) -> SqlResult<Option<Vec<u8>>> {
        let cover: Option<Vec<u8>> = self.conn.query_row("SELECT cover FROM songs WHERE id = ?", params![id], |r| r.get(0))?;
        Ok(cover)
    }
}
