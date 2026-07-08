# Zplayer Phase 1 Daily Playback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add next episode playback, autoplay, recent plays, recent search terms, and fuller keyboard control.

**Architecture:** Keep playback orchestration in `src/App.tsx`, pure selection helpers in `src/appLogic.ts`, local persistence in `src-tauri/src/store.rs`, and mpv process completion detection in `src-tauri/src/playback_watch.rs`. Home recent-play data loads through `load_home_more` so first-screen home loading stays fast.

**Tech Stack:** Tauri 2, React 19, TypeScript, Rust, mpv Lua control script, existing JSON store.

---

### Task 1: Add completion detection to playback stop events

**Files:**
- Modify: `src-tauri/src/mpv.rs`
- Modify: `src-tauri/src/playback_watch.rs`
- Modify: `src/App.tsx`

- [ ] **Step 1: Write failing Rust tests for completion detection**

Add these tests inside the existing `#[cfg(test)] mod tests` in `src-tauri/src/playback_watch.rs`:

```rust
#[test]
fn completion_requires_success_and_90_percent_progress() {
    let progress = PlaybackStateResult {
        time_pos: Some(91.0),
        duration: Some(100.0),
        paused: false,
        muted: false,
        volume: Some(100),
        video_ready: true,
    };

    assert!(playback_completed(Some(&progress), false, false));
    assert!(!playback_completed(Some(&progress), true, false));
    assert!(!playback_completed(Some(&progress), false, true));
}

#[test]
fn completion_rejects_short_or_missing_duration() {
    let short = PlaybackStateResult {
        time_pos: Some(50.0),
        duration: Some(100.0),
        paused: false,
        muted: false,
        volume: Some(100),
        video_ready: true,
    };
    let missing_duration = PlaybackStateResult {
        duration: None,
        ..short.clone()
    };

    assert!(!playback_completed(Some(&short), false, false));
    assert!(!playback_completed(Some(&missing_duration), false, false));
    assert!(!playback_completed(None, false, false));
}
```

- [ ] **Step 2: Run the failing tests**

Run: `cd src-tauri && cargo test playback_completed --lib`

Expected: FAIL because `playback_completed` does not exist.

- [ ] **Step 3: Track explicit stop commands in mpv control**

In `src-tauri/src/mpv.rs`, change the collection import and add a stop set:

```rust
use std::collections::{HashMap, HashSet};
```

Near the existing static playback maps, add:

```rust
static EXPLICIT_STOPS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
```

Add these helpers near `forget_control`:

```rust
fn remember_explicit_stop(play_session_id: &str) {
    if let Ok(mut stops) = EXPLICIT_STOPS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
    {
        stops.insert(play_session_id.to_string());
    }
}

pub(crate) fn take_explicit_stop(play_session_id: &str) -> bool {
    EXPLICIT_STOPS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .map(|mut stops| stops.remove(play_session_id))
        .unwrap_or(false)
}
```

In `control`, after `let command = normalize_command(command)?;`, add:

```rust
if command == "stop" {
    remember_explicit_stop(play_session_id);
}
```

In `forget_control`, remove any leftover stop marker:

```rust
if let Some(stops) = EXPLICIT_STOPS.get() {
    if let Ok(mut stops) = stops.lock() {
        stops.remove(play_session_id);
    }
}
```

- [ ] **Step 4: Add completion flag to the stop event**

In `src-tauri/src/playback_watch.rs`, extend the event struct:

```rust
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackStoppedEvent {
    item_id: String,
    play_session_id: String,
    failed: bool,
    completed: bool,
}
```

Add this helper below `final_position_ticks`:

```rust
fn playback_completed(
    progress: Option<&PlaybackStateResult>,
    failed: bool,
    explicit_stop: bool,
) -> bool {
    if failed || explicit_stop {
        return false;
    }
    let Some(progress) = progress else {
        return false;
    };
    let Some(duration) = progress.duration.filter(|duration| *duration > 0.0) else {
        return false;
    };
    progress
        .time_pos
        .map(|time| time / duration >= 0.90)
        .unwrap_or(false)
}
```

Before emitting `PlaybackStoppedEvent`, compute:

