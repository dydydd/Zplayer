# Zplayer phase 4: mature product UI

Date: 2026-07-08

## Goal

Polish the existing screens after the playback and library data surfaces exist.

## Features

- Reorder and polish home shelves for favorites, recently played, and continue-watching after those data sources exist.
- Replace inactive hero buttons with real actions or remove them.
- Improve detail pages with more visible runtime, rating, genres, studios, and progress.
- Improve empty and loading states for home, library, detail, and search.
- Tighten small-window responsive behavior for poster grids, player controls, and modals.

## Architecture

- Reuse existing `MediaShelf`, `Poster`, `ScrollableStage`, and `LibraryView`.
- Add only small props where existing components already have the needed item data.
- Keep layout changes in existing CSS files by feature area.
- Prefer hiding unavailable controls over disabled decorative controls.

## Data flow

- Home loads first-screen data as it does today.
- Secondary shelves continue loading through `load_home_more`.
- UI-only improvements must not add new blocking requests to app startup.

## Error handling

- Home remains usable if secondary shelves fail.
- Empty states should describe what happened and offer one available action.
- Player controls must keep stable dimensions while state changes.

## Tests

- Build check with `npm run build`.
- Manual responsive checks at narrow, default, and wide desktop widths.
- Manual check that player controls do not overlap at small widths.

## Acceptance criteria

- Home exposes the most useful shelves without duplicate or dead controls.
- Detail pages show richer metadata without hiding the primary play action.
- Empty and loading states are useful across home, library, detail, and search.
- Player controls remain readable and non-overlapping in small windows.
