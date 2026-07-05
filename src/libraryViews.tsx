import { useEffect, useRef } from "react";
import type { LibraryItemType, LibraryPayload, LibrarySortBy, LibrarySortOrder, MediaItem } from "./types";
import { Poster, useFloatingBackVisible } from "./viewParts";

export function LibraryView({
  payload,
  loadingMore,
  onBack,
  onOpenItem,
  onLoadMore,
  itemType,
  sortBy,
  sortOrder,
  onOptionsChange,
}: {
  payload: LibraryPayload;
  loadingMore: boolean;
  onBack: () => void;
  onOpenItem: (id: string) => void;
  onLoadMore: () => void;
  itemType: LibraryItemType;
  sortBy: LibrarySortBy;
  sortOrder: LibrarySortOrder;
  onOptionsChange: (itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder) => void;
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
      {
        root: scroller,
        rootMargin: "700px 0px",
      }
    );
    observer.observe(target);
    return () => observer.disconnect();
  }, [loadingMore, onLoadMore, payload.hasMore, payload.items.length]);

  return (
    <div className="page library-page">
      <button className={`back floating-back ${backVisible ? "" : "hidden"}`} onClick={onBack} aria-label="返回" />
      <div className="library-heading">
        <h1>{payload.library.name}</h1>
        <div className="filters">
          <span>{payload.totalCount} 项</span>
          <select value={itemType} onChange={(event) => onOptionsChange(event.target.value as LibraryItemType, sortBy, sortOrder)}>
            <option value="">全部</option>
            <option value="Movie">电影</option>
            <option value="Series">剧集</option>
            <option value="Episode">单集</option>
            <option value="Video">视频</option>
          </select>
          <select value={sortBy} onChange={(event) => onOptionsChange(itemType, event.target.value as LibrarySortBy, sortOrder)}>
            <option value="DateCreated">最近添加</option>
            <option value="SortName">名称</option>
            <option value="PremiereDate">首播日期</option>
            <option value="CommunityRating">评分</option>
          </select>
          <select value={sortOrder} onChange={(event) => onOptionsChange(itemType, sortBy, event.target.value as LibrarySortOrder)}>
            <option value="Descending">降序</option>
            <option value="Ascending">升序</option>
          </select>
        </div>
      </div>
      {!payload.items.length && !loadingMore && <div className="empty-panel">这个媒体库暂时没有可显示的项目</div>}
      <div className="poster-grid" ref={gridRef}>
        {payload.items.map((item) => (
          <Poster key={item.id} item={item} onOpen={onOpenItem} />
        ))}
      </div>
      {loadingMore && <div className="loading-more">加载中...</div>}
    </div>
  );
}

export function SearchOverlay({
  results,
  query,
  loading,
  onOpen,
}: {
  results: MediaItem[];
  query: string;
  loading: boolean;
  onOpen: (id: string) => void;
}) {
  return (
    <div className="page search-page">
      <h1>搜索</h1>
      <p className="search-count">
        {loading ? "搜索中..." : results.length ? `找到 ${results.length} 个结果` : `没有找到“${query}”`}
      </p>
      {loading && <div className="search-loading">正在查询媒体库</div>}
      {!loading && !results.length && <div className="empty-panel">换个关键词试试</div>}
      <div className="poster-grid">
        {results.map((item) => (
          <Poster key={item.id} item={item} onOpen={onOpen} />
        ))}
      </div>
    </div>
  );
}
