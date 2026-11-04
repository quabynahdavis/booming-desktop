use std::fmt;

use rusqlite::{params, Connection};

/// A song identified by its DB path, resolved later to a song id.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SongPath {
    pub path: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaylistType {
    User,
    RecentlyPlayed,
    MostPlayed,
}

impl PlaylistType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "user" => PlaylistType::User,
            "recently_played" => PlaylistType::RecentlyPlayed,
            "most_played" => PlaylistType::MostPlayed,
            _ => PlaylistType::User,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PlaylistType::User => "user",
            PlaylistType::RecentlyPlayed => "recently_played",
            PlaylistType::MostPlayed => "most_played",
        }
    }
}

impl From<PlaylistType> for String {
    fn from(pt: PlaylistType) -> String {
        pt.as_str().to_string()
    }
}

#[derive(Debug)]
pub enum PlaylistError {
    NotFound(String),
    SongNotFound(String),
    Io(std::io::Error),
    Rusqlite(rusqlite::Error),
    M3U(String),
}

impl fmt::Display for PlaylistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlaylistError::NotFound(msg) => write!(f, "Playlist not found: {msg}"),
            PlaylistError::SongNotFound(msg) => write!(f, "Song not found: {msg}"),
            PlaylistError::Io(e) => write!(f, "IO error: {e}"),
            PlaylistError::Rusqlite(e) => write!(f, "Rusqlite error: {e}"),
            PlaylistError::M3U(msg) => write!(f, "M3U parse error: {msg}"),
        }
    }
}

impl std::error::Error for PlaylistError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl From<std::io::Error> for PlaylistError {
    fn from(e: std::io::Error) -> Self {
        PlaylistError::Io(e)
    }
}

impl From<rusqlite::Error> for PlaylistError {
    fn from(e: rusqlite::Error) -> Self {
        PlaylistError::Rusqlite(e)
    }
}

/// A playlist in the system.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub playlist_type: PlaylistType,
    pub date_created: String,
}

/// M3U/M3U8 parser that reads `#EXTM3U`, `#EXTINF:duration,artist - title`,
/// `#EXTALBUM`, `#EXTARTIST` lines and produces `Vec<SongPath>`.
///
/// Paths are returned as-is; the caller resolves them to song ids via the DB.
pub fn parse_m3u(content: &str) -> Result<Vec<SongPath>, PlaylistError> {
    let mut songs = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_artist: Option<String> = None;
    let mut current_album: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line == "#EXTM3U" {
            current_title = None;
            current_artist = None;
            current_album = None;
            continue;
        }

        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            let rest = rest.trim();
            if let Some((dur_str, after_comma)) = rest.split_once(',') {
                // Parse duration (kept for metadata completeness; paths only are surfaced)
                let _ = dur_str.trim().parse::<f64>();
                let after = after_comma.trim();
                if let Some((artist_part, title_part)) = after.split_once(" - ") {
                    current_artist = Some(artist_part.trim().to_string());
                    current_title = Some(title_part.trim().to_string());
                } else if !after.is_empty() {
                    current_title = Some(after.to_string());
                    current_artist = None;
                }
            } else if let Some(title_part) = rest.split_once(" - ") {
                // Format without duration: "artist - title"
                current_artist = Some(title_part.0.trim().to_string());
                current_title = Some(title_part.1.trim().to_string());
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("#EXTALBUM:") {
            current_album = Some(rest.trim().to_string());
            continue;
        }

        if let Some(rest) = line.strip_prefix("#EXTARTIST:") {
            current_artist = Some(rest.trim().to_string());
            continue;
        }

        if line.starts_with('#') {
            // Other metadata/comment lines — not a song path
            continue;
        }

        // Plain line: treat as file path
        let title = current_title.clone();
        let artist = current_artist.clone();
        let album = current_album.clone();
        songs.push(SongPath {
            path: line.to_string(),
            title,
            artist,
            album,
        });

        current_title = None;
        current_artist = None;
        current_album = None;
    }

    Ok(songs)
}

/// Resolve a `SongPath` to a song id via the DB.
pub fn resolve_song_path(db: &Connection, path: &str) -> Result<Option<i64>, PlaylistError> {
    let mut stmt = db.prepare("SELECT id FROM songs WHERE path = ?")?;
    let id: Option<i64> = stmt.query_row(params![path], |r| r.get(0))?;
    Ok(id)
}

/// Create a new playlist.
pub fn create_playlist(
    db: &Connection,
    name: &str,
    playlist_type: &str,
) -> Result<Playlist, PlaylistError> {
    let playlist_type = PlaylistType::from_str(playlist_type);
    let type_str: String = playlist_type.into();
    let now = chrono::Utc::now().to_rfc3339();
    db.execute(
        "INSERT INTO playlists (name, type, date_created) VALUES (?1, ?2, ?3)",
        params![name, type_str, now],
    )?;
    let playlist_id = db.last_insert_rowid();
    Ok(Playlist {
        id: playlist_id,
        name: name.to_string(),
        playlist_type,
        date_created: now,
    })
}

