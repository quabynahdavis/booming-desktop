//! Minimal *dynamic* bindings to the libmpv client API.
//!
//! ## Why dlopen instead of the `libmpv2` crate binding
//!
//! The maintained Rust bindings (`libmpv2`, and its older `libmpv`
//! ancestor) always emit a link-time dependency — their `*-sys` build
//! scripts unconditionally print `cargo:rustc-link-lib=mpv` and none of the
//! current releases exposes the historic `dlopen` feature. This project must
//! compile (and *link*) on machines without libmpv installed, failing only at
//! runtime — so the backend resolves the handful of functions it needs from
//! the shared library at runtime instead.
//!
//! This is deliberately a stepping stone, not a replacement for `libmpv2`:
//! once target machines have libmpv installed (see `docs/en/bootstrap/`),
//! playback can graduate to the full safe binding without touching anything
//! above [`MpvSymbols`] — `MpvEngine` is the only consumer.
//!
//! ## Lifetime model
//!
//! The loaded library is intentionally leaked: the symbols outlive any
//! `MpvEngine` instance, repeated mounts reuse one resolution, and mpv is
//! never mapped out during a player process' lifetime.

use std::ffi::{c_char, c_int, c_uint, c_void};
use std::sync::OnceLock;

use libloading::Library;

use super::error::MpvBackendError;

/// Opaque mpv client handle (`mpv_handle*`) as used by the C API.
pub type MpvHandle = *mut c_void;

// ---- C signatures of the subset the backend needs ------------------------

type MpvCreateFn = unsafe extern "C" fn() -> MpvHandle;
type MpvInitializeFn = unsafe extern "C" fn(MpvHandle) -> c_int;
type MpvSetOptionStringFn = unsafe extern "C" fn(MpvHandle, *const c_char, *const c_char) -> c_int;
type MpvSetPropertyStringFn = unsafe extern "C" fn(MpvHandle, *const c_char, *const c_char) -> c_int;
type MpvGetPropertyStringFn = unsafe extern "C" fn(MpvHandle, *const c_char) -> *mut c_char;
type MpvFreeFn = unsafe extern "C" fn(*mut c_void);
type MpvTerminateDestroyFn = unsafe extern "C" fn(MpvHandle);
type MpvClientApiVersionFn = unsafe extern "C" fn() -> c_uint;

/// Copyable table of resolved ABI entry points.
///
/// Function pointers, not closures/state: copying this table around is free
/// and the table auto-propagates `Send`/`Sync` (plain fn pointers).
#[derive(Clone, Copy)]
pub struct MpvSymbols {
    /// `mpv_create` — allocate an (uninitialized) client handle.
    pub create: MpvCreateFn,
    /// `mpv_initialize` — initialize the previously created handle.
    pub initialize: MpvInitializeFn,
    /// `mpv_set_option_string` — set options pre-initialization.
    pub set_option_string: MpvSetOptionStringFn,
    /// `mpv_set_property_string` — tweak properties post-initialization.
    ///
    /// M0 ABI completeness: resolved eagerly with the rest of the table
    /// even though the backend only probes today; M1's playback/event code
    /// needs it immediately.
    #[allow(dead_code)]
    pub set_property_string: MpvSetPropertyStringFn,
    /// `mpv_get_property_string` — read a property, `NULL` on error; the
    /// returned allocation must be released with [`MpvSymbols::free`].
    pub get_property_string: MpvGetPropertyStringFn,
    /// `mpv_free` — release strings returned by the client API.
    pub free: MpvFreeFn,
    /// `mpv_terminate_destroy` — destroy the handle and free resources.
    pub terminate_destroy: MpvTerminateDestroyFn,
    /// `mpv_client_api_version` — no handle needed.
    pub client_api_version: MpvClientApiVersionFn,
}

/// Environment variable that points at a specific libmpv file to try first
/// (ahead of the well-known sonames below).
pub const MPV_LIBRARY_ENV: &str = "BOOMING_MPV_LIBRARY";

