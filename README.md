# Zplayer

Zplayer is a native Rust desktop client for Emby and Jellyfin media servers, built with [GPUI Kit](https://crates.io/crates/gpui-kit). The interface follows the Windows 11 Fluent design language: layered neutral surfaces, a collapsible left navigation pane with an accent selection indicator, a compact page header, and a single brand-green accent that adapts to the system light and dark appearances.

## Status

Under active development. The current foundation includes:

- GPUI Kit window bootstrap with a Fluent-style application shell
- Light / dark / system-follow theming
- Emby/Jellyfin API layer
- UI-independent playback negotiation and session model
- Protocol-neutral media-source selection and URL normalization
- Playback mode, network options, and initial track selection

## Build

Requirements:

- Rust 1.92 or newer
- Windows: Visual Studio 2022 C++ Build Tools, Windows SDK, and CMake
- libmpv runtime

```bash
cargo check
cargo test
cargo run
```

## Layout

```text
src/
  api/         Emby/Jellyfin protocol clients
  models/      Serializable API and application models
  playback/    Playback requests, source selection, resolved sessions
  ui/          GPUI Kit views and application bootstrap
Cargo.toml
```

Icons are retained under `assets-icons/` for future packaging.
