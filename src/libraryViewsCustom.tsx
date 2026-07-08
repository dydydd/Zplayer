import { useEffect, useRef, useState } from "react";
import type { LibraryFilters, LibraryItemType, LibraryPayload, LibraryPlayedFilter, LibrarySortBy, LibrarySortOrder, MediaItem, PosterDensity } from "./types";
import { Poster, useFloatingBackVisible } from "./viewParts";

const typeOptions: { value: LibraryItemType; label: string }[] = [
  { value: "", label: "全部" },
  { value: "Movie", label: "电影" },
  { value: "Series", label: "剧集" },
  { value: "Episode", label: "单集" },
  { value: "Video", label: "视频" },
];
const sortOptions: { value: LibrarySortBy; label: string }[] = [
  { value: "DateCreated", label: "最近添加" },
  { value: "SortName", label: "名称" },
  { value: "PremiereDate", label: "首播日期" },
  { value: "CommunityRating", label: "评分" },
];
const orderOptions: { value: LibrarySortOrder; label: string }[] = [
  { value: "Descending", label: "降序" },
  { value: "Ascending", label: "升序" },
];
const playedOptions: { value: LibraryPlayedFilter; label: string }[] = [
  { value: "", label: "全部" },
  { value: "unplayed", label: "未看" },
  { value: "played", label: "已看" },
];
const favoriteOptions = [
  { value: "", label: "全部" },
  { value: "favorite", label: "收藏" },
] as const;

export function LibraryView({
  payload,
  title,
  loadingMore,
  onBack,
  onOpenItem,
  onLoadMore,
  itemType,
  sortBy,
  sortOrder,
  filters,
  posterDensity,
  onOptionsChange,
  onToggleFavorite,
  onTogglePlayed,
}: {
  payload: LibraryPayload;
  title?: string;
  loadingMore: boolean;
  onBack: () => void;
  onOpenItem: (id: string) => void;
  onLoadMore: () => void;
  itemType: LibraryItemType;
  sortBy: LibrarySortBy;
  sortOrder: LibrarySortOrder;
  filters: LibraryFilters;
  posterDensity: PosterDensity;
  onOptionsChange: (itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder, filters: LibraryFilters) => void;
  onToggleFavorite: (item: MediaItem) => void;
  onTogglePlayed: (item: MediaItem) => void;
}) {
  const backVisible = useFloatingBackVisible(payload.library.id);
  const gridRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!payload.hasMore || loadingMore) return;
    const scroller = document.querySelector<HTMLElement>(".workspace");
    const target = gridRef.current?.lastElementChild;
    if (!scroller || !target) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) onLoadMore();
      },
      { root: scroller, rootMargin: "700px 0px" },
    );
    observer.observe(target);
    return () => observer.disconnect();
  }, [loadingMore, onLoadMore, payload.hasMore, payload.items.length]);

  return (
    <div className="page library-page">
      <button className={`back floating-back ${backVisible ? "" : "hidden"}`} onClick={onBack} aria-label="返回" />
      <div className="library-heading">
        <h1>{title ?? payload.library.name}</h1>
        <div className="filters">
          <span>{payload.totalCount} 项</span>
          <FilterMenu label="类型" value={itemType} options={typeOptions} onChange={(value) => onOptionsChange(value, sortBy, sortOrder, filters)} />
          <FilterMenu label="排序" value={sortBy} options={sortOptions} onChange={(value) => onOptionsChange(itemType, value, sortOrder, filters)} />
          <FilterMenu label="方向" value={sortOrder} options={orderOptions} onChange={(value) => onOptionsChange(itemType, sortBy, value, filters)} />
          <FilterMenu label="观看" value={filters.played ?? ""} options={playedOptions} onChange={(value) => onOptionsChange(itemType, sortBy, sortOrder, { ...filters, played: value })} />
          <FilterMenu label="收藏" value={filters.favorite ? "favorite" : ""} options={favoriteOptions} onChange={(value) => onOptionsChange(itemType, sortBy, sortOrder, { ...filters, favorite: value === "favorite" || undefined })} />
        </div>
      </div>
      {!payload.items.length && !loadingMore && <div className="empty-panel">这个媒体库暂时没有可显示的项目</div>}
      <div className={`poster-grid poster-density-${posterDensity}`} ref={gridRef}>
        {payload.items.map((item) => (
          <Poster key={item.id} item={item} onOpen={onOpenItem} hideMeta onToggleFavorite={onToggleFavorite} onTogglePlayed={onTogglePlayed} />
        ))}
      </div>
      {loadingMore && <div className="loading-more">加载中...</div>}
    </div>
  );
}

function FilterMenu<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement | null>(null);
  const selected = options.find((option) => option.value === value) ?? options[0];

  useEffect(() => {
    if (!open) return;
    function close(event: MouseEvent) {
      if (!ref.current?.contains(event.target as Node)) setOpen(false);
    }
    function closeOnEscape(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    window.addEventListener("mousedown", close);
    window.addEventListener("keydown", closeOnEscape);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("keydown", closeOnEscape);
    };
  }, [open]);

  return (
    <div className={`filter-menu ${open ? "open" : ""}`} ref={ref}>
      <button className="filter-menu-trigger" type="button" aria-label={`${label}: ${selected.label}`} aria-expanded={open} onClick={() => setOpen((current) => !current)}>
        <strong>{selected.label}</strong>
        <i />
      </button>
      {open && (
        <div className="filter-menu-popover">
          {options.map((option) => (
            <button
              key={option.value}
              className={option.value === value ? "active" : ""}
              type="button"
              onClick={() => {
                onChange(option.value);
                setOpen(false);
              }}
            >
              {option.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function SearchOverlay({
  results,
  query,
  loading,
  posterDensity,
  recentTerms,
  onUseRecentTerm,
  onOpen,
}: {
  results: MediaItem[];
  query: string;
  loading: boolean;
  posterDensity: PosterDensity;
  recentTerms: string[];
  onUseRecentTerm: (term: string) => void;
  onOpen: (id: string) => void;
}) {
  const trimmedQuery = query.trim();

  return (
    <div className="page search-page">
      <h1>搜索</h1>
      <p className="search-count">
        {!trimmedQuery
          ? "最近搜索"
          : loading
            ? "搜索中..."
            : results.length
              ? `找到 ${results.length} 个结果`
              : `没有找到“${query}”`}
      </p>
      {!trimmedQuery && recentTerms.length > 0 && (
        <div className="search-recents">
          {recentTerms.map((term) => (
            <button key={term} onClick={() => onUseRecentTerm(term)}>{term}</button>
          ))}
        </div>
      )}
      {!loading && trimmedQuery && !results.length && <div className="empty-panel">换个关键词试试</div>}
      <div className={`poster-grid poster-density-${posterDensity} ${loading ? "is-loading" : ""}`}>
        {results.map((item) => (
          <Poster key={item.id} item={item} onOpen={onOpen} />
        ))}
      </div>
    </div>
  );
}
