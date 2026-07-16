# libmpv runtime files

Zplayer loads libmpv at runtime instead of spawning the mpv executable. Platform runtime files are stored here for Tauri packaging:

| Platform | Folder | Primary library file |
| --- | --- | --- |
| Windows x64 | `libmpv/windows-x86_64` | `libmpv-2.dll` |
| macOS universal | `libmpv/macos-universal` | `Mpv.xcframework/macos-arm64_x86_64/Mpv.framework/Versions/A/Mpv` |
| Linux x64 | `libmpv/linux-x86_64` | `libmpv.so.2` |

Windows and macOS resources come from media-kit prebuilt libmpv releases. Linux x64 currently uses Ubuntu's `libmpv2` amd64 package as the primary `libmpv.so.2`.

Linux release builds run `libmpv/bundle-linux-runtime.sh` after installing the distro `libmpv2` package. The script copies `libmpv.so.2`'s non-core shared library dependencies into `libmpv/linux-x86_64`, sets `$ORIGIN` RPATH with `patchelf` when available, and the app preloads sibling `.so` files before loading libmpv. A source checkout that only contains `libmpv.so.2` can still require matching system libraries until that script has populated the runtime directory.

Zplayer also accepts a custom libmpv file or directory through the existing playback path setting.
