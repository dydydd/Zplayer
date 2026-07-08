# Zplayer Phase 4 Product UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Polish home, detail, empty/loading states, and responsive player/library layouts after the feature data exists.

**Architecture:** Reuse `MediaShelf`, `Poster`, `ScrollableStage`, `LibraryView`, and existing CSS modules. This phase is UI-only unless a small prop is needed to expose data that already exists.

**Tech Stack:** React 19, TypeScript, CSS modules by feature area, existing Tauri app shell.

---

### Task 1: Replace inactive home hero buttons

**Files:**
- Modify: `src/homeView.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles/home.css`

- [ ] **Step 1: Replace hero action props**

In `src/homeView.tsx`, add props:

```ts
onOpenFavorites: () => void;
onNextHero: () => void;
```

The `onNextHero` prop can be optional if the component already advances hero internally:

```ts
onNextHero?: () => void;
```

- [ ] **Step 2: Replace inactive buttons**

Replace:

```tsx
<button className="round-icon add-icon" />
<button className="round-icon next-icon" />
```

With:

```tsx
<button className="round-icon add-icon" onClick={onOpenFavorites} aria-label="打开收藏" />
<button className="round-icon next-icon" onClick={() => setHeroIndex((index) => (index + 1) % Math.max(heroItems.length, 1))} aria-label="下一张推荐" />
```

- [ ] **Step 3: Pass favorites action from App**

In `src/App.tsx`, pass the existing Phase 2 `openFavorites` function:

```tsx
onOpenFavorites={openFavorites}
```

- [ ] **Step 4: Add visible focus styles**

In `src/styles/home.css`, add:

```css
.feature-actions .round-icon:focus-visible {
  outline: 2px solid rgba(255, 255, 255, 0.82);
  outline-offset: 3px;
}
```

- [ ] **Step 5: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/homeView.tsx src/App.tsx src/styles/home.css
git commit -m "feat: activate home hero actions"
```

### Task 2: Reorder and polish home shelves

**Files:**
- Modify: `src/homeView.tsx`
- Modify: `src/styles/home.css`

- [ ] **Step 1: Put high-frequency shelves first**

In `src/homeView.tsx`, order shelves inside `feature-banner` and `home-shelves` as:

```tsx
<MediaShelf title="继续播放" items={home?.resumeItems ?? []} onOpenItem={onOpenItem} className="hero-shelf" showProgress />
<MediaShelf title="最近播放" items={home?.recentItems ?? []} onOpenItem={onOpenItem} className="hero-shelf secondary-hero-shelf" showProgress />
```

Inside `.home-shelves`, render libraries first, then server latest rows:

```tsx
{home?.libraries.length ? <LibraryShelf libraries={home.libraries} onOpenLibrary={onOpenLibrary} /> : null}
{(home?.libraryLatest ?? []).map((row) => (
  <MediaShelf key={row.library.id} title={row.library.name} items={row.items} onOpenItem={onOpenItem} libraryId={row.library.id} onOpenLibrary={onOpenLibrary} floatingControls poster />
))}
```

- [ ] **Step 2: Hide empty duplicate shelves**

Keep the existing `if (!items.length) return null;` in `MediaShelf`. Do not add empty visual shelves.

- [ ] **Step 3: Add spacing for the secondary hero shelf**

In `src/styles/home.css`, add:

```css
.secondary-hero-shelf {
  margin-top: 16px;
}
```

- [ ] **Step 4: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/homeView.tsx src/styles/home.css
git commit -m "feat: polish home shelf order"
```

### Task 3: Improve detail metadata density

**Files:**
- Modify: `src/media.ts`
- Modify: `src/detailViews.tsx`
- Modify: `src/styles/player-detail.css`

- [ ] **Step 1: Add runtime formatter**

In `src/media.ts`, add:

```ts
export function runtimeLabel(runTimeTicks?: number | null) {
  if (!runTimeTicks) return "";
  const minutes = Math.round(runTimeTicks / 600_000_000);
  if (minutes < 60) return `${minutes} 分钟`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest ? `${hours} 小时 ${rest} 分钟` : `${hours} 小时`;
}
```

- [ ] **Step 2: Render richer chips**

In `src/detailViews.tsx`, import `runtimeLabel` and replace the chips block with:

```tsx
<div className="chips">
  {item.year && <span>{item.year}</span>}
  {runtimeLabel(item.runTimeTicks) && <span>{runtimeLabel(item.runTimeTicks)}</span>}
  {item.officialRating && <span>{item.officialRating}</span>}
  {item.communityRating && <span>评分 {item.communityRating.toFixed(1)}</span>}
  {item.genres.slice(0, 3).map((genre) => <button key={genre} className="chip-button" onClick={() => onOpenGenre(genre)}>{genre}</button>)}
</div>
```

