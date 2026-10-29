//! Booming Desktop Rust entry point.
//!
//! M0: bare Tauri shell. The playback engine facade is wired in the next
//! commits (`engine` module + `engine_probe` command).

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
