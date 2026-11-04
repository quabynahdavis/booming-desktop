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
use crate::playback::{PlaybackService, PlaybackStateInfo, GainMode};
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