/// Get or create a smart playlist by type.
pub fn get_or_create_smart_playlist(
    db: &Connection,
    playlist_type: &str,
) -> Result<Playlist, PlaylistError> {
    let pt = PlaylistType::from_str(playlist_type);
    let type_str: String = pt.into();
    let name = match pt {
        PlaylistType::RecentlyPlayed => "Recently Played",
        PlaylistType::MostPlayed => "Most Played",
        PlaylistType::User => "User",
    };
    let now = chrono::Utc::now().to_rfc3339();

    // First try to find an existing playlist of this type
    let mut stmt = db.prepare("SELECT id, name, type, date_created FROM playlists WHERE type = ?")?;
    let mut rows = stmt.query_map(params![type_str], |r| {
        Ok(Playlist {
            id: r.get(0)?,
            name: r.get(1)?,
            playlist_type: PlaylistType::from_str(&r.get::<_, String>(2)?),
            date_created: r.get(3)?,
        })
    })?;
    let existing = rows.next().and_then(|r| r.ok());
    if let Some(existing) = existing {
        return Ok(existing);
    }

    // Insert new if not found
    db.execute(
        "INSERT INTO playlists (name, type, date_created) VALUES (?1, ?2, ?3)",
        params![name, type_str, now],
    )?;
    let playlist_id = db.last_insert_rowid();
    Ok(Playlist {
        id: playlist_id,
        name: name.to_string(),
        playlist_type: pt,
        date_created: now,
    })
}

/// Add a song to a playlist (no-op if already present).
pub fn add_song_to_playlist(
    db: &Connection,
    playlist_id: i64,
    song_id: i64,
) -> Result<(), PlaylistError> {
    // Check if song already in playlist
    let mut stmt = db.prepare(
        "SELECT position FROM playlist_items WHERE playlist_id = ? AND song_id = ?",
    )?;
    let mut rows = stmt.query_map(params![playlist_id, song_id], |r| {
        r.get::<_, i32>(0)
    })?;
    let already_present = rows.next().and_then(|r| r.ok()).is_some();
    if already_present {
        // Already exists — no-op
        return Ok(());
    }
    // Add with position = max(position) + 1, or 0 if empty
    let mut stmt2 = db.prepare(
        "SELECT COALESCE(MAX(position), -1) FROM playlist_items WHERE playlist_id = ?",
    )?;
    let mut rows2 = stmt2.query_map(params![playlist_id], |r| r.get::<_, i32>(0))?;
    let max_pos: i32 = rows2.next().and_then(|r| r.ok()).unwrap_or(-1);
    let pos = max_pos + 1;
    db.execute(
        "INSERT INTO playlist_items (playlist_id, song_id, position) VALUES (?1, ?2, ?3)",
        params![playlist_id, song_id, pos],
    )?;
    Ok(())
}

/// Remove a song from a playlist and renumber remaining positions.
pub fn remove_song_from_playlist(
    db: &Connection,
    playlist_id: i64,
    song_id: i64,
) -> Result<(), PlaylistError> {
    db.execute(
        "DELETE FROM playlist_items WHERE playlist_id = ? AND song_id = ?",
        params![playlist_id, song_id],
    )?;

    // Renumber remaining items
    let mut stmt = db.prepare(
        "SELECT song_id FROM playlist_items WHERE playlist_id = ? ORDER BY position",
    )?;
    let mut rows = stmt.query_map(params![playlist_id], |r| r.get::<_, i64>(0))?;
    let mut remaining: Vec<i64> = Vec::new();
    while let Some(Ok(sid)) = rows.next() {
        remaining.push(sid);
    }
    // Delete all items for this playlist so we can renumber
    db.execute(
        "DELETE FROM playlist_items WHERE playlist_id = ?",
        params![playlist_id],
    )?;
    for (pos, song_id) in remaining.iter().enumerate() {
        db.execute(
            "INSERT INTO playlist_items (playlist_id, song_id, position) VALUES (?1, ?2, ?3)",
            params![playlist_id, song_id, pos as i32],
        )?;
    }
    Ok(())
}

/// Reorder a playlist: `order` is the full new sequence of song ids.
pub fn reorder_playlist(
    db: &Connection,
    playlist_id: i64,
    order: Vec<i64>,
) -> Result<(), PlaylistError> {
    db.execute(
        "DELETE FROM playlist_items WHERE playlist_id = ?",
        params![playlist_id],
    )?;
    for (pos, song_id) in order.iter().enumerate() {
        db.execute(
            "INSERT INTO playlist_items (playlist_id, song_id, position) VALUES (?1, ?2, ?3)",
            params![playlist_id, song_id, pos as i32],
        )?;
    }
    Ok(())
}

