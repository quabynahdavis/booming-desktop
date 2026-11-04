# Packaging & bundling

Per-OS bundling strategy for Booming Desktop releases, plus how the mpv
runtime library is shipped with each artifact.

## Tauri bundle configuration

`src-tauri/tauri.conf.json`:

```json
{
  "bundle": {
    "active": true,
    "targets": ["deb", "appimage"],
    "category": "Audio",
    "shortDescription": "Offline music player",
    "icon": ["icons/32x32.png", "icons/icon.png"],
    "resources": [
      "resources/",
      "resources/mpv/libmpv.so.2",
      "resources/mpv/libmpv-2.dll",
      "resources/mpv/libmpv.dylib"
    ]
  }
}
```

- **Active:** true — bundle packaging runs during release builds.
- **Targets:** currently Linux-only (`deb` + `appimage`).
- **Category:** `Audio`.
- **Icon:** `icons/32x32.png` + `icons/icon.png`.
- **Resources:** the entire `resources/` directory plus the mpv library
  placeholders.

## Per-OS mpv library strategy

The engine discovers libmpv at runtime via dlopen. At release time the
library must be present at one of the discovery candidates:

```text
resources/mpv/libmpv.so.2   # Linux
resources/mpv/libmpv-2.dll # Windows
resources/mpv/libmpv.dylib # macOS
```

### Linux — deb / appimage

- Source: official mpv devfiles.
  - `https://mpv.io/downloads/mpv-devfiles/libmpv.so.2`
- Place at `resources/mpv/libmpv.so.2`.
- The app then loads it via `BOOMING_MPV_LIBRARY` override or bundled path.
- For deb: Tauri packages the resource with the binary; for appimage the
  resource is archived in `*.AppImage`.
- Note: `libmpv.so.2` has a soname that must match the loader expectations.

### Windows — msi / nsis (future), currently deb/appimage

- Source: mpv release zip, `v0.39.0`.
  - Download: `https://github.com/mpv-player/mpv/releases/download/v0.39.0/mpv-x86_64-vulkan-20240119-git-9643e06.zip`
  - Extract `libmpv-2.dll` from `mpv-x86_64-vulkan-20240119-git-9643e06/` into `resources/mpv/libmpv-2.dll`.
- The engine's Windows candidate list includes `mpv-2.dll`, `mpv.dll`,
  `libmpv-2.dll`, `libmpv.dll`.

### macOS — dmg

- Source: Homebrew or official mpv release DMG.
- Preferred via Homebrew:
  - `brew install libmpv` (formula) or `brew install --cask mpv` (DMG).
  - Copy the framework library into `resources/mpv/`:
    - `$(brew --prefix)/lib/libmpv.dylib`
    - or from mpv DMG: `/Library/Frameworks/mpv.framework/Versions/*/Resources/libmpv.dylib`
- Destination: `resources/mpv/libmpv.dylib`.

## `scripts/fetch-mpv-libs.sh`

Usage:

```sh
scripts/fetch-mpv-libs.sh [linux|windows|macos]
```

Default (no arg / invalid arg): no-op (placeholders remain).

```sh
#!/usr/bin/env bash
# Fetch real mpv engine runtime libraries into resources/mpv/ at release time.

case "${1:-}" in
  linux)
    curl -L -o "$TARGET/libmpv.so.2" https://mpv.io/downloads/mpv-devfiles/libmpv.so.2
    ;;
  windows)
    ZIP="$TARGET/mpv-x86_64-vulkan-20240119-git-9643e06.zip"
    curl -L -o "$ZIP" https://github.com/mpv-player/mpv/releases/download/v0.39.0/mpv-x86_64-vulkan-20240119-git-9643e06.zip
    unzip -o "$ZIP" "mpv-x86_64-vulkan-20240119-git-9643e06/libmpv-2.dll" -d "$TARGET"
    mv "$TARGET/mpv-x86_64-vulkan-20240119-git-9643e06/libmpv-2.dll" "$TARGET/libmpv-2.dll"
    ;;
  macos)
    # brew install libmpv / brew install --cask mpv
    # cp "$(brew --prefix)/lib/libmpv.dylib" "$TARGET/libmpv.dylib"
    # or from mpv release DMG: /Library/Frameworks/mpv.framework/Versions/*/Resources/libmpv.dylib
    ;;
  *)
    echo "usage: scripts/fetch-mpv-libs.sh [linux|windows|macos]" >&2
    exit 2
    ;;
esac
```

## Packaging step order

1. `bun install && bun run build` — build frontend into `dist/`.
2. `cargo build` — build Rust backend (needs no libmpv at build time).
3. Run `scripts/fetch-mpv-libs.sh <os>` — replaces placeholder stubs with real binaries.
4. `tauri build` — packages deb / appimage / (future windows msi/nsis, macos dmg).

## Placeholder stubs

Before the fetch step, the placeholders ship with the repo:

```text
resources/mpv/libmpv.so.2    (Linux)
resources/mpv/libmpv-2.dll   (Windows)
resources/mpv/libmpv.dylib   (macOS)
```

The engine loads these when present; when they are placeholders the engine
still runs but falls back to the null engine.

## Documentation reference

See [`docs/en/bootstrap/01-setup.md`](../../bootstrap/01-setup.md) for the
full dev environment, runtime libmpv installation and verification matrix,
and [`docs/en/playback/01-engine.md`](../../playback/01-engine.md) for the
dlopen discovery order.
