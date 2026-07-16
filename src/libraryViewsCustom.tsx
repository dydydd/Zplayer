import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { LibraryFilters, LibraryItemType, LibraryPayload, LibraryPlayedFilter, LibrarySortBy, LibrarySortOrder, MediaItem, PosterDensity } from "./types";
import { UiIcon } from "./icons";
import { EmptyState, Poster, useFloatingBackVisible } from "./viewParts";

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
  onOpenItem: (id: string, serverId?: string | null) => void;
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
  const { t } = useTranslation();
  const backVisible = useFloatingBackVisible(payload.library.id);
  const gridRef = useRef<HTMLDivElement | null>(null);
  const typeOptions: { value: LibraryItemType; label: string }[] = [
    { value: "", label: t("library.all") },
    { value: "Movie", label: t("library.movie") },
    { value: "Series", label: t("library.series") },
    { value: "Episode", label: t("library.episode") },
    { value: "Video", label: t("library.video") },
  ];
  const sortOptions: { value: LibrarySortBy; label: string }[] = [
    { value: "DateCreated", label: t("library.recentlyAdded") },
    { value: "SortName", label: t("library.name") },
    { value: "PremiereDate", label: t("library.premiereDate") },
    { value: "CommunityRating", label: t("library.rating") },
  ];
  const orderOptions: { value: LibrarySortOrder; label: string }[] = [
    { value: "Descending", label: t("library.descending") },
    { value: "Ascending", label: t("library.ascending") },
  ];
  const playedOptions: { value: LibraryPlayedFilter; label: string }[] = [
    { value: "", label: t("library.all") },
    { value: "unplayed", label: t("library.unplayed") },
    { value: "played", label: t("library.played") },
  ];
  const favoriteOptions: readonly { value: "" | "favorite"; label: string }[] = [
    { value: "", label: t("library.all") },
    { value: "favorite", label: t("library.favorite") },
  ];

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
      <button className={`back floating-back ${backVisible ? "" : "hidden"}`} onClick={onBack} aria-label={t("common.back")}><UiIcon name="chevron-left" /></button>
      <div className="library-heading">
        <h1>{title ?? payload.library.name}</h1>
        <div className="filters">
          <span>{t("library.itemsCount", { count: payload.totalCount })}</span>
          <FilterMenu label={t("library.type")} value={itemType} options={typeOptions} onChange={(value) => onOptionsChange(value, sortBy, sortOrder, filters)} />
          <FilterMenu label={t("library.sort")} value={sortBy} options={sortOptions} onChange={(value) => onOptionsChange(itemType, value, sortOrder, filters)} />
          <FilterMenu label={t("library.direction")} value={sortOrder} options={orderOptions} onChange={(value) => onOptionsChange(itemType, sortBy, value, filters)} />
          <FilterMenu label={t("library.watched")} value={filters.played ?? ""} options={playedOptions} onChange={(value) => onOptionsChange(itemType, sortBy, sortOrder, { ...filters, played: value })} />
          <FilterMenu label={t("library.favorites")} value={filters.favorite ? "favorite" : ""} options={favoriteOptions} onChange={(value) => onOptionsChange(itemType, sortBy, sortOrder, { ...filters, favorite: value === "favorite" || undefined })} />
        </div>
      </div>
      {!payload.items.length && !loadingMore && <EmptyState title={t("library.empty")} />}
      <div className={`poster-grid poster-density-${posterDensity}`} ref={gridRef}>
        {payload.items.map((item) => (
          <Poster key={`${item.serverId ?? ""}:${item.id}`} item={item} onOpen={onOpenItem} hideMeta onToggleFavorite={onToggleFavorite} onTogglePlayed={onTogglePlayed} />
        ))}
      </div>
      {loadingMore && <div className="loading-more">{t("library.loadingMore")}</div>}
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
        <UiIcon name="chevron-right" className="filter-menu-chevron" />
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
  onOpen: (id: string, serverId?: string | null) => void;
}) {
  const { t } = useTranslation();
  const trimmedQuery = query.trim();

  return (
    <div className="page search-page">
      <h1>{t("library.search")}</h1>
      <p className="search-count">
        {!trimmedQuery
          ? t("library.recentSearch")
          : loading
            ? t("library.searching")
            : results.length
              ? t("library.results", { count: results.length })
              : t("library.noResult", { query })}
      </p>
      {!trimmedQuery && recentTerms.length > 0 && (
        <div className="search-recents">
          {recentTerms.map((term) => (
            <button key={term} onClick={() => onUseRecentTerm(term)}>{term}</button>
          ))}
        </div>
      )}
      {!trimmedQuery && !recentTerms.length && <EmptyState title={t("library.noRecent")} />}
      {!loading && trimmedQuery && !results.length && <EmptyState title={t("library.tryAnother")} />}
      <div className={`poster-grid poster-density-${posterDensity} ${loading ? "is-loading" : ""}`}>
        {results.map((item) => (
          <Poster key={`${item.serverId ?? ""}:${item.id}`} item={item} onOpen={onOpen} />
        ))}
      </div>
    </div>
  );
}
