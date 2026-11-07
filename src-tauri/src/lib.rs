//! Booming Desktop Rust entry point.
//!
//! Startup shape (M0):
//! 1. logging comes up first so engine-selection fallbacks are visible;
//! 2. the strongest engine for this machine is mounted (`mpv` preferred,
//!    `null` fallback — see `engine::mount_engine`);
//! 3. the engine chassis is managed and surfaced as the `engine_probe` IPC
//!    command. Real playback arrives in M1.

mod commands;
mod engine;
mod library;
mod lyrics;
mod playback;

use std::sync::Arc;
use engine::SharedEngine;
use playback::PlaybackService;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Info-level default so the "falling back to the null engine" warning
    // from mount_engine is visible even without RUST_LOG set.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let engine: SharedEngine = engine::mount_engine();
    let playback_service = Arc::new(PlaybackService::new(engine.clone()));

    tauri::Builder::default()
        .manage(engine)
        .manage(playback_service)
        .invoke_handler(tauri::generate_handler![
            commands::engine_probe,
            commands::scan_start,
            commands::scan_cancel,
            commands::scan_status,
            commands::play_song,
            commands::pause_playback,
            commands::resume_playback,
            commands::seek_playback,
            commands::next_track,
            commands::previous_track,
            commands::get_playback_state,
            commands::set_replay_gain_mode,
            commands::list_playlists,
            commands::list_playlist_songs,
            commands::create_playlist,
            commands::add_song_to_playlist,
            commands::remove_from_playlist,
            commands::reorder_playlist,
            commands::import_m3u,
            commands::export_m3u,
            commands::get_cover_data_url,
            commands::list_albums,
            commands::list_artists,
            commands::list_folders,
            commands::list_genres,
            commands::get_album_songs,
            commands::get_artist_songs,
            commands::get_recently_played,
            commands::get_most_played,
            commands::set_repeat_mode,
            commands::get_repeat_mode,
            commands::set_shuffle,
            commands::get_shuffle,
            commands::set_sleep_timer,
            commands::cancel_sleep_timer,
            commands::get_sleep_timer_status,
            commands::set_volume,
            commands::get_volume,
            commands::set_balance,
            commands::set_speed,
            commands::parse_lrc,
            commands::save_lrc_sidecar,
            commands::load_lrc_sidecar,
            commands::delete_lrc_sidecar,
            commands::get_eq_state,
            commands::set_eq_enabled,
            commands::set_eq_band,
            commands::set_eq_bands,
            commands::load_eq_preset,
            commands::import_autoeq,
        ])
        .setup(|app| {
            // set app handle for events
            let handle = app.handle().clone();
            if let Some(ps) = app.try_state::<Arc<PlaybackService>>() {
                ps.set_app_handle(handle);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
