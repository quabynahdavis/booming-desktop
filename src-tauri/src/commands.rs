//! Tauri IPC surface (M0).
//!
//! One command so far: `engine_probe` — the smoke test reporting which
//! playback backend is actually live in this process. Playback IPC lands in
//! M1 alongside the real engine loop.

use tauri::State;
use tauri::AppHandle;
use tauri::Manager;
use std::sync::Arc;

use crate::engine::{EngineStatus, SharedEngine};
use crate::library::db::Db;
use crate::library::playlist::{self, Playlist, SongPath};
use crate::library::scan::Scanner;
use crate::library::cover_cache::CoverCache;
use crate::library::settings::Settings;
use crate::playback::{PlaybackService, PlaybackStateInfo, GainMode, RepeatMode};
use std::sync::OnceLock;

#[tauri::command]
pub(crate) async fn set_replay_gain_mode(playback: State<'_, Arc<PlaybackService>>, mode: GainMode) -> Result<(), String> {
    playback.set_replay_gain_mode(mode);
    Ok(())
}
use std::path::PathBuf;

/// Probe the mounted playback engine.
///
/// Payload contract: `{ engine, loaded, version, error }` — `engine` is
/// `"mpv"` or `"null"`, `loaded` reflects whether the backend's runtime
/// library/handle is usable right now, and `error` explains a degraded
/// backend (e.g. libmpv missing on this machine).
///
/// `async` keeps the dlopen/mpv-handle dance in `probe()` off the UI thread.
#[tauri::command(async)]
pub(crate) fn engine_probe(engine: State<'_, SharedEngine>) -> EngineStatus {
    let engine = engine
        .lock()
        .expect("engine mutex poisoned (engine impls never panic while held)");
    engine.probe()
}

#[tauri::command]
pub(crate) async fn scan_start(app: AppHandle, root: String) -> Result<String, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_data_dir.join("booming.db");
    let db = Db::open(db_path).map_err(|e| e.to_string())?;
    let scanner = Scanner::new(db);
    let root_path = PathBuf::from(root);
    // Simplified scan; progress events omitted for brevity
    let _ = scanner.scan_folder(root_path);
    Ok("scan_started".to_string())
}

#[tauri::command]
pub(crate) async fn scan_cancel(_id: String) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub(crate) async fn scan_status(_id: String) -> Result<String, String> {
    Ok("idle".to_string())
}

