# Zplayer Phase 3 Professional Playback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add visible speed, delay, external subtitle, and remembered stream/source preferences on top of the existing mpv player.

**Architecture:** Extend the current mpv text-command bridge instead of adding a second control channel. Persist preferences in the existing JSON store and resolve them in React before calling `play_item`.

**Tech Stack:** Tauri 2, React 19, TypeScript, Rust, mpv Lua command script, existing JSON store.

---

### Task 1: Expose playback speed state

**Files:**
- Modify: `src-tauri/src/models/input.rs`
- Modify: `src-tauri/src/mpv.rs`
- Modify: `src/types.ts`
- Modify: `src/detailViews.tsx`

- [ ] **Step 1: Write failing Rust test for speed parsing**

In `src-tauri/src/mpv.rs` tests, extend `parses_progress_written_by_mpv_script` with:

```rust
let state = parse_state(
    r#"{"timePos":12.5,"duration":120.0,"paused":false,"muted":true,"volume":42,"videoReady":true,"speed":1.25}"#,
)
.unwrap();
assert_eq!(state.speed, Some(1.25));
```

- [ ] **Step 2: Run the failing test**

Run: `cd src-tauri && cargo test parses_progress_written_by_mpv_script --lib`

Expected: FAIL because `PlaybackStateResult` has no `speed` field.

- [ ] **Step 3: Add speed to playback state**

In `src-tauri/src/models/input.rs`, add:

```rust
pub(crate) speed: Option<f64>,
```

Update tests constructing `PlaybackStateResult` by adding:

```rust
speed: Some(1.0),
```

In the Lua progress string in `src-tauri/src/mpv.rs`, change the JSON format:

```lua
file:write(string.format('{"timePos":%.3f,"duration":%.3f,"paused":%s,"muted":%s,"volume":%.0f,"videoReady":%s,"speed":%.2f}',
  mp.get_property_number("time-pos", 0),
  mp.get_property_number("duration", 0),
  tostring(mp.get_property_bool("pause", false)),
  tostring(mp.get_property_bool("mute", false)),
  mp.get_property_number("volume", 100),
  tostring(video_ready),
  mp.get_property_number("speed", 1)))
```

In `src/types.ts`, add:

```ts
speed?: number | null;
```

In `src/detailViews.tsx`, compute:

```ts
const speed = state?.speed ?? 1;
```

Show it in the player option grid:

```tsx
<button onClick={() => void onCommand("speed_down")}><span>速度</span><strong>{speed.toFixed(2)}x</strong></button>
```

- [ ] **Step 4: Run tests and build**

Run: `cd src-tauri && cargo test --lib`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/models/input.rs src-tauri/src/mpv.rs src/types.ts src/detailViews.tsx
git commit -m "feat: expose playback speed"
```

### Task 2: Add mpv commands for speed, delay, and external subtitles

**Files:**
- Modify: `src-tauri/src/mpv.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Write failing command normalization tests**

In `src-tauri/src/mpv.rs` tests, add:

```rust
#[test]
fn normalizes_professional_playback_commands() {
    assert_eq!(normalize_command("speed_set:1.25").unwrap(), "speed_set:1.25");
    assert_eq!(normalize_command("speed_set:9").unwrap(), "speed_set:2.00");
    assert_eq!(normalize_command("audio_delay_set:-0.250").unwrap(), "audio_delay_set:-0.250");
    assert_eq!(normalize_command("subtitle_delay_set:0.500").unwrap(), "subtitle_delay_set:0.500");
    assert_eq!(
        normalize_command("external_subtitle:C:/subs/movie.srt").unwrap(),
        "external_subtitle:C:/subs/movie.srt"
    );
}
```

- [ ] **Step 2: Run the failing test**

Run: `cd src-tauri && cargo test normalizes_professional_playback_commands --lib`

Expected: FAIL because the new commands are unknown.

- [ ] **Step 3: Extend command normalization**

In `normalize_command`, add:

