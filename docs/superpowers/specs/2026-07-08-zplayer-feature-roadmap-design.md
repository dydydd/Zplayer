# Zplayer feature roadmap index

Date: 2026-07-08

## Context

Zplayer is a Tauri + React desktop player for Emby/Jellyfin backed by mpv. The app already supports server login, home shelves, resume items, library browsing, search, item detail pages, series episodes, favorite/played actions, media source selection, audio/subtitle selection, mpv controls, playback progress reporting, settings, in-memory metadata cache, and playback diagnostics.

The next work should deepen those existing flows instead of adding a separate product surface.

## Split specs

1. [Phase 1: Daily playback convenience](2026-07-08-zplayer-phase-1-daily-playback-design.md)
2. [Phase 2: Media library management](2026-07-08-zplayer-phase-2-library-management-design.md)
3. [Phase 3: Professional playback controls](2026-07-08-zplayer-phase-3-professional-playback-design.md)
4. [Phase 4: Mature product UI](2026-07-08-zplayer-phase-4-product-ui-design.md)

## Recommended order

Implement the specs in phase order. Phase 1 and Phase 2 create the data and navigation surfaces that Phase 4 polishes. Phase 3 is mostly independent after Phase 1, but it should still wait until basic playback flow changes settle.

## Shared non-goals

- No streaming server implementation.
- No new media database owned by Zplayer.
- No custom playback engine; mpv remains the player.
- No new dependency unless a feature cannot be done with Tauri, browser APIs, Rust stdlib, existing crates, or mpv.
- No cloud sync.

## Options considered

### Option A: One large release

Build every feature area in one branch. This gives one visible launch, but it mixes UI, server API, playback, and persistence changes. Regression risk is high.

### Option B: Phased release

Ship four small stages. Each stage reuses existing views, IPC patterns, store JSON, and mpv control commands. This is the recommended path because it keeps each diff reviewable and usable.

### Option C: Settings-first release

Expose many toggles before building full flows. It is fast, but it creates a settings-heavy app without clear user value.

Recommended: Option B.

## Cross-phase acceptance

- Existing playback start, stop, progress reporting, and resume reporting still work.
- The app builds with existing package scripts.
- Each phase remains useful when shipped without later phases.

## Scope guard

If any phase grows beyond its spec, split that phase again before implementation. The first split candidate is Phase 3 external subtitle file picking, because native file dialogs may require an added Tauri plugin.
