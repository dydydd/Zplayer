# Zplayer Phase 2 Library Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add favorites, watched filters, poster actions, genre/actor navigation, and collection-aware library grids.

**Architecture:** Extend the existing `library` route instead of adding new grid screens. Rust normalizes filter inputs, `api::get_library_items` maps them to Emby/Jellyfin query params, and React reuses `LibraryView` plus `Poster`.

**Tech Stack:** Tauri 2 IPC, React 19, TypeScript discriminated unions, Rust request builders, existing Emby/Jellyfin APIs.

---

### Task 1: Add typed library filters end to end

**Files:**
- Modify: `src/types.ts`
- Modify: `src/ipc.ts`
- Modify: `src-tauri/src/models/input.rs`
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Extend shared frontend types**

In `src/types.ts`, add:

```ts
export type LibraryPlayedFilter = "" | "played" | "unplayed";

export type LibraryFilters = {
  played?: LibraryPlayedFilter;
  favorite?: boolean;
  genre?: string;
  personId?: string;
  collectionId?: string;
};
```

Extend the library view:

```ts
| {
    name: "library";
    id: string;
    title?: string;
    itemType?: LibraryItemType;
    sortBy?: LibrarySortBy;
    sortOrder?: LibrarySortOrder;
    filters?: LibraryFilters;
  }
```

- [ ] **Step 2: Extend IPC input**

Change `src/ipc.ts` `loadLibrary` signature:

```ts
loadLibrary: (
  libraryId: string,
  startIndex: number,
  limit: number,
  itemType: LibraryItemType,
  sortBy: LibrarySortBy,
  sortOrder: LibrarySortOrder,
  filters: LibraryFilters = {},
) => invoke<LibraryPayload>("load_library", { input: { libraryId, startIndex, limit, itemType, sortBy, sortOrder, filters } }),
```

Add `LibraryFilters` to the type import.

- [ ] **Step 3: Extend Rust input**

In `src-tauri/src/models/input.rs`, add:

```rust
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct LibraryFiltersInput {
    pub(crate) played: Option<String>,
    pub(crate) favorite: Option<bool>,
    pub(crate) genre: Option<String>,
    pub(crate) person_id: Option<String>,
    pub(crate) collection_id: Option<String>,
}
```

Add to `LibraryInput`:

```rust
pub(crate) filters: Option<LibraryFiltersInput>,
```

- [ ] **Step 4: Add normalized filters in commands**

In `src-tauri/src/commands.rs`, import `LibraryFiltersInput` and add:

```rust
#[derive(Default, Clone)]
struct LibraryFilters {
    played: Option<bool>,
    favorite: Option<bool>,
    genre: Option<String>,
    person_id: Option<String>,
    collection_id: Option<String>,
}

fn normalize_library_filters(input: Option<LibraryFiltersInput>) -> LibraryFilters {
    let input = input.unwrap_or_default();
    LibraryFilters {
        played: match input.played.as_deref() {
            Some("played") => Some(true),
            Some("unplayed") => Some(false),
            _ => None,
        },
        favorite: input.favorite,
        genre: input.genre.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
        person_id: input.person_id.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
        collection_id: input.collection_id.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
    }
}
```

- [ ] **Step 5: Wire filters into library loading**

In `load_library_sync`, compute:

```rust
let filters = normalize_library_filters(input.filters.clone());
```

Pass `&filters` to the API call in both first-page and later-page branches. This requires Task 2 to change the API function signature.

- [ ] **Step 6: Run build to see expected Rust compile failure**

Run: `cd src-tauri && cargo test --lib`

Expected: FAIL because `api::get_library_items` does not accept filters yet.

- [ ] **Step 7: Leave the tree uncommitted and continue to Task 2**

Do not commit this task before Task 2 because the Rust function signature change is incomplete.

### Task 2: Map filters to Emby/Jellyfin item queries

**Files:**
- Modify: `src-tauri/src/api/mod.rs`
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Write failing API parameter tests**

In `src-tauri/src/api/mod.rs` tests, add:

