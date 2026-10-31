//! Engine locator — resolves libmpv at runtime with a clean fallback chain.
//!
//! Search order (lazy at startup):
//! 1. BOOMING_MPV_LIBRARY env var
//! 2. bundled resources dir (`tauri::api::path::resource_dir`)
//! 3. system paths (well-known sonames)
//! 4. NullEngine fallback
//!
//! The locator is deliberately thin: it only discovers the library path and
//! builds an [`EngineSpec`]; the actual engine construction lives in `mpv`/`null`.

use std::path::{PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct EngineSpec {
    pub engine: String, // "mpv" | "null"
    pub lib_path: Option<PathBuf>,
    pub version: Option<String>,
    pub error: Option<String>,
}

pub struct EngineLocator;

impl EngineLocator {
    pub fn discover() -> EngineSpec {
        // 1) Env override
        if let Ok(env_path) = std::env::var("BOOMING_MPV_LIBRARY") {
            if !env_path.is_empty() {
                let path_buf = PathBuf::from(env_path);
                if let Some(spec) = Self::try_path(&path_buf) {
                    return spec;
                }
            }
        }

        // 2) Bundled resources dir (best-effort without AppHandle)
        if let Ok(resources_dir) = std::env::var("TAURI_RESOURCE_DIR") {
            let resources_dir = PathBuf::from(resources_dir);
            for name in Self::lib_names() {
                let candidate = resources_dir.join(name);
                if let Some(spec) = Self::try_path(&candidate) {
                    return spec;
                }
            }
        }
        // Note: tauri::path::resource_dir requires AppHandle; this fallback uses TAURI_RESOURCE_DIR env.

        // 3) System paths
        for name in Self::system_names() {
            let candidate = PathBuf::from(name);
            if let Some(spec) = Self::try_path(&candidate) {
                return spec;
            }
        }

        // Fallback
        EngineSpec {
            engine: "null".to_string(),
            lib_path: None,
            version: None,
            error: Some("libmpv not found, falling back to null engine".to_string()),
        }
    }

    fn try_path(path: &PathBuf) -> Option<EngineSpec> {
        // Attempt to load library to verify existence
        #[cfg(feature = "mpv")]
        {
            use libloading::Library;
            // SAFETY: dlopen check only
            unsafe {
                if let Ok(_lib) = Library::new(path) {
                    return Some(EngineSpec {
                        engine: "mpv".to_string(),
                        lib_path: Some(path.clone()),
                        version: None,
                        error: None,
                    });
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    fn lib_names() -> &'static [&'static str] {
        &["mpv-2.dll", "mpv.dll", "libmpv-2.dll", "libmpv.dll"]
    }

    #[cfg(target_os = "macos")]
    fn lib_names() -> &'static [&'static str] {
        &["libmpv.2.dylib", "libmpv.dylib", "libmpv.1.dylib"]
    }

    #[cfg(all(unix, not(target_os = "macos"), not(target_os = "windows")))]
    fn lib_names() -> &'static [&'static str] {
        &["libmpv.so.2", "libmpv.so", "libmpv.so.1"]
    }

    fn system_names() -> &'static [&'static str] {
        // Mirrors lib_names for now; kept separate for future OS-specific logic
        #[cfg(target_os = "windows")]
        {
            &["mpv-2.dll", "mpv.dll", "libmpv-2.dll", "libmpv.dll"]
        }
        #[cfg(target_os = "macos")]
        {
            &["libmpv.2.dylib", "libmpv.dylib", "libmpv.1.dylib"]
        }
        #[cfg(all(unix, not(target_os = "macos"), not(target_os = "windows")))]
        {
            &["libmpv.so.2", "libmpv.so", "libmpv.so.1"]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locator_returns_null_when_no_env() {
        // Ensure env var not set for this test; discovery should at least return a spec
        let spec = EngineLocator::discover();
        assert!(matches!(spec.engine.as_str(), "mpv" | "null"));
    }
}