```rust
let explicit_stop = mpv::take_explicit_stop(&play_session_id);
let completed = playback_completed(progress.as_ref(), failed, explicit_stop);
```

Then emit:

```rust
PlaybackStoppedEvent {
    item_id,
    play_session_id,
    failed,
    completed,
}
```

- [ ] **Step 5: Update the TypeScript event type**

In `src/App.tsx`, change `PlaybackStoppedEvent` to:

```ts
type PlaybackStoppedEvent = {
  itemId: string;
  playSessionId: string;
  failed: boolean;
  completed: boolean;
};
```

- [ ] **Step 6: Run tests**

Run: `cd src-tauri && cargo test --lib`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/mpv.rs src-tauri/src/playback_watch.rs src/App.tsx
git commit -m "feat: detect completed playback sessions"
```

### Task 2: Persist recent plays and autoplay setting

**Files:**
- Modify: `src-tauri/src/models/server.rs`
- Modify: `src-tauri/src/store.rs`
- Modify: `src/types.ts`
- Modify: `src/serverViews.tsx`

- [ ] **Step 1: Write failing Rust tests for recent-play list behavior**

Add this helper test to `src-tauri/src/store.rs` tests:

```rust
#[test]
fn updated_recent_plays_dedupes_and_trims() {
    let ids = (0..55).map(|index| format!("item-{index}")).collect();
    let updated = updated_recent_plays(ids, "item-20");

    assert_eq!(updated.first(), Some(&"item-20".to_string()));
    assert_eq!(updated.len(), 50);
    assert_eq!(updated.iter().filter(|id| *id == "item-20").count(), 1);
}
```

- [ ] **Step 2: Run the failing test**

Run: `cd src-tauri && cargo test updated_recent_plays --lib`

Expected: FAIL because `updated_recent_plays` does not exist.

- [ ] **Step 3: Extend stored settings and store shape**

In `src-tauri/src/models/server.rs`, add imports:

```rust
use std::collections::HashMap;
```

Add to `ServerStore`:

```rust
#[serde(default)]
pub(crate) recent_plays: HashMap<String, Vec<String>>,
```

Add to `AppSettings`:

```rust
#[serde(default = "default_true")]
pub(crate) autoplay_next_episode: bool,
```

Add to `SaveSettingsInput`:

```rust
pub(crate) autoplay_next_episode: Option<bool>,
```

Set defaults in `impl Default for AppSettings` and `normalize_settings`:

```rust
autoplay_next_episode: default_true(),
```

```rust
autoplay_next_episode: input.autoplay_next_episode.unwrap_or_else(default_true),
```

- [ ] **Step 4: Add recent-play store helpers**

In `src-tauri/src/store.rs`, add:

```rust
const RECENT_PLAY_LIMIT: usize = 50;

fn updated_recent_plays(mut ids: Vec<String>, item_id: &str) -> Vec<String> {
    ids.retain(|id| id != item_id);
    ids.insert(0, item_id.to_string());
    ids.truncate(RECENT_PLAY_LIMIT);
    ids
}

pub(crate) fn remember_recent_play(
    app: &AppHandle,
    server_id: &str,
    item_id: &str,
) -> Result<(), String> {
    let mut store = load_store(app)?;
    let ids = store.recent_plays.remove(server_id).unwrap_or_default();
    store
        .recent_plays
        .insert(server_id.to_string(), updated_recent_plays(ids, item_id));
    save_store(app, &store)
}

