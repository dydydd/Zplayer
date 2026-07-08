# Zplayer feature roadmap design

Date: 2026-07-08

## Context

Zplayer is a Tauri + React desktop player for Emby/Jellyfin backed by mpv. The app already supports server login, home shelves, resume items, library browsing, search, item detail pages, series episodes, favorite/played actions, media source selection, audio/subtitle selection, mpv controls, playback progress reporting, settings, in-memory metadata cache, and playback diagnostics.

The next feature set should deepen those existing flows instead of adding a separate product surface.

## Goal

Add all requested improvement areas in a phased roadmap:

1. Daily playback convenience.
2. Media library management.
3. Professional playback controls.
4. More mature product UI.

Each phase must be independently useful and shippable.

## Non-goals

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

## Phase 1: Daily playback convenience

### Features

- Add previous/next episode actions in the player for series playback.
- Add automatic next episode after normal playback completion. Default it on for series playback and add one playback setting to disable it.
- Add a lightweight recently played shelf on the home page.
- Improve search with recent search terms and clearer empty/loading states.
- Add keyboard coverage for player commands that already exist: play/pause, seek, volume, mute, fullscreen, previous/next episode, audio, subtitle, and speed.

### Architecture

- Extend the player view state with optional series context: current episode id, ordered episode ids, and current episode index.
- Reuse existing `DetailView` episode data when playback starts from a show detail page.
- For playback launched from home/search/library where episode context is not loaded, fall back to playing only the selected item.
- Store recent plays locally in the existing Tauri store JSON. Keep only the latest 50 item ids per server.
- Add home shelf hydration by resolving recent item ids through existing item loading APIs.
- Extend the playback-stopped event with a `completed` flag. Compute it from the final playback position and ignore explicit app-issued stop commands.

### Data flow

1. User starts playback.
2. React passes optional episode context to `PlayerView`.
3. mpv playback stop event returns to `App`.
4. If playback completed and autoplay is enabled for series context, `App` launches the next episode.
5. Store updates recent play ids after playback starts or stops.

### Error handling

- If next episode launch fails, show the existing error banner and return to the previous view.
- If recent item resolution fails, omit that item from the shelf.
- If episode context is missing, hide previous/next controls instead of guessing.

### Tests

- Unit test recent-play dedupe and max-size trimming.
- Unit test next-episode selection at first, middle, and last episode.
- Manual check: play an episode, stop, finish, and verify resume/progress reporting still works.

## Phase 2: Media library management

### Features

- Add a Favorites view using the existing favorite item metadata and server APIs.
- Add library filters for watched/unwatched/favorite.
- Add "mark watched/unwatched" and "favorite/unfavorite" actions to poster cards where the item data is already present.
- Add genre and actor navigation from detail pages.
- Add collection support when the server exposes box sets/collections.

### Architecture

- Extend `LibraryInput` with optional filters: played state, favorite state, genre, person, and collection id.
- Reuse `LibraryView` for filtered result pages instead of creating separate grid components.
- Use one route shape for filtered grids: `library` plus optional query fields.
- Keep favorite and watched mutations routed through existing `mark_favorite` and `mark_played` commands.
- Invalidate home, library, and detail caches after user-data mutations.

### Data flow

1. User opens a filtered view from a shelf, filter menu, genre chip, actor card, or collection.
2. React calls `load_library` with filter options.
3. Rust maps options to Emby/Jellyfin query parameters.
4. Results render through existing poster grid and pagination.

### Error handling

- Unsupported filters return an empty state with the existing error banner.
- Mutation failures keep the UI unchanged and show the server error.
- Partial metadata gaps hide the unavailable filter entry.

### Tests

- Unit test filter normalization in Rust.
- Unit test cache invalidation after favorite/played mutation.
- Manual check on both movie and series libraries.

## Phase 3: Professional playback controls

### Features

- Add visible playback speed controls and speed state.
- Remember preferred audio and subtitle stream by server, series/item, and language when possible.
- Add subtitle delay and audio delay controls through mpv commands.
- Add a simple external subtitle path/URL field in the player menu.
- Add default media source preference: highest quality, lowest bitrate, or remember last selected.

### Architecture

- Extend mpv progress reporting with speed if mpv exposes it in the current Lua progress script.
- Add normalized mpv commands for speed set, subtitle delay, audio delay, and external subtitle loading.
- Persist playback preferences in the existing store JSON under server id.
- Apply preferences when building `play_item` input; fall back to current default stream logic.
- Start external subtitles with a typed path/URL field. Add a native file picker only if Tauri dialog support is later accepted as a dependency.

### Data flow

1. User selects audio, subtitle, speed, or source.
2. Player sends mpv command immediately.
3. App stores the chosen preference after command success.
4. Future playback resolves preferences before calling `play_item`.

### Error handling

- Invalid stream indexes are rejected by existing command normalization.
- Missing preferred streams fall back to default streams.
- Invalid external subtitle paths/URLs show an error and keep current subtitles unchanged.

### Tests

- Unit test preference matching by exact item, series, then language.
- Unit test command normalization for new mpv commands.
- Manual check speed, subtitle delay, audio delay, and external subtitle loading in mpv.

## Phase 4: Mature product UI

### Features

- Reorder and polish home shelves for favorites, recently played, and continue-watching after those data sources exist.
- Replace inactive placeholder buttons in the hero actions with real actions or remove them.
- Improve detail pages with more visible runtime, rating, genres, studios, and progress.
- Improve empty and loading states for home, library, detail, and search.
- Tighten small-window responsive behavior for poster grids, player controls, and modals.

### Architecture

- Reuse existing `MediaShelf`, `Poster`, `ScrollableStage`, and `LibraryView`.
- Add only small props where existing components already have the needed item data.
- Keep layout changes in existing CSS files by feature area.
- Prefer hiding unavailable controls over disabled decorative controls.

### Data flow

- Home loads first-screen data as it does today.
- Secondary shelves continue loading through `load_home_more`.
- UI-only improvements must not add new blocking requests to app startup.

### Error handling

- Home remains usable if secondary shelves fail.
- Empty states should describe what happened and offer one available action.
- Player controls must keep stable dimensions while state changes.

### Tests

- Build check with `npm run build`.
- Manual responsive checks at narrow, default, and wide desktop widths.
- Manual check that player controls do not overlap at small widths.

## Implementation order

1. Phase 1: next episode, autoplay, recent plays, search states, keyboard coverage.
2. Phase 2: favorites view, watched/favorite filters, poster actions, genre/actor navigation.
3. Phase 3: speed UI, persisted audio/subtitle/source preferences, delay controls, external subtitle field.
4. Phase 4: home shelves and UI cleanup.

## Acceptance criteria

- A user can finish one episode and continue to the next without returning to the detail page.
- A user can find favorites, unwatched items, and watched items from library views.
- A user can adjust speed, audio, subtitles, and delays without leaving playback.
- A user sees useful home shelves and no dead placeholder controls.
- Existing playback start, stop, progress reporting, and resume reporting still work.
- The app builds with existing package scripts.

## Scope guard

If any phase grows beyond these boundaries, split it into a separate spec before implementation. The first split candidate is Phase 3 external subtitle file picking, because native file dialogs may require an added Tauri plugin.