```rust
value if value.starts_with("speed_set:") => {
    let speed = value
        .trim_start_matches("speed_set:")
        .parse::<f64>()
        .map_err(|_| "Invalid playback speed.".to_string())?
        .clamp(0.5, 2.0);
    Ok(format!("speed_set:{speed:.2}"))
}
value if value.starts_with("audio_delay_set:") => {
    let delay = value
        .trim_start_matches("audio_delay_set:")
        .parse::<f64>()
        .map_err(|_| "Invalid audio delay.".to_string())?
        .clamp(-10.0, 10.0);
    Ok(format!("audio_delay_set:{delay:.3}"))
}
value if value.starts_with("subtitle_delay_set:") => {
    let delay = value
        .trim_start_matches("subtitle_delay_set:")
        .parse::<f64>()
        .map_err(|_| "Invalid subtitle delay.".to_string())?
        .clamp(-10.0, 10.0);
    Ok(format!("subtitle_delay_set:{delay:.3}"))
}
value if value.starts_with("external_subtitle:") => {
    let target = value.trim_start_matches("external_subtitle:").trim();
    if target.is_empty() {
        return Err("Invalid external subtitle.".to_string());
    }
    Ok(format!("external_subtitle:{target}"))
}
```

- [ ] **Step 4: Update Lua control parsing**

Replace the whitespace stripping line:

```lua
command = command and command:gsub("%s+", "")
```

With:

```lua
command = command and command:gsub("^%s+", ""):gsub("%s+$", "")
```

Add command branches:

```lua
elseif command and command:match("^speed_set:") then
  local speed = tonumber(command:match("^speed_set:(.+)$"))
  if speed then mp.set_property_number("speed", speed) end
elseif command and command:match("^audio_delay_set:") then
  local delay = tonumber(command:match("^audio_delay_set:(.+)$"))
  if delay then mp.set_property_number("audio-delay", delay) end
elseif command and command:match("^subtitle_delay_set:") then
  local delay = tonumber(command:match("^subtitle_delay_set:(.+)$"))
  if delay then mp.set_property_number("sub-delay", delay) end
elseif command and command:match("^external_subtitle:") then
  local target = command:match("^external_subtitle:(.+)$")
  if target and #target > 0 then mp.commandv("sub-add", target, "select") end
```

- [ ] **Step 5: Extend frontend command type**

In `src/types.ts`, extend `PlaybackCommand`:

```ts
| `speed_set:${number}`
| `audio_delay_set:${number}`
| `subtitle_delay_set:${number}`
| `external_subtitle:${string}`
```

- [ ] **Step 6: Run checks**

Run: `cd src-tauri && cargo test --lib`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/mpv.rs src/types.ts
git commit -m "feat: add advanced mpv commands"
```

### Task 3: Persist playback preferences

**Files:**
- Modify: `src-tauri/src/models/server.rs`
- Modify: `src-tauri/src/models/input.rs`
- Modify: `src-tauri/src/store.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/types.ts`
- Modify: `src/ipc.ts`

- [ ] **Step 1: Write failing store preference test**

Add to `src-tauri/src/store.rs` tests:

```rust
#[test]
fn playback_preference_key_prefers_series_then_item() {
    assert_eq!(playback_preference_key(Some("series-1"), "episode-1"), "series:series-1");
    assert_eq!(playback_preference_key(None, "movie-1"), "item:movie-1");
}
```

- [ ] **Step 2: Run the failing test**

Run: `cd src-tauri && cargo test playback_preference_key --lib`

Expected: FAIL because `playback_preference_key` does not exist.

- [ ] **Step 3: Add persisted preference types**

In `src-tauri/src/models/server.rs`, ensure the imports include:

```rust
use std::collections::HashMap;
```

Then add:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct PlaybackPreference {
    pub(crate) media_source_id: Option<String>,
    pub(crate) audio_stream_index: Option<i32>,
    pub(crate) audio_language: Option<String>,
    pub(crate) subtitle_stream_index: Option<i32>,
    pub(crate) subtitle_language: Option<String>,
}
```

Add to `ServerStore`:

```rust
#[serde(default)]
pub(crate) playback_preferences: HashMap<String, HashMap<String, PlaybackPreference>>,
```

