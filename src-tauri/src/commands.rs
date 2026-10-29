//! Tauri IPC surface (M0).
//!
//! One command so far: `engine_probe` — the smoke test reporting which
//! playback backend is actually live in this process. Playback IPC lands in
//! M1 alongside the real engine loop.

use tauri::State;

use crate::engine::{EngineStatus, SharedEngine};

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