- [ ] **Step 3: Add progress hint under overview**

Below the overview paragraph, add:

```tsx
{item.playedPercentage ? (
  <div className="detail-progress">
    <span style={{ width: `${Math.min(Math.max(item.playedPercentage, 0), 100)}%` }} />
  </div>
) : null}
```

- [ ] **Step 4: Add CSS**

In `src/styles/player-detail.css`, add:

```css
.chip-button {
  border: 0;
  color: inherit;
  background: rgba(255, 255, 255, 0.12);
}

.detail-progress {
  width: min(420px, 100%);
  height: 4px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.18);
}

.detail-progress span {
  display: block;
  height: 100%;
  background: rgba(255, 255, 255, 0.82);
}
```

- [ ] **Step 5: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/media.ts src/detailViews.tsx src/styles/player-detail.css
git commit -m "feat: enrich detail metadata"
```

### Task 4: Add reusable empty state component

**Files:**
- Modify: `src/viewParts.tsx`
- Modify: `src/homeView.tsx`
- Modify: `src/libraryViewsCustom.tsx`
- Modify: `src/styles/base.css`
- Modify: `src/styles/library-search.css`

- [ ] **Step 1: Add component**

In `src/viewParts.tsx`, add:

```tsx
export function EmptyState({ title, actionLabel, onAction }: { title: string; actionLabel?: string; onAction?: () => void }) {
  return (
    <div className="empty-panel">
      <strong>{title}</strong>
      {actionLabel && onAction && <button onClick={onAction}>{actionLabel}</button>}
    </div>
  );
}
```

- [ ] **Step 2: Use it in library and search**

In `src/libraryViewsCustom.tsx`, import `EmptyState`. Replace:

```tsx
<div className="empty-panel">这个媒体库暂时没有可显示的项目</div>
```

With:

```tsx
<EmptyState title="这个媒体库暂时没有可显示的项目" />
```

Replace:

```tsx
<div className="empty-panel">换个关键词试试</div>
```

With:

```tsx
<EmptyState title="换个关键词试试" />
```

- [ ] **Step 3: Use it in home when active server has no home rows**

In `src/homeView.tsx`, import `EmptyState` and add after the feature banner:

```tsx
{home && !home.resumeItems.length && !home.recentItems.length && !home.libraries.length && (
  <EmptyState title="这个服务器暂时没有可显示的媒体" onAction={onOpenServers} actionLabel="服务器管理" />
)}
```

- [ ] **Step 4: Add CSS**

In `src/styles/base.css` or `src/styles/library-search.css`, add:

```css
.empty-panel {
  display: grid;
  gap: 12px;
  align-content: center;
  min-height: 160px;
  color: rgba(255, 255, 255, 0.74);
}

.empty-panel strong {
  font-size: 18px;
  font-weight: 650;
}
```

- [ ] **Step 5: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/viewParts.tsx src/homeView.tsx src/libraryViewsCustom.tsx src/styles/base.css src/styles/library-search.css
git commit -m "feat: add reusable empty states"
```

### Task 5: Tighten responsive player and grids

**Files:**
- Modify: `src/styles/responsive.css`
- Modify: `src/styles/player-detail.css`
- Modify: `src/styles/library-search.css`

- [ ] **Step 1: Constrain poster grid columns**

In `src/styles/library-search.css`, ensure poster grids use stable minimums:

```css
.poster-grid {
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
}

.poster-density-compact {
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
}
```

- [ ] **Step 2: Add narrow player layout**

In `src/styles/responsive.css`, add:

```css
@media (max-width: 720px) {
  .player-option-grid {
    grid-template-columns: 1fr;
  }

  .player-main-actions {
    gap: 10px;
  }

  .player-heading {
    grid-template-columns: 1fr;
  }
}
```

- [ ] **Step 3: Prevent detail action wrapping overlap**

In `src/styles/player-detail.css`, add:

```css
.hero-actions {
  flex-wrap: wrap;
}

.hero-option-row {
  max-width: min(760px, 100%);
}
```

- [ ] **Step 4: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/styles/responsive.css src/styles/player-detail.css src/styles/library-search.css
git commit -m "feat: tighten responsive layouts"
```

### Task 6: Phase verification

**Files:**
- No code changes.

- [ ] **Step 1: Run build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 2: Manual UI checks**

Use `npm run tauri dev` and verify:

```text
1. Home hero buttons all perform actions.
2. Home shelves appear in the intended order.
3. Detail metadata fits at narrow and wide widths.
4. Library, search, and home empty states display useful text.
5. Player controls do not overlap around 720px width.
6. Poster grid cards keep stable dimensions while loading images.
```

- [ ] **Step 3: Commit verification fixes if needed**

```bash
git add src
git commit -m "fix: polish product UI"
```
