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
            commands::set_replay_gain_mode
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
