# Zplayer

Zplayer is a desktop media client for Emby and Jellyfin. It is built with Tauri, React, TypeScript, and Rust, with mpv handling playback.

## Screenshots

### Home

![Zplayer home view](docs/img/home.png)

### Media detail

![Zplayer media detail view](docs/img/detail.png)

### Playback

![Zplayer playback view](docs/img/media.png)

## Features

- Add, test, switch, and manage Emby or Jellyfin servers.
- Browse home recommendations, latest media, continue-watching rows, favorites, and libraries.
- Open rich media detail pages with seasons, episodes, people, artwork, similar titles, and available sources.
- Search media and browse libraries with type, sort, played, favorite, and genre filters.
- Play through mpv with progress reporting, resume support, audio and subtitle selection, speed controls, volume controls, and next-episode playback.
- Configure language, theme, poster density, metadata caching, diagnostics, seek steps, subtitles, volume, autoplay, proxy behavior, and custom mpv paths.

## Development

### Prerequisites

- Node.js and npm
- Rust and the Tauri toolchain
- mpv for playback

Packaged Windows builds include mpv. Linux and macOS builds use the system mpv: install it with `sudo apt install mpv` on Debian/Ubuntu or `brew install mpv` on macOS, or set a custom mpv path in settings.

### Commands

```bash
npm install
npm run dev
npm run tauri -- dev
npm run tauri -- build
```
