//! Backend-private errors for the mpv backend.
//!
//! Kept separate from the shared [`EngineError`](crate::engine::EngineError)
//! vocabulary: this enum carries exactly what can go wrong *inside* the
//! libmpv dance (discovery/loading/ABI/handle), which `MpvEngine::mount`'
//! callers log before falling back to the null engine.

use std::fmt;

/// Everything that can fail inside the mpv backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MpvBackendError {
    /// libmpv is not present on this machine. Carries the sonames that were
    /// searched plus the loader's detail, so the user knows what to install.
    LibraryNotFound {
        candidates: Vec<String>,
        detail: String,
    },
    /// The library loaded, but an expected ABI symbol was missing (e.g. a
    /// too-old libmpv). Recorded as data, not a panic.
    SymbolMissing {
        symbol: String,
        detail: String,
    },
    /// An mpv client-API call returned a negative error code.
    Api {
        what: &'static str,
        code: i32,
    },
    /// Contract violation of the mpv client API (NULL handle, missing
    /// property, invalid UTF-8, …).
    Internal {
        what: &'static str,
    },
}

impl fmt::Display for MpvBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MpvBackendError::LibraryNotFound {
                candidates,
                detail,
            } => {
                write!(f, "libmpv not found (searched ")?;
                for (i, candidate) in candidates.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{candidate}")?;
                }
                write!(f, "; {detail})")
            }
            MpvBackendError::SymbolMissing { symbol, detail } => {
                write!(f, "libmpv missing required symbol `{symbol}`: {detail}")
            }
            MpvBackendError::Api { what, code } => {
                write!(f, "mpv api call `{what}` failed with code {code}")
            }
            MpvBackendError::Internal { what } => write!(f, "mpv backend misuse: {what}"),
        }
    }
}

impl std::error::Error for MpvBackendError {}
