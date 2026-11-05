//! Lyrics support (P7).
//!
//! Submodules:
//! - [`lrc`] — LRC document parser (line + word timings, metadata, offsets).
//! - [`sidecar`] — per-track `.lrc` files living next to the audio files.

pub mod lrc;
pub mod sidecar;
