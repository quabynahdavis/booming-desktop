# Booming Desktop

Offline music player for **Windows, macOS and Linux**. Rust backend (Tauri v2)
with a vanilla TypeScript frontend — no frontend framework.

## Status: Beta (v0.1.0)

Features shipped:
- Library scan (M3U import), playback queue, gapless audio, ReplayGain
- 15-band equalizer + AutoEq profile import
- Sync lyrics (LRCLIB provider) with word-level karaoke
- Last.fm / ListenBrainz scrobbling
- Material 3 UI (vanilla TS + `@material/web`)
- First-run onboarding, settings, themes

## Installation

### Linux
```sh
# Download latest release from GitHub Releases
wget https://github.com/davisville/BoomingDesktop/releases/latest/download/BoomingDesktop_0.1.0_amd64.deb
sudo apt install ./BoomingDesktop_0.1.0_amd64.deb
```

### Windows
```powershell
# Download .msi from GitHub Releases
Start-BitsTransfer -Source "https://github.com/davisville/BoomingDesktop/releases/latest/download/BoomingDesktop_0.1.0_x64.msi"
msiexec /i BoomingDesktop_0.1.0_x64.msi
```

### macOS
```sh
# Download .dmg from GitHub Releases
brew install --cask booming-desktop
# or manually download from GitHub Releases
```

> **Note:** The app bundles `libmpv` for audio playback. If the bundled version causes issues, install mpv system-wide and restart the app.

## Development

- Product brief, milestones and environment setup: [`docs/en/`](docs/en/).
- Frontend: Vite + TypeScript (`bun install`, then `bun run dev` / `bun run build`).
- Backend: Tauri v2 crate under [`src-tauri/`](src-tauri/) (`cargo check` there
  to avoid linking system libraries during quick verification).

### Headless-friendly verification

```sh
bun install && bun run build     # frontend (writes dist/, needed by the Rust macro)
cargo check --manifest-path src-tauri/Cargo.toml                 # with mpv feature
cargo check --manifest-path src-tauri/Cargo.toml --no-default-features  # without
cargo test --lib
```

The mpv audio backend loads `libmpv` at **runtime only** (dlopen); nothing
links against it at build time, so this repo compiles on machines without
mpv installed and simply falls back to the null engine, with a clear log.

## Contributions

Open the app, use it, and report issues at <https://github.com/davisville/BoomingDesktop>.

### Build for release

1. Install mpv on your build machine (`sudo apt install mpv libmpv-dev` / `brew install mpv`).
2. Run `script/fetch-mpv-libs.sh linux` (or `windows`, `macos`) to bundle the native library.
3. `cargo tauri build` produces platform-specific packages.
