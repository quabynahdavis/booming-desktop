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

use std::ffi::{CStr, CString};
use std::sync::Mutex;

use super::{Engine, EngineError, EngineStatus, PlaybackState};
pub use error::MpvBackendError;

/// libmpv-backed engine (cargo feature `mpv`).
pub struct MpvEngine {
    symbols: ffi::MpvSymbols,
    handle: Mutex<Option<ffi::MpvHandle>>,
}

impl MpvEngine {
    /// Attempt to mount the mpv backend.
    ///
    /// `Err` borrows a process-lifetime memoized discovery failure — cheap
    /// to log/repeat, never a panic.
    pub fn mount() -> Result<Self, &'static MpvBackendError> {
        let symbols = ffi::symbols()?;
        Ok(Self {
            symbols,
            handle: Mutex::new(None),
        })
    }

    /// Numeric mpv client API version (no handle needed), e.g. `0x030000`.
    pub fn client_api_version(&self) -> Option<u32> {
        Some(unsafe { (self.symbols.client_api_version)() })
    }

    fn ensure_handle(&self) -> Result<ffi::MpvHandle, MpvBackendError> {
        let mut guard = self.handle.lock().expect("handle mutex poisoned");
        if let Some(h) = guard.as_mut() {
            return Ok(*h);
        }
        // create new handle
        let handle = unsafe { (self.symbols.create)() };
        if handle.is_null() {
            return Err(MpvBackendError::Internal { what: "mpv_create returned NULL" });
        }
        // set vo=null before init
        let rc = unsafe {
            (self.symbols.set_option_string)(handle, std::ffi::CStr::from_bytes_with_nul(b"vo\0").unwrap().as_ptr(), std::ffi::CStr::from_bytes_with_nul(b"null\0").unwrap().as_ptr())
        };
        if rc < 0 {
            return Err(MpvBackendError::Api { what: "mpv_set_option_string(vo=null)", code: rc });
        }
        let rc = unsafe { (self.symbols.initialize)(handle) };
        if rc < 0 {
            return Err(MpvBackendError::Api { what: "mpv_initialize", code: rc });
        }
        *guard = Some(handle);
        Ok(handle)
    }

    fn mpv_command(&self, args: &[&str]) -> Result<(), MpvBackendError> {
        let handle = self.ensure_handle()?;
        // build C string array
        let cstrings: Vec<CString> = args.iter().map(|s| CString::new(*s).map_err(|_| MpvBackendError::Internal { what: "CString NUL" })).collect::<Result<_, _>>()?;
        let ptrs: Vec<*const std::ffi::c_char> = cstrings.iter().map(|s| s.as_ptr()).collect();
        let mut ptrs_with_null = ptrs;
        ptrs_with_null.push(std::ptr::null());
        let rc = unsafe { (self.symbols.command)(handle, ptrs_with_null.as_ptr()) };
        if rc < 0 {
            return Err(MpvBackendError::Api { what: "mpv_command", code: rc });
        }
        Ok(())
    }

    fn get_property_string(&self, name: &str) -> Result<String, MpvBackendError> {
        let handle = self.ensure_handle()?;
        let cname = CString::new(name).map_err(|_| MpvBackendError::Internal { what: "CString NUL" })?;
        let raw = unsafe { (self.symbols.get_property_string)(handle, cname.as_ptr()) };
        if raw.is_null() {
            return Err(MpvBackendError::Internal { what: "property not found" });
        }
        let s = unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned();
        unsafe { (self.symbols.free)(raw as *mut std::ffi::c_void) };
        Ok(s)
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
        let raw = self.client_api_version()?;
        Some(format!("libmpv client API {raw:#010x}"))
    }

    fn probe(&self) -> EngineStatus {
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

    fn load(&self, url: &str) -> Result<(), EngineError> {
        self.mpv_command(&["loadfile", url])?;
        Ok(())
    }

    fn play(&self) -> Result<(), EngineError> {
        self.mpv_command(&["play"])?;
        Ok(())
    }

    fn pause(&self) -> Result<(), EngineError> {
        self.mpv_command(&["pause"])?;
        Ok(())
    }

    fn seek(&self, pos_secs: f64) -> Result<(), EngineError> {
        let pos = pos_secs.to_string();
        self.mpv_command(&["seek", &pos, "absolute"])?;
        Ok(())
    }

    fn get_position(&self) -> Result<f64, EngineError> {
        let s = self.get_property_string("time-pos")?;
        s.parse::<f64>().map_err(|_| EngineError::Backend("invalid time-pos".into()))
    }

    fn get_duration(&self) -> Result<f64, EngineError> {
        let s = self.get_property_string("duration")?;
        s.parse::<f64>().map_err(|_| EngineError::Backend("invalid duration".into()))
    }

    fn get_state(&self) -> Result<PlaybackState, EngineError> {
        let pause_str = self.get_property_string("pause").unwrap_or_else(|_| "1".to_string());
        let paused = pause_str == "1" || pause_str.eq_ignore_ascii_case("true");
        if paused {
            Ok(PlaybackState::Paused)
        } else {
            // check if playing
            let pos = self.get_position().unwrap_or(0.0);
            if pos > 0.0 {
                Ok(PlaybackState::Playing)
            } else {
                Ok(PlaybackState::Stopped)
            }
        }
    }

    fn next(&self) -> Result<(), EngineError> {
        self.mpv_command(&["playlist-next"])?;
        Ok(())
    }

    fn previous(&self) -> Result<(), EngineError> {
        self.mpv_command(&["playlist-prev"])?;
        Ok(())
    }

    fn set_replay_gain_db(&self, gain: f64) -> Result<(), EngineError> {
        // Use set_property_string for replaygain-track-gain
        let handle = self.ensure_handle()?;
        let name = CString::new("replaygain-track-gain").unwrap();
        let val = CString::new(gain.to_string()).unwrap();
        let rc = unsafe { (self.symbols.set_property_string)(handle, name.as_ptr(), val.as_ptr()) };
        if rc < 0 {
            return Err(EngineError::Backend("set replay gain failed".into()));
        }
        Ok(())
    }
}

impl From<MpvBackendError> for EngineError {
    fn from(err: MpvBackendError) -> Self {
        EngineError::Backend(err.to_string())
    }
}
