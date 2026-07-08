# Zplayer phase 2: media library management

Date: 2026-07-08

## Goal

Make the existing library data easier to slice, manage, and revisit.

## Features

- Add a Favorites view using the existing favorite item metadata and server APIs.
- Add library filters for watched, unwatched, and favorite items.
- Add "mark watched/unwatched" and "favorite/unfavorite" actions to poster cards where the item data is already present.
- Add genre and actor navigation from detail pages.
- Add collection support when the server exposes box sets or collections.

## Architecture

- Extend `LibraryInput` with optional filters: played state, favorite state, genre, person, and collection id.
- Reuse `LibraryView` for filtered result pages instead of creating separate grid components.
- Use one route shape for filtered grids: `library` plus optional query fields.
- Keep favorite and watched mutations routed through existing `mark_favorite` and `mark_played` commands.
- Invalidate home, library, and detail caches after user-data mutations.

## Data flow

1. User opens a filtered view from a shelf, filter menu, genre chip, actor card, or collection.
2. React calls `load_library` with filter options.
3. Rust maps options to Emby/Jellyfin query parameters.
4. Results render through existing poster grid and pagination.

## Error handling

- Unsupported filters return an empty state with the existing error banner.
- Mutation failures keep the UI unchanged and show the server error.
- Partial metadata gaps hide the unavailable filter entry.

## Tests

- Unit test filter normalization in Rust.
- Unit test cache invalidation after favorite/played mutation.
- Manual check on both movie and series libraries.

## Acceptance criteria

- A user can open favorites from the app chrome or home surface.
- A user can filter a library by watched, unwatched, and favorite.
- Poster-level favorite and watched actions update server state.
- Detail-page genres and actors open filtered grids when the server provides enough metadata.
