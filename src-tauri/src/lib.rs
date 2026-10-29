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

use engine::SharedEngine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Info-level default so the "falling back to the null engine" warning
    // from mount_engine is visible even without RUST_LOG set.
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let engine: SharedEngine = engine::mount_engine();

    tauri::Builder::default()
        .manage(engine)
        .invoke_handler(tauri::generate_handler![commands::engine_probe])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
