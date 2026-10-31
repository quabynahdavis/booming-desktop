//! NullEngine — always-available fallback backend.
//!
//! Accepted as the process engine whenever nothing stronger mounts. Every
//! command is a no-op returning [`EngineError::NotImplemented`] so the UI can
//! distinguish "scaffolded" from "attempted and failed".

use super::{Engine, EngineError, EngineStatus, PlaybackState};

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
            loaded: false,
            version: None,
            error: None,
        }
    }

    fn load(&self, _url: &str) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn play(&self) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn pause(&self) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn seek(&self, _pos_secs: f64) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn get_position(&self) -> Result<f64, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn get_duration(&self) -> Result<f64, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn get_state(&self) -> Result<PlaybackState, EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn next(&self) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn previous(&self) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }

    fn set_replay_gain_db(&self, _gain: f64) -> Result<(), EngineError> {
        Err(EngineError::NotImplemented)
    }
}
