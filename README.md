<div align="center">
  <h1>Zplayer</h1>
  <p><strong>A polished desktop client for Emby and Jellyfin, powered by mpv.</strong></p>
  <p>
    <img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2.x-24C8DB?style=for-the-badge&logo=tauri&logoColor=white">
    <img alt="React 19" src="https://img.shields.io/badge/React-19-61DAFB?style=for-the-badge&logo=react&logoColor=06121F">
    <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-6.x-3178C6?style=for-the-badge&logo=typescript&logoColor=white">
    <img alt="Rust" src="https://img.shields.io/badge/Rust-backend-B7410E?style=for-the-badge&logo=rust&logoColor=white">
    <img alt="mpv playback" src="https://img.shields.io/badge/mpv-playback-691F69?style=for-the-badge">
  </p>
  <p>
    <img src="docs/img/home.png" alt="Zplayer home screen" width="920">
  </p>
</div>

## Overview

Zplayer brings an Emby or Jellyfin media library into a native desktop app. It focuses on fast library browsing, cinematic media pages, and reliable mpv playback with server progress reporting.

<table>
  <tr>
    <td width="33%"><strong>Library first</strong><br><sub>Home shelves, favorites, continue watching, library filters, search, and server switching.</sub></td>
    <td width="33%"><strong>Playback first</strong><br><sub>mpv playback with resume, progress sync, audio/subtitle selection, speed, volume, and next episode controls.</sub></td>
    <td width="33%"><strong>Desktop first</strong><br><sub>Tauri shell, Rust backend, configurable mpv path, proxy behavior, cache settings, themes, and diagnostics.</sub></td>
  </tr>
</table>

## Screenshots

<table>
  <tr>
    <td width="50%">
      <img src="docs/img/detail.png" alt="Zplayer media detail screen">
      <br>
      <strong>Rich media details</strong>
      <br>
      <sub>Artwork, seasons, episodes, people, related titles, and source metadata in one place.</sub>
    </td>
    <td width="50%">
      <img src="docs/img/media.png" alt="Zplayer playback screen">
      <br>
      <strong>Focused playback</strong>
      <br>
      <sub>Clean controls for seeking, audio, subtitles, volume, speed, and episode flow.</sub>
    </td>
  </tr>
</table>

## Highlights

| Area | What Zplayer provides |
| --- | --- |
| Servers | Add, test, switch, and manage Emby or Jellyfin connections. |
| Home | Recommendations, latest media, resume rows, recent plays, favorites, and library shelves. |
| Libraries | Type filters, sorting, played/unplayed state, favorites, genres, and poster density options. |
| Details | Seasons, episodes, cast and crew, artwork, similar titles, and media source inspection. |
| Playback | mpv-backed streaming, resume support, progress reporting, subtitle and audio controls, and autoplay next episode. |
| Settings | Language, theme, cache, diagnostics, seek steps, default volume, subtitles, proxy behavior, and custom mpv path. |

## Quick Start

### Prerequisites

- Node.js and npm
- Rust and the Tauri toolchain
- mpv for playback

### Run locally

```bash
npm install
npm run tauri -- dev
```

### Build

```bash
npm run tauri -- build
```

## Runtime Notes

| Platform | mpv requirement |
| --- | --- |
| Windows | Packaged builds include mpv. |
| Linux | Install with `sudo apt install mpv` on Debian/Ubuntu, or configure a custom path in settings. |
| macOS | Install with `brew install mpv`, or configure a custom path in settings. |

## Tech Stack

Zplayer uses Tauri for the desktop shell, React and TypeScript for the interface, Rust for native integration and server communication, and mpv for media playback.
