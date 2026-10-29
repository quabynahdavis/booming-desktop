# Booming Desktop — Documentation (English)

Offline music player for **Windows, macOS and Linux**. Rust backend
(Tauri v2) + vanilla TypeScript frontend (Vite, no framework). Audio engine:
libmpv, loaded at **runtime only** (dlopen) so the whole stack compiles and
links without mpv installed.

## File index

| File | Scope |
| --- | --- |
| [`bootstrap/01-setup.md`](bootstrap/01-setup.md) | Toolchain, repo layout, M0 verification matrix, libmpv runtime install |
| [`CHANGELOG.md`](CHANGELOG.md) | Project and docs revision history (this language) |

## Domain map

- **bootstrap** — environment and toolchain facts, how to verify builds,
  host package-manager notes, runtime library installation.
- *(planned)* **architecture** — engine facade, IPC surface, UI state.
- *(planned)* **engine** — libmpv backend details, discovery rules,
  M1 playback wiring.

## Key project rules

- Engine selection must **never crash** on a machine without libmpv: it
  degrades to the null engine and logs the reason.
- Nothing links against libmpv at build time; mpv is resolved via dlopen
  (env override: `BOOMING_MPV_LIBRARY`).
- All documentation lives under `/docs/{lang}/…`; commit subject format is
  Conventional Commits, atomic per related-file group.
