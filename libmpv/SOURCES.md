# libmpv Binary Sources

Downloaded on 2026-07-12 for Zplayer's runtime-loaded libmpv integration.

| Platform | Source | Archive SHA256 | Extracted runtime file | Runtime SHA256 |
| --- | --- | --- | --- | --- |
| Windows x64 | https://github.com/media-kit/libmpv-win32-video-build/releases/download/2023-09-24/mpv-dev-x86_64-20230924-git-652a1dd.7z | `dce982222d7a23e4a1c6f0fb6cc39f6e899a6714624b95ea49cff6558ee97572` | `libmpv/windows-x86_64/libmpv-2.dll` | `d5f0694b08c124e785d858d00082f3e3b158dd9138bfc48c0382bf1eb443a5fc` |
| macOS universal | https://github.com/media-kit/libmpv-darwin-build/releases/download/v0.6.0/libmpv-xcframeworks_v0.6.0_macos-universal-video-default.tar.gz | `84d2ad98e046e82c6dc34d8547d76c2afeaee89c0f53032773be8985c95536d6` | `libmpv/macos-universal/Mpv.xcframework/macos-arm64_x86_64/Mpv.framework/Versions/A/Mpv` | `146c00e1c7fec9e51114d74c41c530e72967e2a59a91147439cbd07d118c4ea5` |
| Linux x64 | https://mirrors.kernel.org/ubuntu/pool/universe/m/mpv/libmpv2_0.37.0-1ubuntu4_amd64.deb | `12ea9d67e291a90cea5b7497f3f9d196643bf2e85ad17cb683e9c7ad18981c00` | `libmpv/linux-x86_64/libmpv.so.2` | `f2aa472878adee5f44419b6cce90ccecc43bf81595ac16dbf223de190cdead0a` |

Notes:

- The Windows archive's upstream CMake metadata also publishes MD5 `a832ef24b3a6ff97cd2560b5b9d04cd8`.
- The macOS archive SHA256 matches the upstream media-kit Makefile value.
- The Linux runtime file is copied from Ubuntu's `libmpv2` amd64 package as `libmpv.so.2` because Windows extraction cannot preserve the Debian symlink.
