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
use crate::library::scan::Scanner;
use crate::playback::{PlaybackService, PlaybackStateInfo, GainMode};

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
pub(crate) async fn get_playback_state(app: AppHandle, playback: State<'_, Arc<PlaybackService>>) -> Result<PlaybackStateInfo, String> {
    let db = get_db(&app)?;
    Ok(playback.get_state(&db))
}