/// Delete a playlist and its items.
pub fn delete_playlist(db: &Connection, id: i64) -> Result<(), PlaylistError> {
    db.execute("DELETE FROM playlists WHERE id = ?", params![id])?;
    db.execute(
        "DELETE FROM playlist_items WHERE playlist_id = ?",
        params![id],
    )?;
    Ok(())
}

/// List all playlists.
pub fn list_playlists(db: &Connection) -> Result<Vec<Playlist>, PlaylistError> {
    let mut stmt = db.prepare("SELECT id, name, type, date_created FROM playlists")?;
    let rows = stmt.query_map([], |r| {
        Ok(Playlist {
            id: r.get(0)?,
            name: r.get(1)?,
            playlist_type: PlaylistType::from_str(&r.get::<_, String>(2)?),
            date_created: r.get(3)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// List songs in a playlist.
/// For user playlists this joins through `playlist_items` ordered by position;
/// for smart playlists it queries the underlying stats tables.
pub fn list_playlist_songs(
    db: &Connection,
    playlist_id: i64,
) -> Result<Vec<SongPath>, PlaylistError> {
    // First determine the playlist type
    let mut stmt = db.prepare(
        "SELECT id, name, type, date_created FROM playlists WHERE id = ?",
    )?;
    let mut rows = stmt.query_map(params![playlist_id], |r| {
        Ok(Playlist {
            id: r.get(0)?,
            name: r.get(1)?,
            playlist_type: PlaylistType::from_str(&r.get::<_, String>(2)?),
            date_created: r.get(3)?,
        })
    })?;
    let playlist = rows
        .next()
        .and_then(|r| r.ok())
        .ok_or(PlaylistError::NotFound(playlist_id.to_string()))?;
    let ptype = playlist.playlist_type;

    let sql = match ptype {
        PlaylistType::User => {
            "SELECT s.id, s.path, s.title, s.artist, s.album FROM songs s \
             JOIN playlist_items pi ON s.id = pi.song_id \
             WHERE pi.playlist_id = ? ORDER BY pi.position"
        }
        PlaylistType::RecentlyPlayed => {
            "SELECT s.id, s.path, s.title, s.artist, s.album FROM songs s \
             WHERE s.last_played IS NOT NULL \
             ORDER BY s.last_played DESC LIMIT 50"
        }
        PlaylistType::MostPlayed => {
            "SELECT s.id, s.path, s.title, s.artist, s.album FROM songs s \
             ORDER BY s.play_count DESC LIMIT 50"
        }
    };
    let mut stmt2 = db.prepare(sql)?;
    let mut rows2 = stmt2.query_map(params![playlist_id], |r| {
        Ok(SongPath {
            path: r.get(1)?,
            title: r.get(2)?,
            artist: r.get(3)?,
            album: r.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows2 {
        out.push(row?);
    }
    Ok(out)
}

/// Import songs from an M3U file into a playlist.
pub fn import_m3u(
    db: &Connection,
    path: &str,
    playlist_id: i64,
) -> Result<(), PlaylistError> {
    let content = std::fs::read_to_string(path).map_err(PlaylistError::Io)?;
    let songs = parse_m3u(&content)?;
    for song_path in songs {
        if let Some(song_id) = resolve_song_path(db, &song_path.path)? {
            let _ = add_song_to_playlist(db, playlist_id, song_id);
        }
    }
    Ok(())
}

/// Export a playlist to an M3U file.
pub fn export_m3u(
    db: &Connection,
    playlist_id: i64,
    out_path: &str,
) -> Result<(), PlaylistError> {
    let songs = list_playlist_songs(db, playlist_id)?;
    let mut lines: Vec<String> = vec!["#EXTM3U".to_string()];
    for song in &songs {
        let title = song.title.as_deref().unwrap_or("");
        let artist = song.artist.as_deref().unwrap_or("");
        let album = song.album.as_deref().unwrap_or("");
        let duration = 0u32;
        let track_line = format!(
            "#EXTINF:{},{}",
            duration,
            if artist.is_empty() {
                title.to_string()
            } else {
                format!("{} - {}", artist, title)
            }
        );
        lines.push(track_line);
        if !album.is_empty() {
            lines.push(format!("#EXTALBUM:{}", album));
        }
        if !artist.is_empty() && title != artist {
            lines.push(format!("#EXTARTIST:{}", artist));
        }
        lines.push(song.path.clone());
    }
    std::fs::write(out_path, lines.join("\n")).map_err(PlaylistError::Io)?;
    Ok(())
}

/// Smart playlist: "Recently Played" — last_played DESC, limit 50.
pub fn recently_played(db: &Connection) -> Result<Vec<i64>, PlaylistError> {
    let mut stmt = db.prepare(
        "SELECT id FROM songs WHERE last_played IS NOT NULL ORDER BY last_played DESC LIMIT 50",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Smart playlist: "Most Played" — play_count DESC, limit 50.
pub fn most_played(db: &Connection) -> Result<Vec<i64>, PlaylistError> {
    let mut stmt = db.prepare(
        "SELECT id FROM songs ORDER BY play_count DESC LIMIT 50",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, i64>(0))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::db::Db;
    use crate::library::scan::TrackMeta;
    use std::path::PathBuf;
    use std::env;

    #[test]
    fn m3u_parses_extinf_and_paths() {
        let content = "#EXTM3U\n#EXTINF:210,Artist - Title\n#EXTALBUM:Album\n/home/music/song.mp3\n";
        let songs = parse_m3u(content).unwrap();
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].path, "/home/music/song.mp3");
        assert_eq!(songs[0].artist.as_deref(), Some("Artist"));
        assert_eq!(songs[0].title.as_deref(), Some("Title"));
        assert_eq!(songs[0].album.as_deref(), Some("Album"));
    }

    #[test]
    fn playlist_create_and_add_song() {
        let db_path = PathBuf::from(env::temp_dir()).join("booming_test_playlists.db");
        let db = Db::open(db_path.clone()).unwrap();
        // Insert a test song
        let _ = db.insert_or_update_song(&TrackMeta {
            path: "/home/music/test.mp3".to_string(),
            title: Some("Test Song".to_string()),
            artist: Some("Test Artist".to_string()),
            album: Some("Test Album".to_string()),
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
        });
        // Create a playlist
        let playlist = create_playlist(&db.connection(), "Test Playlist", "user").unwrap();
        // Add the song to the playlist
        add_song_to_playlist(&db.connection(), playlist.id, 1).unwrap(); // song id 1 should be newly inserted, auto-increment
        // List songs in playlist
        let songs = list_playlist_songs(&db.connection(), playlist.id).unwrap();
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].title, Some("Test Song".to_string()));
        // Cleanup
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn m3u_import_resolves_paths() {
        let db_path = PathBuf::from(env::temp_dir()).join("booming_test_m3u.db");
        let db = Db::open(db_path.clone()).unwrap();
        // Insert a test song matching the M3U path
        let _ = db.insert_or_update_song(&TrackMeta {
            path: "/home/music/song.mp3".to_string(),
            title: Some("Imported Song".to_string()),
            artist: Some("Import Artist".to_string()),
            album: Some("Import Album".to_string()),
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
        });
        // Write a temporary M3U file
        let m3u_path = PathBuf::from(env::temp_dir()).join("test_import.m3u");
        let m3u_content = "#EXTM3U\n#EXTINF:210,Import Artist - Imported Song\n/home/music/song.mp3\n";
        std::fs::write(&m3u_path, m3u_content).unwrap();
        // Import into a new playlist
        let playlist = create_playlist(&db.connection(), "Imported Playlist", "user").unwrap();
        import_m3u(&db.connection(), m3u_path.to_str().unwrap(), playlist.id).unwrap();
        // Verify the song was added
        let songs = list_playlist_songs(&db.connection(), playlist.id).unwrap();
        assert_eq!(songs.len(), 1);
        assert_eq!(songs[0].title, Some("Imported Song".to_string()));
        // Cleanup
        let _ = std::fs::remove_file(&m3u_path);
        let _ = std::fs::remove_file(&db_path);
    }

    #[test]
    fn export_m3u_roundtrip() {
        let db_path = PathBuf::from(env::temp_dir()).join("booming_test_export.db");
        let db = Db::open(db_path.clone()).unwrap();
        // Insert a test song
        let _ = db.insert_or_update_song(&TrackMeta {
            path: "/home/music/exported.mp3".to_string(),
            title: Some("Exported Song".to_string()),
            artist: Some("Export Artist".to_string()),
            album: Some("Export Album".to_string()),
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
        });
        // Create a playlist with the song
        let playlist = create_playlist(&db.connection(), "Export Test", "user").unwrap();
        add_song_to_playlist(&db.connection(), playlist.id, 1).unwrap();
        // Export to M3U
        let export_path = PathBuf::from(env::temp_dir()).join("exported.m3u");
        export_m3u(&db.connection(), playlist.id, export_path.to_str().unwrap()).unwrap();
        // Read back and verify content
        let exported = std::fs::read_to_string(&export_path).unwrap();
        assert!(exported.contains("#EXTM3U"));
        assert!(exported.contains("#EXTINF:"));
        assert!(exported.contains("Exported Song"));
        assert!(exported.contains("Export Artist"));
        // Cleanup
        let _ = std::fs::remove_file(&export_path);
        let _ = std::fs::remove_file(&db_path);
    }
}