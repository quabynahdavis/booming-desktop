#!/usr/bin/env bash
# Fetch real mpv engine runtime libraries into resources/mpv/ at release time.
#
# The Tauri bundle (`src-tauri/tauri.conf.json` -> bundle.resources) points at
# these placeholders; the fetch script replaces them with the real binaries.
#
# Usage: scripts/fetch-mpv-libs.sh [linux|windows|macos]
# Default (no arg / invalid arg): no-op (placeholders remain).

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="$DIR/resources/mpv"

case "${1:-}" in
  linux)
    # Official mpv devfiles (Debian/Ubuntu style shared lib with soname libmpv.so.2)
    curl -L -o "$TARGET/libmpv.so.2" https://mpv.io/downloads/mpv-devfiles/libmpv.so.2
    ;;
  windows)
    # Official mpv Windows release zip (x86_64 vulkan build); extract libmpv-2.dll
    ZIP="$TARGET/mpv-x86_64-vulkan-20240119-git-9643e06.zip"
    curl -L -o "$ZIP" https://github.com/mpv-player/mpv/releases/download/v0.39.0/mpv-x86_64-vulkan-20240119-git-9643e06.zip
    unzip -o "$ZIP" "mpv-x86_64-vulkan-20240119-git-9643e06/libmpv-2.dll" -d "$TARGET"
    mv "$TARGET/mpv-x86_64-vulkan-20240119-git-9643e06/libmpv-2.dll" "$TARGET/libmpv-2.dll"
    ;;
  macos)
    # Real binary comes from either the mpv release DMG or Homebrew libmpv.
    # Homebrew path (preferred on macOS hosts):
    #   brew install --cask mpv   # cask copies the DMG app; libmpv lives in the framework
    #   brew install libmpv       # formula provides the shared library
    # After installing, copy the framework library into resources/mpv:
    #   cp "$(brew --prefix)/lib/libmpv.dylib" "$TARGET/libmpv.dylib"
    # or from mpv release DMG via `open` + cp of /Library/Frameworks/mpv.framework/Versions/*/Resources/libmpv.dylib
    ;;
  *)
    echo "usage: scripts/fetch-mpv-libs.sh [linux|windows|macos]" >&2
    exit 2
    ;;
esac