#[cfg(target_os = "windows")]
const MPV_LIB_NAMES: &[&str] = &["mpv-2.dll", "mpv.dll", "libmpv-2.dll", "libmpv.dll"];
#[cfg(target_os = "macos")]
const MPV_LIB_NAMES: &[&str] = &["libmpv.2.dylib", "libmpv.dylib", "libmpv.1.dylib"];
#[cfg(all(unix, not(target_os = "macos"), not(target_os = "windows")))]
const MPV_LIB_NAMES: &[&str] = &["libmpv.so.2", "libmpv.so", "libmpv.so.1"];

/// One-shot resolution table for the process. The sticky `Result` (success
/// or the discovery error) is memoized so fallbacks and retries don't keep
/// reopening the library.
static SYMBOLS: OnceLock<Result<MpvSymbols, MpvBackendError>> = OnceLock::new();

/// Resolve the mpv client API subset from whatever libmpv is available.
///
/// Experience the sticky error mode: the returned `Err` borrows the memoized
/// discovery failure, safe to log repeatedly.
pub fn symbols() -> Result<MpvSymbols, &'static MpvBackendError> {
    match SYMBOLS.get_or_init(discover) {
        Ok(symbols) => Ok(*symbols),
        Err(err) => Err(err),
    }
}

fn discover() -> Result<MpvSymbols, MpvBackendError> {
    let library = load_library()?;

    // Leak on purpose (see module docs): the library must outlive the raw
    // function pointers handed out below, and a player process keeps mpv
    // mapped for its entire lifetime anyway.
    let library: &'static Library = Box::leak(Box::new(library));

    // SAFETY: every name is the documented, ABI-stable mpv client API
    // entry point paired with the exact type alias above. The class of
    // mistakes allowed here (wrong signature) is prevented by keeping the
    // signatures next to their `extern`-declaration peers in one table.
    unsafe {
        Ok(MpvSymbols {
            create: resolve(library, b"mpv_create\0")?,
            initialize: resolve(library, b"mpv_initialize\0")?,
            set_option_string: resolve(library, b"mpv_set_option_string\0")?,
            set_property_string: resolve(library, b"mpv_set_property_string\0")?,
            get_property_string: resolve(library, b"mpv_get_property_string\0")?,
            free: resolve(library, b"mpv_free\0")?,
            terminate_destroy: resolve(library, b"mpv_terminate_destroy\0")?,
            client_api_version: resolve(library, b"mpv_client_api_version\0")?,
        })
    }
}

/// Resolve one symbol, borrowing nothing after the copy (function pointers
/// to a leaked library remain valid for the process).
///
/// # Safety
/// Caller must guarantee that `T` is the correct signature for
/// `symbol_name`. The pointer handed back by the dynamic loader is copied
/// out of a `libloading::Symbol` bound to a `'static` library.
unsafe fn resolve<T: Copy>(
    library: &'static Library,
    symbol_name: &[u8],
) -> Result<T, MpvBackendError> {
    let symbol = library
        .get::<T>(symbol_name)
        .map_err(|err| MpvBackendError::SymbolMissing {
            symbol: match std::str::from_utf8(symbol_name) {
                Ok(name) => name.trim_end_matches('\0').to_string(),
                Err(_) => String::from("<non-utf8 symbol name>"),
            },
            detail: err.to_string(),
        })?;
    Ok(*symbol)
}

fn load_library() -> Result<Library, MpvBackendError> {
    let mut candidates: Vec<String> = Vec::new();

    // 1) Explicit override first...
    if let Ok(override_path) = std::env::var(MPV_LIBRARY_ENV) {
        if !override_path.is_empty() {
            candidates.push(override_path);
        }
    }
    // 2) ...then well-known sonames of the current OS.
    candidates.extend(MPV_LIB_NAMES.iter().map(|name| (*name).to_string()));

    let mut last_detail = "no candidate".to_string();

    for candidate in &candidates {
        // SAFETY: dlopen of a shared lib image; libloading handles
        // OS-specific loading semantics (RTLD_NOW-equivalent by default).
        match unsafe { Library::new(candidate) } {
            Ok(library) => return Ok(library),
            Err(err) => last_detail = err.to_string(),
        }
    }

    Err(MpvBackendError::LibraryNotFound {
        candidates,
        detail: last_detail,
    })
}
