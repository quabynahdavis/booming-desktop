//! Playback engine facade.
//!
//! [`Engine`] is the single seam the rest of the app talks to. Backends:
//!
//! - [`null::NullEngine`] — always available, plays nothing. It is the
//!   process-wide fallback whenever a real backend cannot be mounted, so a
//!   missing system library degrades to it instead of crashing.
//! - `engine::mpv::MpvEngine` (cargo feature `mpv`, enabled by default) loads
//!   libmpv lazily at *runtime* via dlopen; see that module for details.
//!
//! M0 scope: identity/probe surfaces are real, playback controls are stubs.

pub mod null;

#[cfg(feature = "mpv")]
pub mod mpv;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::Serialize;

/// Reference to one playable item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackRef {
    /// Filesystem path (or backend-specific URI) of the track.
    pub path: PathBuf,
    /// Optional display title if metadata is already known.
    pub title: Option<String>,
}

impl TrackRef {
    /// M0 scaffolding note: convenience constructor for M1 callers.
    #[allow(dead_code)]
    pub fn new(path: impl Into<PathBuf>, title: impl Into<Option<String>>) -> Self {
        Self {
            path: path.into(),
            title: title.into(),
        }
    }
}

/// Seek intent passed to [`Engine::seek`].
///
/// Values are in seconds of the current track.
///
/// M0 scaffolding note: consumed by `Engine::seek` callers starting in M1.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeekStatus {
    /// Jump to an absolute position counting from track start.
    Absolute(f64),
    /// Jump relative to the current position (positive → forward).
    Relative(f64),
}

/// Health/identity snapshot returned by [`Engine::probe`].
///
/// Shaped to match the `engine_probe:` IPC payload contract exactly.
#[derive(Debug, Clone, Serialize)]
pub struct EngineStatus {
    /// Backend family: `"mpv"` or `"null"`.
    pub engine: String,
    /// `true` iff the backend's runtime library/handle is loaded and usable
    /// in this process right now. The null backend reports `false` because
    /// it has no audio capability by design.
    pub loaded: bool,
    /// Human-readable version if the backend can provide one.
    pub version: Option<String>,
    /// Explanation when the backend is degraded (e.g. libmpv missing).
    pub error: Option<String>,
}

/// Errors surfaced by the playback engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "snake_case")]
pub enum EngineError {
    /// Accepted by the facade but not implemented yet (M0 scaffold).
    NotImplemented,
    /// Capable backend expected, but degraded in this process (e.g. missing
    /// runtime library): operation is impossible right now.
    ///
    /// M0 scaffolding note: backends still return `NotImplemented` for
    /// playback; `Unavailable` starts flowing once M1 wires real ops.
    #[allow(dead_code)]
    Unavailable(String),
    /// Backend-specific failure message (isolate library/OS noise here).
    ///
    /// Constructed by the mpv backend (cargo feature `mpv`); kept in the
    /// shared vocabulary so the null/feature-off builds type-check against
    /// the same error contract.
    #[allow(dead_code)]
    Backend(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::NotImplemented => write!(f, "not implemented yet (M0 scaffold)"),
            EngineError::Unavailable(message) => write!(f, "engine unavailable: {message}"),
            EngineError::Backend(message) => write!(f, "engine backend error: {message}"),
        }
    }
}

impl std::error::Error for EngineError {}

/// The single playback seam for the whole app.
///
/// Implementations bookkeep their own interior mutability so callers only
/// ever need `&self` (Tauri manages [`SharedEngine`] behind a mutex).
///
/// M0 note: `probe`/`name`/`version` are fully live; `play` … `enqueue`
/// return [`EngineError::NotImplemented`] until M1 wires the player loop.
/// M0 scaffolding note: only `probe`/`name`/`version` are consumed today;
/// playback methods are part of the stable contract but stay unconsumed
/// until M1 wires the player loop (hence the container-level allow).
#[allow(dead_code)]
pub trait Engine: Send + Sync {
    /// Stable backend identifier (`"mpv"`, `"null"`).
    fn name(&self) -> &'static str;

    /// One-shot health/identity report (backs the `engine_probe` command).
    fn probe(&self) -> EngineStatus;

    /// Human-readable version string if the backend can produce one.
    fn version(&self) -> Option<String>;

    /// Start playing the audio at `path`, replacing anything playing.
    fn play(&self, path: &Path) -> Result<(), EngineError>;

    /// Pause the current playback.
    fn pause(&self) -> Result<(), EngineError>;

    /// Resume the current playback after [`Engine::pause`].
    fn resume(&self) -> Result<(), EngineError>;

    /// Stop the current playback and discard its state.
    fn stop(&self) -> Result<(), EngineError>;

    /// Seek inside the current track.
    fn seek(&self, status: SeekStatus) -> Result<(), EngineError>;

    /// Append a track to the playback queue without starting it.
    fn enqueue(&self, track: TrackRef) -> Result<(), EngineError>;
}

/// Engine instance shared through Tauri's managed state.
pub type SharedEngine = Arc<Mutex<Box<dyn Engine>>>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Headless smoke: engine mount must always produce a usable engine and
    /// a coherent probe status — mpv where libmpv exists, null otherwise —
    /// and must never panic on an engine-less machine (the whole point of
    /// the dlopen/fallback design).
    #[test]
    fn mount_engine_falls_back_safely() {
        let engine = mount_engine();
        let status = engine.lock().expect("engine mutex").probe();

        assert!(
            matches!(status.engine.as_str(), "mpv" | "null"),
            "unexpected backend: {:?}",
            status.engine
        );

        match status.engine.as_str() {
            "mpv" => {
                assert!(
                    status.loaded,
                    "mpv selected but not loaded (this environment has libmpv installed)"
                );
                assert!(
                    status.version.is_some(),
                    "mpv probe must report a version string"
                );
            }
            "null" => {
                assert!(!status.loaded, "the null backend has no audio capability");
                assert!(status.error.is_none(), "a healthy fallback reports no error");
            }
            _ => unreachable!("guarded by the assertion above"),
        }
    }
}

/// Select the strongest engine available on this machine.
///
/// The mpv backend (feature `mpv`, default-on) is tried first; when its
/// runtime library cannot be found/loaded the null engine takes over and
/// the reason is logged. This must never panic — a machine without mpv is
/// a supported configuration, an engine-less one.
pub fn mount_engine() -> SharedEngine {
    #[cfg(feature = "mpv")]
    match mpv::MpvEngine::mount() {
        Ok(engine) => {
            log::info!(
                target: "engine",
                "mounted mpv backend ({})",
                engine.version().unwrap_or_else(|| "unknown version".into())
            );
            return Arc::new(Mutex::new(Box::new(engine)));
        }
        Err(reason) => {
            log::warn!(
                target: "engine",
                "libmpv unavailable, falling back to the null engine: {reason}"
            );
        }
    }

    #[cfg(not(feature = "mpv"))]
    log::info!(target: "engine", "mpv feature disabled, using the null engine");

    Arc::new(Mutex::new(Box::new(null::NullEngine::new())))
}