In `src-tauri/src/models/input.rs`, add:

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavePlaybackPreferenceInput {
    pub(crate) item_id: String,
    pub(crate) series_id: Option<String>,
    pub(crate) media_source_id: Option<String>,
    pub(crate) audio_stream_index: Option<i32>,
    pub(crate) audio_language: Option<String>,
    pub(crate) subtitle_stream_index: Option<i32>,
    pub(crate) subtitle_language: Option<String>,
}
```

- [ ] **Step 4: Add store helpers**

In `src-tauri/src/store.rs`, add:

```rust
pub(crate) fn playback_preference_key(series_id: Option<&str>, item_id: &str) -> String {
    match series_id.filter(|value| !value.is_empty()) {
        Some(series_id) => format!("series:{series_id}"),
        None => format!("item:{item_id}"),
    }
}

pub(crate) fn save_playback_preference(
    app: &AppHandle,
    server_id: &str,
    key: String,
    preference: crate::models::PlaybackPreference,
) -> Result<(), String> {
    let mut store = load_store(app)?;
    store
        .playback_preferences
        .entry(server_id.to_string())
        .or_default()
        .insert(key, preference);
    save_store(app, &store)
}

pub(crate) fn playback_preferences(
    app: &AppHandle,
    server_id: &str,
) -> Result<std::collections::HashMap<String, crate::models::PlaybackPreference>, String> {
    Ok(load_store(app)?
        .playback_preferences
        .get(server_id)
        .cloned()
        .unwrap_or_default())
}
```

- [ ] **Step 5: Add IPC commands**

In `src-tauri/src/commands.rs`, import `PlaybackPreference` and `SavePlaybackPreferenceInput`. Add:

```rust
#[tauri::command]
pub(crate) async fn save_playback_preference(
    app: AppHandle,
    input: SavePlaybackPreferenceInput,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let server = store::active_server(&app)?;
        let key = store::playback_preference_key(input.series_id.as_deref(), &input.item_id);
        store::save_playback_preference(
            &app,
            &server.id,
            key,
            PlaybackPreference {
                media_source_id: input.media_source_id,
                audio_stream_index: input.audio_stream_index,
                audio_language: input.audio_language,
                subtitle_stream_index: input.subtitle_stream_index,
                subtitle_language: input.subtitle_language,
            },
        )
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) async fn load_playback_preferences(
    app: AppHandle,
) -> Result<std::collections::HashMap<String, PlaybackPreference>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let server = store::active_server(&app)?;
        store::playback_preferences(&app, &server.id)
    })
    .await
    .map_err(|err| err.to_string())?
}
```

Register both commands in `src-tauri/src/lib.rs`.

- [ ] **Step 6: Add frontend types and IPC**

In `src/types.ts`, add:

```ts
export type PlaybackPreferenceInput = {
  itemId: string;
  seriesId?: string | null;
  mediaSourceId?: string | null;
  audioStreamIndex?: number | null;
  audioLanguage?: string | null;
  subtitleStreamIndex?: number | null;
  subtitleLanguage?: string | null;
};

export type PlaybackPreference = Omit<PlaybackPreferenceInput, "itemId" | "seriesId">;
```

In `src/ipc.ts`, add:

```ts
savePlaybackPreference: (input: PlaybackPreferenceInput) => invoke<void>("save_playback_preference", { input }),
loadPlaybackPreferences: () => invoke<Record<string, PlaybackPreference>>("load_playback_preferences"),
```

- [ ] **Step 7: Run checks**

Run: `cd src-tauri && cargo test --lib`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/models/server.rs src-tauri/src/models/input.rs src-tauri/src/store.rs src-tauri/src/commands.rs src-tauri/src/lib.rs src/types.ts src/ipc.ts
git commit -m "feat: persist playback preferences"
```

### Task 4: Apply and update preferences from the player

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/detailViews.tsx`
- Modify: `src/appLogic.ts`

- [ ] **Step 1: Add preference matching helpers**

In `src/appLogic.ts`, add:

```ts
export function streamLanguage(stream?: { language?: string | null }) {
  return stream?.language?.trim().toLowerCase() || undefined;
}

export function playbackPreferenceKey(itemId: string, seriesId?: string | null) {
  return seriesId ? `series:${seriesId}` : `item:${itemId}`;
}

