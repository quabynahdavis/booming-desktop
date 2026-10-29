//! NullEngine — always-available fallback backend.
//!
//! Accepted as the process engine whenever nothing stronger mounts. Every
//! command is a no-op returning [`EngineError::NotImplemented`] so the UI can
//! distinguish "scaffolded" from "attempted and failed".

use std::path::Path;

use super::{Engine, EngineError, EngineStatus, SeekStatus, TrackRef};

/// Backend that accepts every command and plays nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullEngine;

impl NullEngine {
    pub const fn new() -> Self {
        Self
    }
}

impl Engine for NullEngine {
    fn name(&self) -> &'static str {
        "null"
    }

    fn version(&self) -> Option<String> {
        None
    }

    fn probe(&self) -> EngineStatus {
        EngineStatus {
            engine: self.name().to_string(),
            // No audio capability by design, hence `loaded: false`.
            loaded: false,
            version: None,
            error: None,
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