```rust
#[test]
fn library_filters_emit_server_params() {
    let filters = LibraryQueryFilters {
        played: Some(false),
        favorite: Some(true),
        genre: Some("Drama".to_string()),
        person_id: Some("person-1".to_string()),
        collection_id: None,
    };
    let mut params = Vec::new();
    append_library_filters(&mut params, &filters);

    assert!(params.contains(&("Filters", "IsUnplayed,IsFavorite".to_string())));
    assert!(params.contains(&("filters", "IsUnplayed,IsFavorite".to_string())));
    assert!(params.contains(&("Genres", "Drama".to_string())));
    assert!(params.contains(&("genres", "Drama".to_string())));
    assert!(params.contains(&("PersonIds", "person-1".to_string())));
    assert!(params.contains(&("personIds", "person-1".to_string())));
}
```

- [ ] **Step 2: Run the failing test**

Run: `cd src-tauri && cargo test library_filters_emit_server_params --lib`

Expected: FAIL because `LibraryQueryFilters` and `append_library_filters` do not exist.

- [ ] **Step 3: Add API filter struct and helper**

In `src-tauri/src/api/mod.rs`, add near item query functions:

```rust
#[derive(Default, Clone)]
pub(crate) struct LibraryQueryFilters {
    pub(crate) played: Option<bool>,
    pub(crate) favorite: Option<bool>,
    pub(crate) genre: Option<String>,
    pub(crate) person_id: Option<String>,
    pub(crate) collection_id: Option<String>,
}

fn append_library_filters(params: &mut Vec<(&'static str, String)>, filters: &LibraryQueryFilters) {
    let mut filter_values = Vec::new();
    if let Some(played) = filters.played {
        filter_values.push(if played { "IsPlayed" } else { "IsUnplayed" });
    }
    if filters.favorite == Some(true) {
        filter_values.push("IsFavorite");
    }
    if !filter_values.is_empty() {
        let value = filter_values.join(",");
        params.push(("Filters", value.clone()));
        params.push(("filters", value));
    }
    if let Some(genre) = filters.genre.as_deref() {
        params.push(("Genres", genre.to_string()));
        params.push(("genres", genre.to_string()));
    }
    if let Some(person_id) = filters.person_id.as_deref() {
        params.push(("PersonIds", person_id.to_string()));
        params.push(("personIds", person_id.to_string()));
    }
}
```

- [ ] **Step 4: Extend `get_library_items`**

Change the signature:

```rust
pub(crate) fn get_library_items(
    client: &Client,
    server: &SavedServer,
    library_id: &str,
    start_index: usize,
    limit: usize,
    item_type: Option<&str>,
    sort_by: &str,
    sort_order: &str,
    filters: &LibraryQueryFilters,
) -> Result<(Vec<MediaItem>, usize), String> {
```

Push `ParentId` only when there is an actual parent:

```rust
let parent_id = filters.collection_id.as_deref().unwrap_or(library_id);
if !parent_id.is_empty() {
    emby_params.push(("ParentId", parent_id.to_string()));
    jellyfin_params.push(("parentId", parent_id.to_string()));
}
```

Call:

```rust
append_library_filters(&mut emby_params, filters);
append_library_filters(&mut jellyfin_params, filters);
```

- [ ] **Step 5: Convert command filters to API filters**

In `src-tauri/src/commands.rs`, replace the local `LibraryFilters` struct with this return type:

```rust
fn normalize_library_filters(input: Option<LibraryFiltersInput>) -> api::LibraryQueryFilters {
    let input = input.unwrap_or_default();
    api::LibraryQueryFilters {
        played: match input.played.as_deref() {
            Some("played") => Some(true),
            Some("unplayed") => Some(false),
            _ => None,
        },
        favorite: input.favorite,
        genre: input.genre.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
        person_id: input.person_id.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
        collection_id: input.collection_id.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
    }
}
```

- [ ] **Step 6: Handle synthetic all-library/favorites payload titles**

In `load_library_sync`, when `input.library_id` is empty on the first page, skip `fetch_libraries` and use:

```rust
MediaLibrary {
    id: input.library_id.clone(),
    name: "收藏".to_string(),
    collection_type: None,
    image_url: None,
}
```

- [ ] **Step 7: Run Rust tests**

Run: `cd src-tauri && cargo test --lib`

Expected: PASS.

- [ ] **Step 8: Commit backend filter support**

```bash
git add src-tauri/src/models/input.rs src-tauri/src/api/mod.rs src-tauri/src/commands.rs src/types.ts src/ipc.ts
git commit -m "feat: add library query filters"
```

