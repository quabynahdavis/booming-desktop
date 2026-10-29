// Prevents an extra console window on Windows in release builds. Standard
// Tauri shell boilerplate; a no-op on Linux/macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    booming_desktop_lib::run()
}
