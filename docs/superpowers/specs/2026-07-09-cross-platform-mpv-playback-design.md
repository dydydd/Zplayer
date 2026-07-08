# Cross-platform mpv playback

Date: 2026-07-09

## Goal

Fix mpv playback on Linux and macOS while preserving the existing Windows embedded mpv experience.

## Current behavior

Playback reaches `src-tauri/src/mpv.rs::launch`, then always calls `add_embed_args`. The Windows implementation adds `--wid` and window flags. The non-Windows implementation returns an error, so Linux and macOS fail before mpv is spawned.

Default mpv discovery also only searches for `mpv.exe`, which matches the bundled Windows runtime but not Linux or macOS installations.

## Approach

Use platform-specific mpv startup behavior in `src-tauri/src/mpv.rs`.

- Windows keeps the current bundled/custom `mpv.exe` discovery and embedded-window arguments.
- Linux and macOS keep custom path support, then fall back to launching `mpv` from `PATH`.
- Linux and macOS do not receive embedded-window arguments. mpv opens its own native window, while the existing Lua progress script, control file, playback watcher, and server progress reporting continue to run.

This is the smallest fix because the playback session lifecycle already works after process spawn and does not depend on Windows APIs.

## Data flow

1. Frontend calls the existing `play_item` IPC command.
2. Backend resolves the playable media URL and calls `mpv::launch`.
3. `mpv::launch` resolves the mpv executable for the current platform.
4. Windows adds embedded-window args. Linux/macOS skip embedding.
5. The existing mpv Lua script writes progress and reads control commands.
6. The existing watcher reports playback progress and stop events.

## Error handling

- If a configured custom mpv path is set but missing, keep returning `Configured mpv was not found: <path>`.
- If no default mpv executable can be found on Linux/macOS, return a clear install/configure message: `mpv was not found. Install mpv or set the mpv path in settings.`
- If mpv starts and exits immediately, keep the existing mpv log-tail diagnostic.

## Tests

- Add focused unit coverage for platform mpv executable naming and non-Windows embedding behavior.
- Run `cargo test --manifest-path src-tauri/Cargo.toml --lib mpv`.

## Non-goals

- Do not implement Linux/macOS mpv window embedding.
- Do not add a new dependency.
- Do not change the frontend player UI.
