# Zplayer phase 1: daily playback convenience

Date: 2026-07-08

## Goal

Make daily watching smoother without changing the playback engine or adding new dependencies.

## Features

- Add previous/next episode actions in the player for series playback.
- Add automatic next episode after normal playback completion. Default it on for series playback and add one playback setting to disable it.
- Add a lightweight recently played shelf on the home page.
- Improve search with recent search terms and clearer empty/loading states.
- Add keyboard coverage for player commands that already exist: play/pause, seek, volume, mute, fullscreen, previous/next episode, audio, subtitle, and speed.

## Architecture

- Extend the player view state with optional series context: current episode id, ordered episode ids, and current episode index.
- Reuse existing `DetailView` episode data when playback starts from a show detail page.
- For playback launched from home/search/library where episode context is not loaded, fall back to playing only the selected item.
- Store recent plays locally in the existing Tauri store JSON. Keep only the latest 50 item ids per server.
- Add home shelf hydration by resolving recent item ids through existing item loading APIs.
- Extend the playback-stopped event with a `completed` flag. Compute it from the final playback position and ignore explicit app-issued stop commands.

## Data flow

1. User starts playback.
2. React passes optional episode context to `PlayerView`.
3. mpv playback stop event returns to `App`.
4. If playback completed and autoplay is enabled for series context, `App` launches the next episode.
5. Store updates recent play ids after playback starts or stops.

## Error handling

- If next episode launch fails, show the existing error banner and return to the previous view.
- If recent item resolution fails, omit that item from the shelf.
- If episode context is missing, hide previous/next controls instead of guessing.

## Tests

- Unit test recent-play dedupe and max-size trimming.
- Unit test next-episode selection at first, middle, and last episode.
- Manual check: play an episode, stop, finish, and verify resume/progress reporting still works.

## Acceptance criteria

- A user can finish one episode and continue to the next without returning to the detail page.
- A user can manually jump to the previous or next episode from the player when episode context exists.
- A user can disable autoplay from settings.
- Recently played items appear on home without blocking first-screen loading.
- Existing progress and resume reporting still work.