fn get_db(app: &AppHandle) -> Result<Db, String> {
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let db_path = app_data_dir.join("booming.db");
    Db::open(db_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn play_song(app: AppHandle, playback: State<'_, Arc<PlaybackService>>, song_id: i64) -> Result<(), String> {
    let db = get_db(&app)?;
    playback.play_song(&db, song_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn pause_playback(playback: State<'_, Arc<PlaybackService>>) -> Result<(), String> {
    playback.pause().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn resume_playback(playback: State<'_, Arc<PlaybackService>>) -> Result<(), String> {
    playback.resume().map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn seek_playback(playback: State<'_, Arc<PlaybackService>>, secs: f64) -> Result<(), String> {
    playback.seek(secs).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn next_track(app: AppHandle, playback: State<'_, Arc<PlaybackService>>) -> Result<(), String> {
    let db = get_db(&app)?;
    playback.next(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn previous_track(app: AppHandle, playback: State<'_, Arc<PlaybackService>>) -> Result<(), String> {
    let db = get_db(&app)?;
    playback.previous(&db).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn list_playlists(app: AppHandle) -> Result<Vec<Playlist>, String> {
    let db = get_db(&app)?;
    Ok(playlist::list_playlists(&db.connection()).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub(crate) async fn list_playlist_songs(app: AppHandle, playlist_id: i64) -> Result<Vec<SongPath>, String> {
    let db = get_db(&app)?;
    Ok(playlist::list_playlist_songs(&db.connection(), playlist_id).map_err(|e| e.to_string())?)
}

#[tauri::command]
pub(crate) async fn create_playlist(app: AppHandle, name: String, playlist_type: String) -> Result<i64, String> {
    let db = get_db(&app)?;
    let playlist = playlist::create_playlist(&db.connection(), &name, &playlist_type).map_err(|e| e.to_string())?;
    Ok(playlist.id)
}

#[tauri::command]
pub(crate) async fn add_song_to_playlist(app: AppHandle, playlist_id: i64, song_id: i64) -> Result<(), String> {
    let db = get_db(&app)?;
    playlist::add_song_to_playlist(&db.connection(), playlist_id, song_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn remove_from_playlist(app: AppHandle, playlist_id: i64, song_id: i64) -> Result<(), String> {
    let db = get_db(&app)?;
    playlist::remove_song_from_playlist(&db.connection(), playlist_id, song_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn reorder_playlist(app: AppHandle, playlist_id: i64, order: Vec<i64>) -> Result<(), String> {
    let db = get_db(&app)?;
    playlist::reorder_playlist(&db.connection(), playlist_id, order).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn import_m3u(app: AppHandle, path: String, playlist_id: i64) -> Result<(), String> {
    let db = get_db(&app)?;
    playlist::import_m3u(&db.connection(), &path, playlist_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn export_m3u(app: AppHandle, playlist_id: i64, out_path: String) -> Result<(), String> {
    let db = get_db(&app)?;
    playlist::export_m3u(&db.connection(), playlist_id, &out_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_playback_state(app: AppHandle, playback: State<'_, Arc<PlaybackService>>) -> Result<PlaybackStateInfo, String> {
    let db = get_db(&app)?;
    Ok(playback.get_state(&db))
}

fn cover_cache() -> &'static CoverCache {
    static CACHE: OnceLock<CoverCache> = OnceLock::new();
    CACHE.get_or_init(CoverCache::new)
}

#[tauri::command]
pub(crate) async fn get_cover_data_url(app: AppHandle, song_id: i64) -> Result<Option<String>, String> {
    let db = get_db(&app)?;
    let cache = cover_cache();
    Ok(cache.get_data_url(&db, song_id, 512 * 1024))
}

#[tauri::command]
pub(crate) async fn list_albums(app: AppHandle, sort: String, page: usize, page_size: usize) -> Result<Vec<crate::library::db::Album>, String> {
    use crate::library::db::SortBy;
    let db = get_db(&app)?;
    let sort_by = match sort.as_str() {
        "title" => SortBy::Title,
        "artist" => SortBy::Artist,
        "year" => SortBy::Year,
        "added" => SortBy::Added,
        "playcount" => SortBy::PlayCount,
        _ => SortBy::Title,
    };
    db.list_albums(sort_by, page, page_size).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn list_artists(app: AppHandle, sort: String, page: usize, page_size: usize) -> Result<Vec<crate::library::db::Artist>, String> {
    use crate::library::db::SortBy;
    let db = get_db(&app)?;
    let sort_by = match sort.as_str() {
        "artist" => SortBy::Artist,
        "playcount" => SortBy::PlayCount,
        _ => SortBy::Artist,
    };
    db.list_artists(sort_by, page, page_size).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn list_folders(app: AppHandle, root: String) -> Result<Vec<crate::library::db::Folder>, String> {
    let db = get_db(&app)?;
    db.list_folders(&root).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn list_genres(app: AppHandle, sort: String, page: usize, page_size: usize) -> Result<Vec<crate::library::db::Genre>, String> {
    use crate::library::db::SortBy;
    let db = get_db(&app)?;
    let sort_by = match sort.as_str() {
        "title" => SortBy::Title,
        "playcount" => SortBy::PlayCount,
        _ => SortBy::Title,
    };
    db.list_genres(sort_by, page, page_size).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn get_album_songs(app: AppHandle, album_id: String) -> Result<Vec<crate::library::db::Song>, String> {
    let db = get_db(&app)?;
    db.get_album_songs(&album_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn get_artist_songs(app: AppHandle, name: String) -> Result<Vec<crate::library::db::Song>, String> {
    let db = get_db(&app)?;
    db.get_artist_songs(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn get_recently_played(app: AppHandle, limit: usize) -> Result<Vec<crate::library::db::Song>, String> {
    let db = get_db(&app)?;
    db.get_recently_played(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn get_most_played(app: AppHandle, limit: usize) -> Result<Vec<crate::library::db::Song>, String> {
    let db = get_db(&app)?;
    db.get_most_played(limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) async fn set_repeat_mode(app: AppHandle, playback: State<'_, Arc<PlaybackService>>, mode: String) -> Result<(), String> {
    let repeat_mode = match mode.as_str() {
        "One" => RepeatMode::One,
        "All" => RepeatMode::All,
        _ => RepeatMode::None,
    };
    playback.set_repeat(repeat_mode);
    // persist to settings
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = app_data_dir.join("settings.json");
    let mut settings = Settings::load(&settings_path);
    settings.repeat_mode = mode;
    settings.save(&settings_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_repeat_mode(playback: State<'_, Arc<PlaybackService>>) -> Result<String, String> {
    let mode = playback.get_repeat();
    let s = match mode {
        RepeatMode::None => "None",
        RepeatMode::One => "One",
        RepeatMode::All => "All",
    };
    Ok(s.to_string())
}

#[tauri::command]
pub(crate) async fn set_shuffle(app: AppHandle, playback: State<'_, Arc<PlaybackService>>, enabled: bool) -> Result<(), String> {
    playback.set_shuffle(enabled);
    // persist to settings
    let app_data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let settings_path = app_data_dir.join("settings.json");
    let mut settings = Settings::load(&settings_path);
    settings.shuffle_enabled = enabled;
    settings.save(&settings_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_shuffle(playback: State<'_, Arc<PlaybackService>>) -> Result<bool, String> {
    Ok(playback.get_shuffle())
}

#[tauri::command]
pub(crate) async fn set_sleep_timer(playback: State<'_, Arc<PlaybackService>>, minutes: i64) -> Result<(), String> {
    playback.set_sleep_timer(minutes);
    Ok(())
}

#[tauri::command]
pub(crate) async fn cancel_sleep_timer(playback: State<'_, Arc<PlaybackService>>) -> Result<(), String> {
    playback.cancel_sleep_timer();
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_sleep_timer_status(playback: State<'_, Arc<PlaybackService>>) -> Result<crate::playback::SleepTimerStatus, String> {
    Ok(playback.get_sleep_timer_status())
}

#[tauri::command]
pub(crate) async fn set_volume(playback: State<'_, Arc<PlaybackService>>, vol: f64) -> Result<(), String> {
    playback.set_volume(vol);
    Ok(())
}

#[tauri::command]
pub(crate) async fn get_volume(playback: State<'_, Arc<PlaybackService>>) -> Result<f64, String> {
    Ok(playback.get_volume())
}

#[tauri::command]
pub(crate) async fn set_balance(playback: State<'_, Arc<PlaybackService>>, balance: f64) -> Result<(), String> {
    playback.set_balance(balance);
    Ok(())
}

#[tauri::command]
pub(crate) async fn set_speed(playback: State<'_, Arc<PlaybackService>>, speed: f64) -> Result<(), String> {
    playback.set_speed(speed);
    Ok(())
}
