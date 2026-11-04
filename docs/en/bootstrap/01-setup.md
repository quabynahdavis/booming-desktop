# Bootstrap & environment setup (M0)

Everything needed to build, verify and (later) run the M0 scaffold.

## 1. Toolchain on the dev host

| Tool | Version |
| --- | --- |
| rustc / cargo | 1.96.1 |
| node | 24.14.0 |
| npm / bun | 11.9.0 / **1.4.4** (preferred) |
| vite | 8.3.5 |
| typescript | 5.9.3 |

Frontend deps: `@tauri-apps/api` 2.12.4, `@tauri-apps/cli` 2.12.4.
Rust backend: `tauri` 2.12.5, `tauri-build` 2.7.4, `libloading` 0.9.0 (dlopen
only), `serde`/`serde_json`, `log` + `env_logger`.

## 2. Repo layout

```
booming-desktop/
├── index.html, src/           # vanilla TypeScript frontend (Vite)
├── package.json               # bun workspace scripts (see §4)
├── vite.config.ts             # dev port 1420 (matches tauri.conf.json)
├── docs/en/                   # this documentation tree
└── src-tauri/                 # Tauri v2 Rust app
    ├── tauri.conf.json        # Linux-first: bundle targets deb + appimage
    ├── capabilities/          # Tauri v2 permission manifest
    ├── src/
    │   ├── main.rs / lib.rs   # shell + startup wiring
    │   ├── commands.rs        # engine_probe IPC
    │   └── engine/
    │       ├── mod.rs         # Engine trait facade + mount_engine()
    │       ├── null.rs        # NullEngine fallback
    │       └── mpv/           # MpvEngine (feature "mpv", default)
    │           ├── error.rs   # backend error taxonomy
    │           ├── ffi.rs     # dlopen libmpv + typed ABI table
    │           └── mod.rs     # probe: vo=null handle + mpv-version
    └── Cargo.toml             # features: ["mpv"] default-on
```

## 3. Engine selection, in one look

1. `engine::mount_engine()` runs **once at startup**.
2. With cargo feature `mpv` (default): `MpvEngine::mount()` →
   `dlopen` order is `BOOMING_MPV_LIBRARY` env override, then
   `libmpv.so.2`, `libmpv.so`, `libmpv.so.1` (Platform-specific tables exist
   for Windows `mpv-2.dll`… and macOS `libmpv.2.dylib`…).
3. On success: backend `mpv` is active.
4. On failure: `log::warn!` with the exact candidate list + loader detail,
   then `NullEngine` is active. **Never a crash.**
5. The probe IPC returns `{ engine: "mpv"|"null", loaded, version, error }`;
   for a live mpv probe we create a throwaway handle with `vo=null`, run
   `mpv_initialize`, fetch `mpv-version`, then destroy the handle. No player
   loop is spawned in M0.

## 4. Verification matrix (headless, exact commands)

```sh
# frontend
bun install
bun run build                # vite build → dist/ (required by the Rust macro)

# backend (compile, two feature states)
cargo check --manifest-path src-tauri/Cargo.toml                  # mpv ON
cargo check --manifest-path src-tauri/Cargo.toml --no-default-features

# backend (full link — the crucial proof that libmpv is NOT needed)
cargo build --manifest-path src-tauri/Cargo.toml

# headless regression (mount + probe logic; passes on machines with and
# without libmpv — mpv path when present, null path when absent)
cargo test --lib --manifest-path src-tauri/Cargo.toml
```

All of the above pass on the dev host (see `docs/en/CHANGELOG.md` §["Verified"]
with concrete timestamps).

Optional interactive run (`tauri dev`-style, needs a display):
`bun run build && (cd src-tauri && cargo run)` — this opens the desktop window;
the "Check engine" button is the M0 smoke test.

## 5. Runtime libmpv installation (for the user)

The scaffold compiles/links **without** mpv; installing the library only
upgrades the engine selection from `null` to `mpv`.

- Debian/Ubuntu hosts:
  ```sh
  sudo apt-get install -y libmpv-dev mpv
  ```
- Arch hosts (the current dev box is Arch; there is no separate `-dev`
  split — the headers ship in the runtime package):
  ```sh
  sudo pacman -S --needed mpv
  ```

To point the engine at a specific library file:
```sh
export BOOMING_MPV_LIBRARY=/path/to/your/libmpv.so.2
```

After installing, re-run the headless test — `engine::tests…` should now
mount `mpv` and print an mpv version like `mpv 0.40.x` from the probe.

## 6. Deferrals (owned by later milestones)

- Real playback loop: `play/pause/resume/stop/seek/enqueue` return
  `NotImplemented` on all backends in M0 (the trait shape is frozen).
- Library scanning, metadata extraction, UI library views.
- Real bundling artifacts (deb/appimage packaging run, release icons).
- Windows/macOS-specific build steps (nothing OS-specific is written today
  outside the standard Tauri boilerplate and the dlopen filename tables).
- Migration of `engine/mpv/ffi.rs` to the full `libmpv2` safe binding once
  host machines link libmpv (localized swap behind `MpvSymbols`).

## 7. Bundling the mpv engine

At release time the Tauri bundle (`src-tauri/tauri.conf.json` →
`bundle.resources`) points at these placeholders:

```
resources/mpv/libmpv.so.2    (Linux)
resources/mpv/libmpv-2.dll   (Windows)
resources/mpv/libmpv.dylib   (macOS)
```

Generate the real binaries before the bundle step:

```sh
scripts/fetch-mpv-libs.sh linux   # Linux: curl from mpv devfiles
scripts/fetch-mpv-libs.sh windows # Windows: curl from mpv release zip
scripts/fetch-mpv-libs.sh macos   # macOS: copy libmpv from Homebrew / mpv DMG
```

Linux line:

```
curl -L -o resources/mpv/libmpv.so.2 https://mpv.io/downloads/mpv-devfiles/libmpv.so.2
```

Windows line:

```
curl -L -o resources/mpv/libmpv-2.dll https://github.com/mpv-player/mpv/releases/download/v0.39.0/mpv-x86_64-vulkan-20240119-git-9643e06.zip
```

(extract `libmpv-2.dll` from the zip onto the target name above).

macOS sources (choose one; the script documents the same choices):

* Homebrew formula `libmpv` (`brew install libmpv`) — copy
  `"$(brew --prefix)/lib/libmpv.dylib"` into `resources/mpv/`.
* Homebrew cask (`brew install --cask mpv`) — copy the DMG's
  `libmpv.dylib` (or equivalent framework resource) into `resources/mpv/`.
* Official mpv release DMG — mount the DMG and copy
  `/Library/Frameworks/mpv.framework/Versions/*/Resources/libmpv.dylib`.

The fetch step is a pre-release step; the placeholders ship with the repo
and remain safe until a release build is cut. Release builds should also pin
a concrete mpv version instead of the "latest devfiles" URL above.

