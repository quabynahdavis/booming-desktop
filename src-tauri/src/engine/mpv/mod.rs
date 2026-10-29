//! MpvEngine — audio backend driven by libmpv, loaded at *runtime*.
//!
//! ## Mount contract
//!
//! 1. [`MpvEngine::mount`] resolves libmpv (`ffi::symbols` → dlopen). On
//!    missing/insufficient library it returns a sticky reason value; the
//!    engine facade then mounts `NullEngine` instead and logs the reason, so
//!    a machine without mpv degrades gracefully, never crashing.
//! 2. [`Engine::probe`] (M0) spins a throwaway handle with `vo = null`,
//!    initializes it and queries the `mpv-version` property. **No player
//!    loop is spawned yet** — that is M1.
//! 3. Playback controls are stubs returning `NotImplemented` until M1 wires
//!    the actual playback/event machinery.
//!
//! ## Not a liberation from the future crate binding
//!
//! Symbols are resolved raw (see [`ffi`]) because current `libmpv2`/`libmpv`
//! releases cannot link without an installed libmpv at *build* time. Once
//! hosts can guarantee libmpv, swap [`ffi`] for the full safe binding; this
//! module stays the boundary (`MpvEngine` does not leak raw pointers to the
//! rest of the app).

pub mod error;

mod ffi;

use std::ffi::CStr;
use std::path::Path;

use super::{Engine, EngineError, EngineStatus, SeekStatus, TrackRef};
pub use error::MpvBackendError;

/// libmpv-backed engine (cargo feature `mpv`).
pub struct MpvEngine {
    symbols: ffi::MpvSymbols,
}

impl MpvEngine {
    /// Attempt to mount the mpv backend.
    ///
    /// `Err` borrows a process-lifetime memoized discovery failure — cheap
    /// to log/repeat, never a panic.
    pub fn mount() -> Result<Self, &'static MpvBackendError> {
        let symbols = ffi::symbols()?;
        Ok(Self { symbols })
    }

    /// Numeric mpv client API version (no handle needed), e.g. `0x030000`.
    pub fn client_api_version(&self) -> Option<u32> {
        Some(unsafe { (self.symbols.client_api_version)() })
    }

    /// Run `body` against a pre-initialized, video-less mpv handle.
    ///
    /// The handle is created with `vo = null` (headless probe semantics, M0)
    /// and destroyed on scope exit; the guard guarantees terminate_destroy
    /// runs even when `body` short-circuits with an error.
    fn with_minimal_handle<T>(
        &self,
        body: impl FnOnce(ffi::MpvHandle, ffi::MpvSymbols) -> Result<T, MpvBackendError>,
    ) -> Result<T, MpvBackendError> {
        // SAFETY: no handle yet; the call simply allocates the client.
        let handle = unsafe { (self.symbols.create)() };
        if handle.is_null() {
            return Err(MpvBackendError::Internal {
                what: "mpv_create returned NULL",
            });
        }
        let guard = MpvHandleGuard {
            handle,
            symbols: self.symbols,
        };

        // Options can only be set *before* mpv_initialize; vo=null keeps the
        // probe headless (audio-only, no video/GL output). Errors here do
        // not leak handles (guard drops them).
        let rc = unsafe {
            (guard.symbols.set_option_string)(guard.handle, c"vo".as_ptr(), c"null".as_ptr())
        };
        if rc < 0 {
            return Err(MpvBackendError::Api {
                what: "mpv_set_option_string(vo=null)",
                code: rc,
            });
        }

        body(guard.handle, guard.symbols)
    }

    /// Query the human `mpv-version` property via a throwaway handle.
    fn query_version(&self) -> Result<String, MpvBackendError> {
        self.with_minimal_handle(|handle, symbols| {
            // SAFETY: handle is a live, not-yet-initialized mpv client.
            let rc = unsafe { (symbols.initialize)(handle) };
            if rc < 0 {
                return Err(MpvBackendError::Api {
                    what: "mpv_initialize",
                    code: rc,
                });
            }

            let raw = unsafe { (symbols.get_property_string)(handle, c"mpv-version".as_ptr()) };
            if raw.is_null() {
                return Err(MpvBackendError::Internal {
                    what: "mpv-version property unavailable",
                });
            }

            // SAFETY: mpv_get_property_string hands back a NUL-terminated
            // C string owned by us, to be released with mpv_free.
            let version = unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned();
            unsafe { (symbols.free)(raw.cast()) };

            Ok(version)
        })
    }
}

/// RAII for one `mpv_handle`: `mpv_terminate_destroy` on drop.
struct MpvHandleGuard {
    handle: ffi::MpvHandle,
    symbols: ffi::MpvSymbols,
}

impl Drop for MpvHandleGuard {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: the handle exists exactly for the lifetime of the
            // guard; destroyed exactly once.
            unsafe { (self.symbols.terminate_destroy)(self.handle) };
        }
    }
}

impl Engine for MpvEngine {
    fn name(&self) -> &'static str {
        "mpv"
    }

    fn version(&self) -> Option<String> {
        // Cheap, no-handle identity: numeric client API version. The human
        // "mpv x.y.z" string needs a live handle and belongs to `probe`.
        let raw = self.client_api_version()?;
        Some(format!("libmpv client API {raw:#010x}"))
    }

    fn probe(&self) -> EngineStatus {
        // M0 probe: create/initialize a temporary vo=null handle and read
        // `mpv-version`. No event loop, no playback — just ABI reality.
        match self.query_version() {
            Ok(version) => EngineStatus {
                engine: self.name().to_string(),
                loaded: true,
                version: Some(version),
                error: None,
            },
            Err(err) => EngineStatus {
                engine: self.name().to_string(),
                loaded: false,
                version: None,
                error: Some(err.to_string()),
            },
        }
    }

    fn play(&self, path: &Path) -> Result<(), EngineError> {
        let _ = path;
        Err(EngineError::NotImplemented)
    }

    fn pause(&self) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn resume(&self) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn stop(&self) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn seek(&self, status: SeekStatus) -> Result<(), EngineError> {
        let _ = status;
        Err(EngineError::NotImplemented)
    }

    fn enqueue(&self, track: TrackRef) -> Result<(), EngineError> {
        let _ = track;
        Err(EngineError::NotImplemented)
    }
}

impl From<MpvBackendError> for EngineError {
    fn from(err: MpvBackendError) -> Self {
        EngineError::Backend(err.to_string())
    }
}