### Task 3: Add filter controls and favorites navigation

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/homeView.tsx`
- Modify: `src/libraryViewsCustom.tsx`

- [ ] **Step 1: Update library load cache key**

In `src/appLogic.ts`, change `libraryKey` to include filters:

```ts
export function libraryKey(libraryId: string, itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder, filters: LibraryFilters = {}) {
  return `${libraryId}:${itemType}:${sortBy}:${sortOrder}:${JSON.stringify(filters)}`;
}
```

Add `LibraryFilters` to the import.

- [ ] **Step 2: Wire filters in `App.tsx`**

Update all `loadLibrary` calls to pass `view.filters ?? {}`.

Change `updateLibraryOptions`:

```ts
function updateLibraryOptions(itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder, filters: LibraryFilters) {
  if (view.name !== "library") return;
  setLibrary(null);
  setView({ ...view, itemType, sortBy, sortOrder, filters });
}
```

Add:

```ts
const openFavorites = useCallback(() => openView({
  name: "library",
  id: "",
  title: "收藏",
  filters: { favorite: true },
}), [openView]);
```

- [ ] **Step 3: Add Favorites entry on home server menu**

In `src/homeView.tsx`, add `onOpenFavorites` prop:

```ts
onOpenFavorites: () => void;
```

Add a button in the server popover:

```tsx
<button onClick={() => {
  setServerMenuOpen(false);
  onOpenFavorites();
}}>收藏</button>
```

Pass it from `App.tsx`:

```tsx
onOpenFavorites={openFavorites}
```

- [ ] **Step 4: Add filter controls to `LibraryView`**

In `src/libraryViewsCustom.tsx`, import `LibraryFilters` and `LibraryPlayedFilter`. Add options:

```ts
const playedOptions: { value: LibraryPlayedFilter; label: string }[] = [
  { value: "", label: "全部" },
  { value: "unplayed", label: "未看" },
  { value: "played", label: "已看" },
];

const favoriteOptions = [
  { value: "", label: "全部" },
  { value: "favorite", label: "收藏" },
] as const;
```

Extend props:

```ts
filters: LibraryFilters;
onOptionsChange: (itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder, filters: LibraryFilters) => void;
```

Render controls:

```tsx
<FilterMenu label="观看" value={filters.played ?? ""} options={playedOptions} onChange={(value) => onOptionsChange(itemType, sortBy, sortOrder, { ...filters, played: value })} />
<FilterMenu label="收藏" value={filters.favorite ? "favorite" : ""} options={favoriteOptions} onChange={(value) => onOptionsChange(itemType, sortBy, sortOrder, { ...filters, favorite: value === "favorite" || undefined })} />
```

- [ ] **Step 5: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/appLogic.ts src/App.tsx src/homeView.tsx src/libraryViewsCustom.tsx
git commit -m "feat: add library filters UI"
```

### Task 4: Add poster-level favorite and watched actions

**Files:**
- Modify: `src/viewParts.tsx`
- Modify: `src/libraryViewsCustom.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles/library-search.css`

- [ ] **Step 1: Extend `Poster` props**

In `src/viewParts.tsx`, change the component signature:

```tsx
export const Poster = memo(function Poster({
  item,
  onOpen,
  hideMeta = false,
  onToggleFavorite,
  onTogglePlayed,
}: {
  item: MediaItem;
  onOpen: (id: string) => void;
  hideMeta?: boolean;
  onToggleFavorite?: (item: MediaItem) => void;
  onTogglePlayed?: (item: MediaItem) => void;
}) {
```

Replace the `Poster` return block with non-nested interactive elements:

```tsx
return (
  <article className="poster">
    <button className="poster-main" onClick={() => onOpen(item.id)}>
      <span className="poster-cover">
        <Image src={item.primaryImageUrl} alt={item.name} />
        {item.communityRating && <span className="score">{item.communityRating.toFixed(1)}</span>}
      </span>
      <strong>{item.name}</strong>
      {!hideMeta && <small>{itemMeta(item)}</small>}
    </button>
    {(onToggleFavorite || onTogglePlayed) && (
      <div className="poster-actions">
        {onToggleFavorite && <button type="button" onClick={() => onToggleFavorite(item)}>{item.favorite ? "已收藏" : "收藏"}</button>}
        {onTogglePlayed && <button type="button" onClick={() => onTogglePlayed(item)}>{item.played ? "已看" : "标记"}</button>}
      </div>
    )}
  </article>
);
```

