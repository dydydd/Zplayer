# Linux Wayland UI smoothness

Date: 2026-07-09

## Goal

Reduce Zplayer UI stutter on Linux Wayland without forcing X11, disabling hardware acceleration, or degrading the visible app experience.

## Current behavior

The app creates one transparent, undecorated Tauri window from `tauri.conf.json`. This is required for the current Windows embedded mpv experience, where the webview becomes transparent while mpv is placed behind it.

On Linux, mpv playback is being designed as an external native mpv window. The Zplayer webview does not need system-level transparency to reveal video behind it. Keeping a transparent WebKitGTK/Tauri window on Wayland can add compositor work during scrolling, animation, and window movement.

## Approach

Create the main window manually and choose platform-specific transparency at startup.

- Set the configured main window to `create: false`.
- In `src-tauri/src/lib.rs`, create the main window in `setup` with `WebviewWindowBuilder::from_config`.
- On Windows, keep the current transparent, undecorated window behavior for embedded mpv.
- On Linux Wayland, create an opaque webview window with the same size, no decorations, and the same dark background color.
- On Linux X11, macOS, and other non-Wayland platforms, keep the existing configured window behavior.
- Keep all existing CSS visual styling. This changes the OS window composition mode, not the product UI design.

This keeps the user-facing experience the same while removing a likely unnecessary Wayland compositor cost.

## Detection

Treat the session as Wayland when either of these is true:

- `XDG_SESSION_TYPE=wayland`
- `WAYLAND_DISPLAY` is set

Do not override `GDK_BACKEND`. The app should respect the user's desktop session.

## Diagnostics

Add a small Linux diagnostics surface, exposed near the existing playback diagnostics or logs:

- `XDG_SESSION_TYPE`
- whether `WAYLAND_DISPLAY` is set
- `GDK_BACKEND`
- whether the app selected an opaque Linux window path

This helps confirm whether the adaptation is active without guessing from behavior.

## Data flow

1. Tauri starts.
2. `setup` reads the configured window entry.
3. Linux runtime detection decides whether Wayland is active.
4. `WebviewWindowBuilder::from_config` builds the main window.
5. If Linux Wayland is active, the builder sets transparent to false and applies the existing dark background color.
6. Frontend loads normally; no route, player, or CSS state changes are required.

## Error handling

- If no configured main window is found, return a startup error instead of creating an incomplete fallback window.
- If Wayland detection is ambiguous, keep the current configured behavior and show diagnostics.
- Do not default to `GDK_BACKEND=x11`; keep that as a manual support workaround only.

## Tests

- Unit test Wayland detection from environment-like key/value input.
- Unit test the platform decision function chooses opaque mode for Linux Wayland and transparent mode for Windows.
- Run Rust tests for the new helper module.
- Manual check on Linux Wayland:
  - home/library scrolling and window dragging show no visible stutter during common use;
  - visual styling still matches the current app;
  - diagnostics show the selected window path.
- Manual check on Windows:
  - embedded mpv still works;
  - `playing-embedded` transparency behavior remains intact.

## References

- Tauri `create: false` and manual window creation: https://v2.tauri.app/reference/config/#windowconfig
- WebKitGTK hardware acceleration policy: https://webkitgtk.org/reference/webkit2gtk/stable/property.Settings.hardware-acceleration-policy.html
- GTK runtime environment variables: https://docs.gtk.org/gtk3/running.html

## Non-goals

- Do not force X11/XWayland by default.
- Do not disable WebKitGTK hardware acceleration by default.
- Do not change mpv playback rendering.
- Do not redesign the frontend UI.
