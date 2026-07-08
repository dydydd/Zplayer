# Zplayer phase 3: professional playback controls

Date: 2026-07-08

## Goal

Expose the mpv controls advanced users expect while keeping mpv as the only playback engine.

## Features

- Add visible playback speed controls and speed state.
- Remember preferred audio and subtitle stream by server, series/item, and language when possible.
- Add subtitle delay and audio delay controls through mpv commands.
- Add a simple external subtitle path/URL field in the player menu.
- Add default media source preference: highest quality, lowest bitrate, or remember last selected.

## Architecture

- Extend mpv progress reporting with speed if mpv exposes it in the current Lua progress script.
- Add normalized mpv commands for speed set, subtitle delay, audio delay, and external subtitle loading.
- Persist playback preferences in the existing store JSON under server id.
- Apply preferences when building `play_item` input; fall back to current default stream logic.
- Start external subtitles with a typed path/URL field. Add a native file picker only if Tauri dialog support is later accepted as a dependency.

## Data flow

1. User selects audio, subtitle, speed, source, or delay.
2. Player sends mpv command immediately.
3. App stores the chosen preference after command success where the choice should persist.
4. Future playback resolves preferences before calling `play_item`.

## Error handling

- Invalid stream indexes are rejected by existing command normalization.
- Missing preferred streams fall back to default streams.
- Invalid external subtitle paths or URLs show an error and keep current subtitles unchanged.

## Tests

- Unit test preference matching by exact item, series, then language.
- Unit test command normalization for new mpv commands.
- Manual check speed, subtitle delay, audio delay, and external subtitle loading in mpv.

## Acceptance criteria

- A user can adjust speed from visible player controls.
- A user can adjust subtitle delay and audio delay during playback.
- A user's stream and source preferences apply on later playback when matching streams exist.
- External subtitle loading works through a typed path or URL.
