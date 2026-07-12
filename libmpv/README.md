# libmpv runtime files

Zplayer loads libmpv at runtime instead of spawning the mpv executable. Platform runtime files are stored here for Tauri packaging:

| Platform | Folder | Primary library file |
| --- | --- | --- |
| Windows x64 | `libmpv/windows-x86_64` | `libmpv-2.dll` |
| macOS universal | `libmpv/macos-universal` | `Mpv.xcframework/macos-arm64_x86_64/Mpv.framework/Versions/A/Mpv` |
| Linux x64 | `libmpv/linux-x86_64` | `libmpv.so.2` |

Windows and macOS resources come from media-kit prebuilt libmpv releases. Linux x64 currently uses Ubuntu's `libmpv2` amd64 package; system libraries still need to satisfy libmpv's transitive dependencies on the target distro. Zplayer also accepts a custom libmpv file or directory through the existing playback path setting.