export function preferredStreamIndex(
  streams: { index?: number | null; language?: string | null }[],
  preferredIndex?: number | null,
  preferredLanguage?: string | null,
) {
  if (preferredIndex !== undefined && preferredIndex !== null && streams.some((stream) => stream.index === preferredIndex)) {
    return preferredIndex;
  }
  const language = preferredLanguage?.trim().toLowerCase();
  return language ? streams.find((stream) => streamLanguage(stream) === language)?.index ?? undefined : undefined;
}

export function preferencePayload(itemId: string, seriesId: string | null | undefined, source: MediaVersion | undefined, audioIndex: number | undefined, subtitleIndex: number | undefined) {
  const audio = source?.audioStreams.find((stream) => stream.index === audioIndex);
  const subtitle = source?.subtitleStreams.find((stream) => stream.index === subtitleIndex);
  return {
    itemId,
    seriesId,
    mediaSourceId: source?.id,
    audioStreamIndex: audioIndex,
    audioLanguage: streamLanguage(audio),
    subtitleStreamIndex: subtitleIndex,
    subtitleLanguage: streamLanguage(subtitle),
  };
}
```

Add `MediaVersion` import.

- [ ] **Step 2: Load preferences into App state**

In `src/App.tsx`, add:

```ts
const [playbackPreferences, setPlaybackPreferences] = useState<Record<string, PlaybackPreference>>({});
```

After settings and server loading effects, add:

```ts
useEffect(() => {
  if (!activeServer) {
    setPlaybackPreferences({});
    return;
  }
  ipc.loadPlaybackPreferences()
    .then(setPlaybackPreferences)
    .catch(() => setPlaybackPreferences({}));
}, [activeServer?.id]);
```

Import `PlaybackPreference` from `src/types.ts`.

- [ ] **Step 3: Apply preferences before playback launch**

At the start of `play`, after finding `title`, add:

```ts
const knownItem = findKnownItem(itemId, home, library, detail);
const preference = playbackPreferences[playbackPreferenceKey(itemId, knownItem?.seriesId)];
const source = mediaSourceForPlayback(sources, mediaSourceId ?? preference?.mediaSourceId);
const preferredAudioIndex = audioStreamIndex ?? preferredStreamIndex(source?.audioStreams ?? [], preference?.audioStreamIndex, preference?.audioLanguage);
const preferredSubtitleIndex = subtitleStreamIndex ?? preferredStreamIndex(source?.subtitleStreams ?? [], preference?.subtitleStreamIndex, preference?.subtitleLanguage);
```

Use `preferredAudioIndex` and `preferredSubtitleIndex` in the existing subtitle selection and `ipc.playItem` call:

```ts
const subtitleSelection = resolveSubtitleSelection(source, preferredSubtitleIndex, resolvedSettings.subtitleMode);
```

```ts
preferredAudioIndex,
subtitleSelection.subtitleStreamIndex,
```

- [ ] **Step 4: Save preferences from detail playback selection**

In `src/App.tsx`, after successful `playItem`, call:

```ts
void ipc.savePlaybackPreference(preferencePayload(
  result.itemId,
  findKnownItem(result.itemId, home, library, detail)?.seriesId,
  source,
  preferredAudioIndex,
  subtitleSelection.subtitleStreamIndex,
)).catch(() => {});
```

- [ ] **Step 5: Save preferences from in-player source switches**

After `switchPlayerSource` succeeds, call:

```ts
void ipc.savePlaybackPreference(preferencePayload(
  result.itemId,
  findKnownItem(result.itemId, home, library, detail)?.seriesId,
  nextSource,
  undefined,
  subtitleSelection.subtitleStreamIndex,
)).catch(() => {});
```

- [ ] **Step 6: Save preferences from audio/subtitle menu choices**

In `PlayerView`, add a prop:

```ts
onPreferenceChange: (audioIndex?: number, subtitleIndex?: number) => void;
```

After setting audio:

```ts
onPreferenceChange(nextIndex, subtitleIndex);
```

After setting subtitle:

```ts
onPreferenceChange(audioIndex, nextIndex);
```

In `App.tsx`, pass:

```tsx
onPreferenceChange={(audioIndex, subtitleIndex) => {
  const source = playerSources.find((entry) => entry.id === view.mediaSourceId) ?? playerSources[0];
  void ipc.savePlaybackPreference(preferencePayload(view.itemId, findKnownItem(view.itemId, home, library, detail)?.seriesId, source, audioIndex, subtitleIndex)).catch(() => {});
}}
```

- [ ] **Step 7: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/App.tsx src/detailViews.tsx src/appLogic.ts
git commit -m "feat: apply player stream preferences"
```

