use std::path::PathBuf;
use rusqlite::{params, Connection, Result as SqlResult};
use crate::scrobble::ScrobbleEvent;

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

#[derive(Debug, Clone, serde::Serialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<u32>,
    pub song_count: i64,
    pub cover: Option<Vec<u8>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Artist {
    pub name: String,
    pub song_count: i64,
    pub album_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Folder {
    pub path: String,
    pub name: String,
    pub song_count: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Genre {
    pub name: String,
    pub song_count: i64,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortBy {
    Title,
    Artist,
    Year,
    Added,
    PlayCount,
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
             CREATE TABLE IF NOT EXISTS scrobble_queue (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 source TEXT NOT NULL,
                 artist TEXT NOT NULL,
                 title TEXT NOT NULL,
                 track_ts_ms INTEGER NOT NULL,
                 duration_ms INTEGER NOT NULL,
                 hash TEXT NOT NULL UNIQUE
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

    pub fn connection(&self) -> &Connection {
        &self.conn
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

    pub fn list_albums(&self, sort: SortBy, page: usize, page_size: usize) -> SqlResult<Vec<Album>> {
        let offset = page * page_size;
        let order = match sort {
            SortBy::Title => "album",
            SortBy::Artist => "artist",
            SortBy::Year => "year",
            SortBy::Added => "min_date_added",
            SortBy::PlayCount => "song_count",
        };
        let desc = !matches!(sort, SortBy::Title | SortBy::Artist); // simple heuristic
        let order_sql = if desc { format!("{order} DESC") } else { format!("{order} ASC") };
        let sql = format!(
            "SELECT COALESCE(album, '') as album, COALESCE(artist, '') as artist, COALESCE(album_artist, '') as album_artist, MAX(year) as year, COUNT(*) as song_count, MIN(date_added) as min_date_added, MAX(play_count) as max_play_count, (SELECT cover FROM songs s2 WHERE s2.album = s1.album LIMIT 1) as cover FROM songs s1 WHERE album IS NOT NULL AND album != '' GROUP BY album, artist, album_artist ORDER BY {order_sql} LIMIT ? OFFSET ?"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![page_size as i64, offset as i64], |r| {
            Ok(Album {
                id: format!("{}::{}", r.get::<_, String>(0)?, r.get::<_, String>(1)?),
                title: r.get(0)?,
                artist: Some(r.get::<_, String>(1)?),
                album_artist: Some(r.get::<_, String>(2)?),
                year: r.get(3)?,
                song_count: r.get(4)?,
                cover: r.get(7)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_artists(&self, sort: SortBy, page: usize, page_size: usize) -> SqlResult<Vec<Artist>> {
        let offset = page * page_size;
        let order = match sort {
            SortBy::Artist => "name",
            SortBy::PlayCount => "song_count",
            _ => "song_count",
        };
        let sql = format!(
            "SELECT COALESCE(artist, '') as name, COUNT(*) as song_count, COUNT(DISTINCT album) as album_count FROM songs WHERE artist IS NOT NULL AND artist != '' GROUP BY artist ORDER BY {order} DESC LIMIT ? OFFSET ?"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![page_size as i64, offset as i64], |r| {
            Ok(Artist {
                name: r.get(0)?,
                song_count: r.get(1)?,
                album_count: r.get(2)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_folders(&self, root: &str) -> SqlResult<Vec<Folder>> {
        let pattern = if root.is_empty() {
            "%".to_string()
        } else {
            format!("{}%", root)
        };
        let mut stmt = self.conn.prepare(
            "SELECT path, COUNT(*) as song_count FROM songs WHERE path LIKE ? GROUP BY substr(path, 1, instr(path, '/') )"
        )?;
        // Simplified: group by first directory component
        let rows = stmt.query_map(params![&pattern], |r| {
            let path: String = r.get(0)?;
            let name = std::path::Path::new(&path)
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(Folder {
                path,
                name,
                song_count: r.get(1)?,
            })
        })?;
        rows.collect()
    }

    pub fn list_genres(&self, sort: SortBy, page: usize, page_size: usize) -> SqlResult<Vec<Genre>> {
        let offset = page * page_size;
        let order = match sort {
            SortBy::Title => "name",
            SortBy::PlayCount => "song_count",
            _ => "song_count",
        };
        let sql = format!(
            "SELECT COALESCE(genre, 'Unknown') as name, COUNT(*) as song_count FROM songs WHERE genre IS NOT NULL GROUP BY genre ORDER BY {order} DESC LIMIT ? OFFSET ?"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![page_size as i64, offset as i64], |r| {
            Ok(Genre {
                name: r.get(0)?,
                song_count: r.get(1)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_album_songs(&self, album_id: &str) -> SqlResult<Vec<Song>> {
        let parts: Vec<&str> = album_id.split("::").collect();
        let album = parts.get(0).unwrap_or(&"");
        let artist = parts.get(1).unwrap_or(&"");
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, artist, album, album_artist, track, disc, year, genre, duration, cover, replay_gain_track, replay_gain_album, replay_peak_track, replay_peak_album, date_added, last_played, play_count FROM songs WHERE album = ? AND COALESCE(artist,'') = ? ORDER BY track ASC"
        )?;
        let rows = stmt.query_map(params![album, artist], |r| {
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

    pub fn get_artist_songs(&self, name: &str) -> SqlResult<Vec<Song>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, artist, album, album_artist, track, disc, year, genre, duration, cover, replay_gain_track, replay_gain_album, replay_peak_track, replay_peak_album, date_added, last_played, play_count FROM songs WHERE artist = ? ORDER BY album ASC, track ASC"
        )?;
        let rows = stmt.query_map(params![name], |r| {
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

    pub fn get_recently_played(&self, limit: usize) -> SqlResult<Vec<Song>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, artist, album, album_artist, track, disc, year, genre, duration, cover, replay_gain_track, replay_gain_album, replay_peak_track, replay_peak_album, date_added, last_played, play_count FROM songs WHERE last_played IS NOT NULL ORDER BY last_played DESC LIMIT ?"
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| {
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

    pub fn get_most_played(&self, limit: usize) -> SqlResult<Vec<Song>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, title, artist, album, album_artist, track, disc, year, genre, duration, cover, replay_gain_track, replay_gain_album, replay_peak_track, replay_peak_album, date_added, last_played, play_count FROM songs ORDER BY play_count DESC LIMIT ?"
        )?;
        let rows = stmt.query_map(params![limit as i64], |r| {
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

    pub fn add_scrobble_event(&self, event: &ScrobbleEvent, hash: &str) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO scrobble_queue (source, artist, title, track_ts_ms, duration_ms, hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![&event.source, &event.artist, &event.title, event.track_ts_ms, event.duration_ms, hash],
        )?;
        Ok(())
    }

    pub fn load_scrobble_queue(&self) -> SqlResult<Vec<ScrobbleEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, source, artist, title, track_ts_ms, duration_ms FROM scrobble_queue ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(ScrobbleEvent {
                id: r.get(0)?,
                source: r.get(1)?,
                artist: r.get(2)?,
                title: r.get(3)?,
                track_ts_ms: r.get(4)?,
                duration_ms: r.get(5)?,
            })
        })?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    pub fn clear_scrobble_queue(&self, ids: &[i64]) -> SqlResult<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
        let sql = format!("DELETE FROM scrobble_queue WHERE id IN ({})", placeholders.join(","));
        let params_vec = rusqlite::params_from_iter(ids.iter().map(|&i| i as i64));
        self.conn.execute(&sql, params_vec)?;
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::scan::TrackMeta;
    use std::path::PathBuf;
    use std::env;

    #[test]
    fn list_albums_groups_songs() {
        let db_path = PathBuf::from(env::temp_dir()).join("booming_test_albums.db");
        let db = Db::open(db_path.clone()).unwrap();
        let meta1 = TrackMeta {
            path: "/a/s1.mp3".to_string(),
            title: Some("S1".to_string()),
            artist: Some("A1".to_string()),
            album: Some("Album X".to_string()),
            album_artist: None,
            track: Some(1),
            disc: None,
            year: Some(2020),
            genre: None,
            duration: Some(180),
            cover: None,
            replay_gain_track: None,
            replay_gain_album: None,
            replay_peak_track: None,
            replay_peak_album: None,
        };
        let meta2 = TrackMeta {
            path: "/a/s2.mp3".to_string(),
            title: Some("S2".to_string()),
            artist: Some("A1".to_string()),
            album: Some("Album X".to_string()),
            album_artist: None,
            track: Some(2),
            disc: None,
            year: Some(2020),
            genre: None,
            duration: Some(200),
            cover: None,
            replay_gain_track: None,
            replay_gain_album: None,
            replay_peak_track: None,
            replay_peak_album: None,
        };
        db.insert_or_update_song(&meta1).unwrap();
        db.insert_or_update_song(&meta2).unwrap();
        let albums = db.list_albums(SortBy::Title, 0, 10).unwrap();
        assert_eq!(albums.len(), 1);
        assert_eq!(albums[0].title, "Album X");
        assert_eq!(albums[0].song_count, 2);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn list_genres_counts() {
        let db_path = PathBuf::from(env::temp_dir()).join("booming_test_genres.db");
        let db = Db::open(db_path.clone()).unwrap();
        let meta = TrackMeta {
            path: "/b/s1.mp3".to_string(),
            title: Some("S".to_string()),
            artist: Some("A".to_string()),
            album: None,
            album_artist: None,
            track: None,
            disc: None,
            year: None,
            genre: Some("Rock".to_string()),
            duration: None,
            cover: None,
            replay_gain_track: None,
            replay_gain_album: None,
            replay_peak_track: None,
            replay_peak_album: None,
        };
        db.insert_or_update_song(&meta).unwrap();
        let genres = db.list_genres(SortBy::Title, 0, 10).unwrap();
        assert_eq!(genres.len(), 1);
        assert_eq!(genres[0].name, "Rock");
        assert_eq!(genres[0].song_count, 1);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn get_recently_played_orders() {
        let db_path = PathBuf::from(env::temp_dir()).join("booming_test_recent.db");
        let db = Db::open(db_path.clone()).unwrap();
        let meta = TrackMeta {
            path: "/c/s1.mp3".to_string(),
            title: Some("S".to_string()),
            artist: Some("A".to_string()),
            album: None,
            album_artist: None,
            track: None,
            disc: None,
            year: None,
            genre: None,
            duration: None,
            cover: None,
            replay_gain_track: None,
            replay_gain_album: None,
            replay_peak_track: None,
            replay_peak_album: None,
        };
        db.insert_or_update_song(&meta).unwrap();
        // update last played
        db.update_last_played(1).unwrap();
        let songs = db.get_recently_played(10).unwrap();
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].id, 1);
        let _ = std::fs::remove_file(&db_path);
    }
}
