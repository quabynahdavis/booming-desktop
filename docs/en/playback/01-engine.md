# Engine trait & MpvEngine

Facade: `Engine` trait (`src-tauri/src/engine/mod.rs`) — the single seam
the rest of the app talks to. Backends: [`NullEngine`] and
[`MpvEngine`] (feature `mpv`, default-on). The facade has **zero build-time
dependency** on libmpv; that library is loaded at *runtime* via dlopen
(`libloading`). See `MpvSymbols` resolution in
`src-tauri/src/engine/mpv/ffi.rs`.

## Engine trait methods

| Method | Signature | Notes |
|---|---|---|
| `name` | `fn name(&self) -> &'static str` | `"mpv"` or `"null"` |
| `probe` | `fn probe(&self) -> EngineStatus` | health snapshot for IPC |
| `version` | `fn version(&self) -> Option<String>` | client API version hex |
| `load` | `fn load(&self, url: &str) -> Result<(), EngineError>` | M1: load track into engine |
| `play` | `fn play(&self) -> Result<(), EngineError>` | M1: start/resume playback |
| `pause` | `fn pause(&self) -> Result<(), EngineError>` | M1 |
| `seek` | `fn seek(&self, pos_secs: f64) -> Result<(), EngineError>` | absolute position |
| `get_position` | `fn get_position(&self) -> Result<f64, EngineError>` | seconds |
| `get_duration` | `fn get_duration(&self) -> Result<f64, EngineError>` | seconds |
| `get_state` | `fn get_state(&self) -> Result<PlaybackState, EngineError>` | Playing/Paused/Stopped |
| `next` | `fn next(&self) -> Result<(), EngineError>` | M1 |
| `previous` | `fn previous(&self) -> Result<(), EngineError>` | M1 |
| `set_replay_gain_db` | `fn set_replay_gain_db(&self, gain: f64) -> Result<(), EngineError>` | M1 |
| `set_audio_filter` | `fn set_audio_filter(&self, filter: String) -> Result<(), EngineError>` | M1 |
| `set_gapless` | `fn set_gapless(&self, enabled: bool) -> Result<(), EngineError>` | M1 |

M0 scope: `probe`/`name`/`version` are fully live; `play` … `enqueue`
return [`EngineError::NotImplemented`] until M1.

## NullEngine

Always-available fallback. Every operation returns `EngineError::NotImplemented`.
Used when libmpv cannot be loaded. Never panics.

## MpvEngine: dlopen + runtime loading

`MpvEngine` is compiled only when cargo feature `mpv` is enabled (default). It
resolves the libmpv shared library **at runtime** via `libloading`. The
symbol resolution order is:

1. **`BOOMING_MPV_LIBRARY`** env var — path specified by the user (absolute or
   relative).
2. **Bundled resources** — `resources/mpv/libmpv.so.2` (Linux) / `libmpv-2.dll`
   (Windows) / `libmpv.dylib` (macOS). Tauri `bundle.resources` config.
3. **System sonames** — platform‑specific table (`libmpv.so.2`, `libmpv.so`,
   `libmpv.so.1` on Linux; `libmpv.2.dylib`, `libmpv.dylib` on macOS;
   `mpv-2.dll`, `mpv.dll` on Windows).

If all candidates fail, the engine facade silently falls back to
`NullEngine` and logs the diagnostic.

### `BOOMING_MPV_LIBRARY` path discovery order

```text
1. env: BOOMING_MPV_LIBRARY
2. tauri bundle.resources dir + OS-specific filename
3. system ld.so / dyld search path (well-known sonames)
```

If the environment variable `BOOMING_MPV_LIBRARY` is set (non-empty), the
locator attempts to `dlopen` that path first. If that succeeds, all
subsequent candidates are skipped.

If the env var is absent or empty, the locator falls back to the bundled
resources directory (compiled into the Tauri binary at release time via
`scripts/fetch-mpv-libs.sh`), then to system paths.

### Runtime options set at init (MpvEngine::ensure_handle)

These options are set **before** `mpv_initialize`:

| Option | Value | Rationale |
|---|---|---|
| `gapless-audio` | `yes` (then `weak` if `yes` fails) | gapless playback between tracks |
| `audio-buffer` | `0.03` | ~30 ms buffer; reduces pre‑buffer seams |
| `vo` | `null` | headless, audio-only (no video output) |
| `playlist-repeat` | `no` | default repeat state |
| `mpv-version` | queried via throwaway handle | probe payload |

### mpv option immutability quirk

`gapless-audio` and other mpv options can only be set **before**
`mpv_initialize`. After initialization, calling `mpv_set_option_string`
returns `rc < 0` with error `mpv_set_option_string: Invalid argument`. The
code knows this and sets all pre-init options in `ensure_handle()`; any
post-init attempts are best‑effort (silently logged, rc returned).

Specifically for `set_gapless`: the `MpvEngine::set_gapless()` method tries
`set_option_string(handle, "gapless-audio", enabled)` after init; if it
returns `rc<0`, a warning is logged and the function returns `Ok(())` — the
request is declined gracefully.

### `BOOMING_MPV_LIBRARY` environment variable

The user can force a specific library path:

```sh
export BOOMING_MPV_LIBRARY=/path/to/libmpv.so.2
```

This overrides candidates 2 and 3 in the discovery order. If the file does
not exist or cannot be loaded, the engine falls back to `NullEngine` with an
`error` payload in the probe response.

## ReplayGain in the engine

`playback::replaygain::ReplayGainManager` is the shared module that decides
how replay gain gets applied per backend.

The `MpvEngine` implements `set_replay_gain_db`:

```rust
fn set_replay_gain_db(&self, gain: f64) -> Result<(), EngineError> {
    let handle = self.ensure_handle()?;
    let name = CString::new("replaygain-track-gain").unwrap();
    let val = CString::new(gain.to_string()).unwrap();
    let rc = unsafe { (self.symbols.set_property_string)(handle, name.as_ptr(), val.as_ptr()) };
    if rc < 0 {
        return Err(EngineError::Backend("set replay gain failed".into()));
    }
    Ok(())
}
```

For `GainMode::Album`, the `PlaybackService` code routes to
`replaygain_album` tag, falling back to `replaygain_track`.

If the gain tag is absent/missing, `loudnorm` fallback is used:
`engine.set_replay_gain_db(0.0)` + `engine.set_audio_filter("loudnorm")`.

## NullEngine vs MpvEngine comparison

| Capability | NullEngine | MpvEngine |
|---|---|---|
| `probe` | `engine: "null", loaded: false` | `engine: "mpv", loaded: true` |
| `load` | `NotImplemented` | loads file into libmpv |
| `play` | `NotImplemented` | starts playback |
| `pause` | `NotImplemented` | pauses playback |
| `seek` | `NotImplemented` | seeks to position |
| `set_replay_gain_db` | `NotImplemented` | sets mpv replaygain property |
| `set_audio_filter` | `NotImplemented` | sets mpv `af` option |
| `set_gapless` | `NotImplemented` | tries mpv `gapless-audio` option |

Both implement the `Engine` trait; the app never crashes regardless of
which backend is active.