### Task 5: Add visible advanced controls

**Files:**
- Modify: `src/detailViews.tsx`
- Modify: `src/styles/player-detail.css`

- [ ] **Step 1: Add local advanced control state**

In `PlayerView`, add state:

```ts
const [subtitleDelay, setSubtitleDelay] = useState(0);
const [audioDelay, setAudioDelay] = useState(0);
const [externalSubtitle, setExternalSubtitle] = useState("");
```

- [ ] **Step 2: Add speed buttons**

In the option grid, replace the speed button from Task 1 with:

```tsx
<div className="player-speed-control">
  <button onClick={() => void onCommand(`speed_set:${Math.max(0.5, speed - 0.1)}`)}><span>速度</span><strong>-</strong></button>
  <button onClick={() => void onCommand("speed_set:1")}><span>{speed.toFixed(2)}x</span><strong>重置</strong></button>
  <button onClick={() => void onCommand(`speed_set:${Math.min(2, speed + 0.1)}`)}><span>速度</span><strong>+</strong></button>
</div>
```

- [ ] **Step 3: Add delay controls**

Add:

```tsx
<div className="player-delay-control">
  <button onClick={() => {
    const next = Number((subtitleDelay - 0.25).toFixed(2));
    setSubtitleDelay(next);
    void onCommand(`subtitle_delay_set:${next}`);
  }}><span>字幕延迟</span><strong>-0.25</strong></button>
  <button onClick={() => {
    const next = Number((subtitleDelay + 0.25).toFixed(2));
    setSubtitleDelay(next);
    void onCommand(`subtitle_delay_set:${next}`);
  }}><span>{subtitleDelay.toFixed(2)}s</span><strong>+0.25</strong></button>
  <button onClick={() => {
    const next = Number((audioDelay + 0.25).toFixed(2));
    setAudioDelay(next);
    void onCommand(`audio_delay_set:${next}`);
  }}><span>音频延迟</span><strong>{audioDelay.toFixed(2)}s</strong></button>
</div>
```

- [ ] **Step 4: Add external subtitle field**

Add:

```tsx
<form className="external-subtitle-form" onSubmit={(event) => {
  event.preventDefault();
  const target = externalSubtitle.trim();
  if (target) void onCommand(`external_subtitle:${target}`);
}}>
  <input value={externalSubtitle} onChange={(event) => setExternalSubtitle(event.currentTarget.value)} aria-label="字幕路径或 URL" />
  <button>加载字幕</button>
</form>
```

- [ ] **Step 5: Add compact CSS**

In `src/styles/player-detail.css`, add:

```css
.player-speed-control,
.player-delay-control,
.external-subtitle-form {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.external-subtitle-form {
  grid-template-columns: minmax(0, 1fr) auto;
}

.external-subtitle-form input {
  min-width: 0;
}
```

- [ ] **Step 6: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/detailViews.tsx src/styles/player-detail.css
git commit -m "feat: add advanced player controls"
```

### Task 6: Phase verification

**Files:**
- No code changes.

- [ ] **Step 1: Run full checks**

Run: `cd src-tauri && cargo test --lib`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

- [ ] **Step 2: Manual checks**

Use `npm run tauri dev` and verify:

```text
1. Start playback and confirm speed shows current mpv speed.
2. Change speed up, down, and reset to 1.00x.
3. Change subtitle delay and audio delay.
4. Load an external subtitle with a URL or simple path without spaces.
5. Choose audio/subtitle/source, stop, replay, and confirm the preference is saved.
```

- [ ] **Step 3: Commit verification fixes if needed**

```bash
git add src src-tauri
git commit -m "fix: polish professional playback controls"
```