pub(crate) fn recent_play_ids(app: &AppHandle, server_id: &str) -> Result<Vec<String>, String> {
    Ok(load_store(app)?
        .recent_plays
        .get(server_id)
        .cloned()
        .unwrap_or_default())
}
```

- [ ] **Step 5: Add frontend setting type and control**

In `src/types.ts`, add `autoplayNextEpisode` to `AppSettings`, `ResolvedAppSettings`, `defaultAppSettings`, and `withAppSettingsDefaults`:

```ts
autoplayNextEpisode?: boolean;
```

```ts
autoplayNextEpisode: boolean;
```

```ts
autoplayNextEpisode: true,
```

```ts
autoplayNextEpisode: settings.autoplayNextEpisode ?? defaultAppSettings.autoplayNextEpisode,
```

In `src/serverViews.tsx`, inside the "播放体验" `SettingsPanel`, add:

```tsx
<Toggle label="自动下一集" checked={draft.autoplayNextEpisode} onChange={(checked) => update("autoplayNextEpisode", checked)} />
```

- [ ] **Step 6: Run tests and build**

Run: `cd src-tauri && cargo test updated_recent_plays --lib`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/models/server.rs src-tauri/src/store.rs src/types.ts src/serverViews.tsx
git commit -m "feat: store recent plays and autoplay setting"
```

### Task 3: Return recent items through home secondary loading

**Files:**
- Modify: `src-tauri/src/models/payload.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src/types.ts`
- Modify: `src/App.tsx`
- Modify: `src/homeView.tsx`

- [ ] **Step 1: Add payload fields**

In `src-tauri/src/models/payload.rs`, add `recent_items` to `HomePayload` and `HomeMorePayload`:

```rust
pub(crate) recent_items: Vec<MediaItem>,
```

In `src/types.ts`, add matching fields:

```ts
recentItems: MediaItem[];
```

- [ ] **Step 2: Remember play starts**

In `play_item_sync` in `src-tauri/src/commands.rs`, after `let item = api::resolve_playable_item(...) ?;`, add:

```rust
let _ = store::remember_recent_play(&app, &server.id, &item.id);
```

- [ ] **Step 3: Load recent items in `load_home_more`**

Add this helper in `src-tauri/src/commands.rs` near `load_recommended_movies`:

```rust
fn load_recent_items(
    app: &AppHandle,
    client: &reqwest::blocking::Client,
    server: &SavedServer,
) -> Vec<crate::models::MediaItem> {
    store::recent_play_ids(app, &server.id)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| api::get_item_raw(client, server, &id).ok())
        .map(|item| api::map_item(server, item))
        .take(16)
        .collect()
}
```

In `load_home_sync`, set:

```rust
recent_items: Vec::new(),
```

In `load_home_more_sync`, clone the app handle before `thread::scope`:

```rust
let app_for_recent = app.clone();
```

Spawn and return recent items:

```rust
let recent_items = scope.spawn(|| load_recent_items(&app_for_recent, &client, &server));
```

Add to `HomeMorePayload`:

```rust
recent_items: recent_items.join().unwrap_or_default(),
```

- [ ] **Step 4: Merge and render recent shelf**

In `src/App.tsx`, include `recentItems` when merging `loadHomeMore`:

```ts
const next = {
  ...current,
  libraryLatest: more.libraryLatest,
  recommendedMovies: more.recommendedMovies,
  recommendedShows: more.recommendedShows,
  recentItems: more.recentItems,
};
```

In `src/homeView.tsx`, add a shelf after "继续播放":

```tsx
<MediaShelf
  title="最近播放"
  items={home?.recentItems ?? []}
  onOpenItem={onOpenItem}
  className="hero-shelf"
  showProgress
/>
```

- [ ] **Step 5: Run checks**

Run: `cd src-tauri && cargo test --lib`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models/payload.rs src-tauri/src/commands.rs src/types.ts src/App.tsx src/homeView.tsx
git commit -m "feat: show recently played items"
```

### Task 4: Add episode queue helpers and wire next episode playback

**Files:**
- Modify: `src/appLogic.ts`
- Modify: `src/types.ts`
- Modify: `src/App.tsx`
- Modify: `src/detailViews.tsx`

- [ ] **Step 1: Add pure queue helpers**

Add to `src/appLogic.ts`:

```ts
export type EpisodePlaybackContext = {
  episodeIds: string[];
  episodeIndex: number;
};

export function episodePlaybackContext(itemId: string, episodeIds: string[]): EpisodePlaybackContext | null {
  const episodeIndex = episodeIds.indexOf(itemId);
  return episodeIndex >= 0 ? { episodeIds, episodeIndex } : null;
}