- [ ] **Step 2: Add mutation helper in App**

In `src/App.tsx`, add:

```ts
async function toggleItemFavorite(item: MediaItem) {
  const nextValue = !item.favorite;
  const result = await run(nextValue ? "收藏媒体" : "取消收藏", () => ipc.markFavorite(item.id, nextValue));
  if (result !== null) {
    invalidatePlaybackCaches(item.id);
    if (view.name === "library") void loadLibrary(view.id, view.itemType ?? "", view.sortBy ?? "DateCreated", view.sortOrder ?? "Descending", view.filters ?? {});
  }
}

async function toggleItemPlayed(item: MediaItem) {
  const nextValue = !item.played;
  const result = await run(nextValue ? "标记已看" : "标记未看", () => ipc.markPlayed(item.id, nextValue));
  if (result !== null) {
    invalidatePlaybackCaches(item.id);
    if (view.name === "library") void loadLibrary(view.id, view.itemType ?? "", view.sortBy ?? "DateCreated", view.sortOrder ?? "Descending", view.filters ?? {});
  }
}
```

- [ ] **Step 3: Pass poster actions from library**

Extend `LibraryView` props:

```ts
onToggleFavorite: (item: MediaItem) => void;
onTogglePlayed: (item: MediaItem) => void;
```

Pass to `Poster`:

```tsx
<Poster key={item.id} item={item} onOpen={onOpenItem} hideMeta onToggleFavorite={onToggleFavorite} onTogglePlayed={onTogglePlayed} />
```

Pass from `App.tsx`:

```tsx
onToggleFavorite={(item) => void toggleItemFavorite(item)}
onTogglePlayed={(item) => void toggleItemPlayed(item)}
```

- [ ] **Step 4: Add minimal CSS**

In `src/styles/library-search.css`, add:

```css
.poster-actions {
  display: flex;
  gap: 8px;
  margin-top: 8px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.72);
}

.poster-main {
  all: unset;
  display: grid;
  gap: 8px;
  cursor: pointer;
}

.poster-actions button {
  padding: 4px 6px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: 6px;
  color: inherit;
  background: transparent;
}
```

- [ ] **Step 5: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/viewParts.tsx src/libraryViewsCustom.tsx src/App.tsx src/styles/library-search.css
git commit -m "feat: add poster library actions"
```

### Task 5: Add genre and actor navigation from details

**Files:**
- Modify: `src/detailViews.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Extend detail props**

In `src/detailViews.tsx`, add props:

```ts
onOpenGenre: (genre: string) => void;
onOpenPerson: (personId: string, name: string) => void;
```

- [ ] **Step 2: Make genre chips clickable**

Replace genre chip rendering with:

```tsx
{item.genres.slice(0, 3).map((genre) => (
  <button key={genre} className="chip-button" onClick={() => onOpenGenre(genre)}>{genre}</button>
))}
```

- [ ] **Step 3: Make people cards clickable when an id exists**

Replace each person card wrapper:

```tsx
<button
  key={`${person.id ?? person.name}-${person.role ?? ""}`}
  className="person-card"
  onClick={() => person.id && onOpenPerson(person.id, person.name)}
  disabled={!person.id}
>
```

- [ ] **Step 4: Open filtered libraries in App**

In `src/App.tsx`, add:

```ts
const openGenre = useCallback((genre: string) => openView({
  name: "library",
  id: "",
  title: genre,
  filters: { genre },
}), [openView]);

const openPerson = useCallback((personId: string, name: string) => openView({
  name: "library",
  id: "",
  title: name,
  filters: { personId },
}), [openView]);
```

Pass to `DetailView`:

```tsx
onOpenGenre={openGenre}
onOpenPerson={openPerson}
```

- [ ] **Step 5: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/detailViews.tsx src/App.tsx
git commit -m "feat: navigate by genre and person"
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
1. Open Favorites and confirm it loads a grid.
2. Filter a library by watched, unwatched, and favorite.
3. Toggle favorite and watched from a poster.
4. Open a genre from details and confirm a filtered grid loads.
5. Open an actor from details and confirm a filtered grid loads.
6. Open a collection-backed item if the server exposes collections and confirm the grid loads.
```

- [ ] **Step 3: Commit verification fixes if needed**

```bash
git add src src-tauri
git commit -m "fix: polish library management"
```
