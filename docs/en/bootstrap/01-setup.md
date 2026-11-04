# Bootstrap & environment setup (M0.5)

Everything needed to build, verify and (later) run the M0.5+ scaffold.
Key updates from M0: bun 1.4.4 as preferred toolchain, `@material/web@2.5.0`
adoption, new workspace structure, library/playback modules, packaging step order.

## 1. Toolchain on the dev host

| Tool | Version |
| --- | --- |
| rustc / cargo | 1.96.1 |
| bun | **1.4.4** (preferred) |
| node | 24.14.0 |
| npm / bun | 1.4.4 (preferred) |
| vite | 8.3.5 |
| typescript | 5.9.3 |

Frontend deps: `@tauri-apps/api` 2.12.4, `@tauri-apps/cli` 2.12.4,
`@material/web@2.5.0`.

Rust backend: `tauri` 2.12.5, `tauri-build` 2.7.4, `libloading` 0.9.0 (dlopen
only), `serde`/`serde_json`, `log` + `env_logger`.

### Per-component imports (Material 3)

All Material 3 web components are imported individually from `@material/web`.
Only the components actually used in the UI are imported. Component
constructors register themselves on the custom-elements registry on first use.

```ts
// Filled-tonal button — used in action bars
import { mdFilledTonalButton } from "@material/web/button/filled-tonal-button.js";
// Filled button
import { mdFilledButton } from "@material/web/button/filled-button.js";
// Outlined button
import { mdOutlinedButton } from "@material/web/button/outlined-button.js";
// Tonal button
import { mdTonalButton } from "@material/web/button/tonal-button.js";
// Card component
import { mdCard } from "@material/web/card/card-element.js";
// Slider component
import { mdSlider } from "@material/web/slider/slider-element.js";
// Progress bar (if needed separately)
import { mdOutlineText } from "@material/web/typography/outlined-text.js";
// M3 token typography
const { md-tonal-body-1, md-tonal-caption-1 } = await import(
  "@material/web/typography/tonal-text.js"
);
```

**Token typography** uses M3 naming: `md-filled-tonal-button`, `md-tonal-body-1`,
etc. The `styles.css` file defines CSS custom properties matching the M3 token
palette (`--accent-a: #7c3aed`, `--accent-b: #06b6d4`, `--ok: #34d399`,
etc.).

## 2. Repo layout

```
booming-desktop/
├ index.html, src/           # vanilla TypeScript frontend (Vite)
├ package.json               # bun workspace scripts (see §4)
├ vite.config.ts             # dev port 1420 (matches tauri.conf.json)
├ docs/en/                   # this documentation tree
├ src-tauri/                 # Tauri v2 Rust app
    ├── tauri.conf.json      # Linux-first: bundle targets deb + appimage
    ├── capabilities/        # Tauri v2 permission manifest
    ├── src/
    │   ├── main.rs / lib.rs   # shell + startup wiring
    │   ├── commands.rs        # engine_probe IPC + playback IPC
    │   ├── engine/          # Engine trait + NullEngine + MpvEngine
    │   │   ├── mod.rs
    │   │   ├── null.rs
    │   │   └── mpv/           # MpvEngine (feature "mpv", default)
    │   │       ├── error.rs
    │   │       ├── ffi.rs
    │   │       └── mod.rs
    │   ├── library/         # db, scan, playlist, cover_cache, settings
    │   │   ├── mod.rs
    │   │   ├── db.rs
    │   │   ├── scan/          # folder scan → lofty tags → SQLite
    │   │   │   ├── mod.rs
    │   │   │   ├── scanner.rs
    │   │   │   └── tags.rs
    │   │   ├── playlist.rs
    │   │   ├── cover_cache.rs
    │   │   └── settings.rs
    │   └── playback/        # PlaybackService + replaygain
    │       ├── mod.rs
    │       └── replaygain.rs
    └── Cargo.toml           # features: ["mpv"] default-on
```

### Verification matrix (headless, exact commands)

```sh
# frontend
bun install
bun run build                # vite build → dist/ (required by the Rust macro)

# backend (compile, two feature states)
cargo check --manifest-path src-tauri/Cargo.toml                   # mpv ON
cargo check --manifest-path src-tauri/Cargo.toml --no-default-features  # mpv OFF

# backend (full link — the crucial proof that libmpv is NOT needed)
cargo build --manifest-path src-tauri/Cargo.toml

# headless regression (mount + probe logic; passes on machines with and
# without libmpv — mpv path when present, null path when absent)
cargo test --lib --manifest-path src-tauri/Cargo.toml
```

All of the above pass on the dev host (see `docs/en/CHANGELOG.md` with concrete timestamps).

### Optional interactive run (`tauri dev`-style, needs a display):

`bun run build && (cd src-tauri && cargo run)` — this opens the desktop window;
the "Check engine" button is the M0.5 smoke test.

## 3. New workspace structure

M0.5 adds three Rust sub-modules under `src-tauri/src/`:

- **library** — metadata extraction, SQLite persistence, folder scanning, playlist management.
- **playback** — `PlaybackService` (Arc<Mutex>), gapless audio, ReplayGain,
  repeat/shuffle logic, ban list of last 3.
- **engine** — Engine trait façade, MpvEngine with runtime dlopen, locator with
  env override + bundled/system fallback.

These modules are always compiled; feature flags gate only the MpvEngine dlopen
code path.

## 4. Packaging step order

The Tauri bundle (`src-tauri/tauri.conf.json` → `bundle.resources`) contains
resources that are swapped at release time:

1. **M0/M0.5** — placeholder stubs shipped in the repo (`resources/mpv/`)
2. **M1+** — `scripts/fetch-mpv-libs.sh` replaces placeholders with real binaries:
   - `linux`: curl from mpv devfiles → `resources/mpv/libmpv.so.2`
   - `windows`: curl from mpv release zip (v0.39.0) → extract `libmpv-2.dll`
   - `macos`: copy from Homebrew `libmpv` or mpv DMG → `resources/mpv/libmpv.dylib`
3. After fetch, `tauri build` packages the resources into the OS-specific
   artifact (deb / appimage on Linux).

Per-OS bundling strategy is documented in `docs/en/packaging/01-bundle.md`.

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

After installing, re-run the headless test — `cargo test --lib --manifest-path src-tauri/Cargo.toml` should now mount `mpv` and print an mpv version like `mpv 0.40.x` from the probe.

## 6. Deferrals (owned by later milestones)

- Real playback loop: `play/pause/resume/stop/seek/enqueue` return
  `NotImplemented` on all backends in M0 (the trait shape is frozen).
- Full library scanning with progress events and UI integration.
- Real bundling artifacts (deb/appimage packaging run, release icons).
- Windows/macOS-specific build steps (nothing OS-specific is written today
  outside the standard Tauri boilerplate and the dlopen filename tables).
- Migration of `engine/mpv/ffi.rs` to the full `libmpv2` safe binding once
  host machines link libmpv (localized swap behind `MpvSymbols`).