export function relativeEpisodeId(context: EpisodePlaybackContext | null | undefined, offset: -1 | 1) {
  if (!context) return undefined;
  return context.episodeIds[context.episodeIndex + offset];
}
```

- [ ] **Step 2: Extend player view state**

In `src/types.ts`, extend the `player` view:

```ts
| {
    name: "player";
    itemId: string;
    title: string;
    playSessionId?: string | null;
    mediaSourceId?: string | null;
    subtitleStreamIndex?: number | null;
    episodeIds?: string[] | null;
    episodeIndex?: number | null;
  };
```

- [ ] **Step 3: Pass episode ids from detail playback**

In `src/detailViews.tsx`, change `onPlay` prop type to include `episodeIds?: string[]`:

```ts
onPlay: (id: string, mediaSourceId?: string, audioStreamIndex?: number, subtitleStreamIndex?: number, sources?: MediaVersion[], episodeIds?: string[]) => Promise<void>;
```

Change the hero play call:

```tsx
onClick={() => void onPlay(selectedPlayableId, selectedSource?.id, audioStreamIndex, subtitleStreamIndex, currentMediaSources, episodes.map((episode) => episode.id))}
```

- [ ] **Step 4: Wire queue in `App.tsx`**

Update imports:

```ts
import { episodePlaybackContext, findKnownItem, libraryKey, relativeEpisodeId } from "./appLogic";
```

Change `play` signature:

```ts
async function play(itemId: string, mediaSourceId?: string, audioStreamIndex?: number, subtitleStreamIndex?: number, sources?: MediaVersion[], episodeIds?: string[]) {
```

Before `openView`, compute:

```ts
const episodeContext = episodeIds ? episodePlaybackContext(result.itemId, episodeIds) : null;
```

Add to the player view:

```ts
episodeIds: episodeContext?.episodeIds ?? null,
episodeIndex: episodeContext?.episodeIndex ?? null,
```

Add a helper in `App`:

```ts
function episodeContextFromView(targetView: View = viewRef.current) {
  if (targetView.name !== "player" || !targetView.episodeIds || targetView.episodeIndex === null || targetView.episodeIndex === undefined) return null;
  return { episodeIds: targetView.episodeIds, episodeIndex: targetView.episodeIndex };
}

async function playRelativeEpisode(offset: -1 | 1, stopCurrent = true) {
  const context = episodeContextFromView();
  const nextItemId = relativeEpisodeId(context, offset);
  if (!nextItemId) return;
  if (stopCurrent && view.name === "player" && view.playSessionId) {
    exitingPlaybackSession.current = view.playSessionId;
    void ipc.controlPlayback(view.playSessionId, "stop").catch(() => {});
  }
  await play(nextItemId, undefined, undefined, undefined, undefined, context?.episodeIds);
}
```

- [ ] **Step 5: Autoplay on completed playback**

After the `resolvedSettings` `useMemo`, add a settings ref:

```ts
const resolvedSettingsRef = useRef<ResolvedAppSettings>(resolvedSettings);
```

Keep it current:

```ts
useEffect(() => {
  resolvedSettingsRef.current = resolvedSettings;
}, [resolvedSettings]);
```

Inside the `playback-stopped` listener branch for current player session, before `goBack()`, add:

```ts
if (event.payload.completed && resolvedSettingsRef.current.autoplayNextEpisode) {
  void playRelativeEpisode(1, false);
  refreshAfterPlaybackStopRef.current(event.payload.itemId);
  return;
}
```

- [ ] **Step 6: Add previous/next buttons to `PlayerView`**

In `src/detailViews.tsx`, extend `PlayerView` props:

```ts
canPlayPrevious: boolean;
canPlayNext: boolean;
onPlayPrevious: () => Promise<void>;
onPlayNext: () => Promise<void>;
```

In `App.tsx`, pass:

```tsx
canPlayPrevious={!!relativeEpisodeId(episodeContextFromView(view), -1)}
canPlayNext={!!relativeEpisodeId(episodeContextFromView(view), 1)}
onPlayPrevious={() => playRelativeEpisode(-1)}
onPlayNext={() => playRelativeEpisode(1)}
```

In the player action row, add buttons near seek controls:

```tsx
{canPlayPrevious && <button className="player-round" onClick={() => void onPlayPrevious()} aria-label="上一集"><SvgIcon name="back" /></button>}
{canPlayNext && <button className="player-round" onClick={() => void onPlayNext()} aria-label="下一集"><SvgIcon name="next" /></button>}
```

- [ ] **Step 7: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add src/appLogic.ts src/types.ts src/App.tsx src/detailViews.tsx
git commit -m "feat: add episode queue playback"
```

### Task 5: Add recent search terms and keyboard coverage

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/libraryViewsCustom.tsx`
- Modify: `src/detailViews.tsx`

- [ ] **Step 1: Persist recent search terms in the browser**

In `src/App.tsx`, add state:

```ts
const [recentSearchTerms, setRecentSearchTerms] = useState<string[]>(() => {
  try {
    return JSON.parse(localStorage.getItem("zplayer:recent-searches") ?? "[]").slice(0, 8);
  } catch {
    return [];
  }
});
```

Add a helper:

```ts
function rememberSearchTerm(query: string) {
  const term = query.trim();
  if (!term) return;
  setRecentSearchTerms((current) => {
    const next = [term, ...current.filter((value) => value !== term)].slice(0, 8);
    localStorage.setItem("zplayer:recent-searches", JSON.stringify(next));
    return next;
  });
}
```

Call it in `openSearchResult` before clearing the query:

```ts
rememberSearchTerm(searchQuery);
```

- [ ] **Step 2: Render recent search terms**

In `src/App.tsx`, change the search overlay condition from query-only to open-state:

```tsx
{view.name !== "player" && searchOpen && (
  <SearchOverlay
    results={searchResults}
    query={searchQuery}
    loading={searchLoading}
    posterDensity={resolvedSettings.posterDensity}
    recentTerms={recentSearchTerms}
    onUseRecentTerm={(term) => {
      setSearchQuery(term);
      setSearchOpen(true);
    }}
    onOpen={openSearchResult}
  />
)}
```

In `src/libraryViewsCustom.tsx`, extend `SearchOverlay` props:

```ts
recentTerms: string[];
onUseRecentTerm: (term: string) => void;
```

Change the count line so empty query is a recent-search state:

```tsx
{!query.trim()
  ? "最近搜索"
  : loading
    ? "搜索中..."
    : results.length
      ? `找到 ${results.length} 个结果`
      : `没有找到“${query}”`}
```

Render before the empty panel:

```tsx
{!query.trim() && recentTerms.length > 0 && (
  <div className="search-recents">
    {recentTerms.map((term) => (
      <button key={term} onClick={() => onUseRecentTerm(term)}>{term}</button>
    ))}
  </div>
)}
```

- [ ] **Step 3: Add player keyboard actions**

In `src/detailViews.tsx`, extend the `commands` map with only playback commands:

```ts
a: "audio_next",
A: "audio_next",
s: "subtitle_next",
S: "subtitle_next",
```

Because fullscreen is not a `PlaybackCommand`, handle it before the map:

```ts
if (event.key === "f" || event.key === "F") {
  event.preventDefault();
  onToggleFullscreen();
  return;
}
if ((event.key === "ArrowLeft" || event.key === "ArrowRight") && event.shiftKey) {
  event.preventDefault();
  void (event.key === "ArrowLeft" ? onPlayPrevious() : onPlayNext());
  return;
}
```

- [ ] **Step 4: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/libraryViewsCustom.tsx src/detailViews.tsx
git commit -m "feat: add recent search and player shortcuts"
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
1. Start an episode from a show detail page.
2. Use previous and next episode controls.
3. Let an episode finish near the end and confirm the next episode starts.
4. Stop playback manually and confirm autoplay does not fire.
5. Start any item and confirm it appears in Recently Played.
6. Search for a term, open an item, reopen search, and confirm the term appears.
7. Use Space, arrows, F, A, S, Shift+Left, and Shift+Right in the player.
```

- [ ] **Step 3: Commit verification notes if needed**

If manual verification uncovers only CSS text or label tweaks, commit them:

```bash
git add src
git commit -m "fix: polish daily playback flow"
```
