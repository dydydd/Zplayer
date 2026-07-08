import { memo, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import type { MediaItem } from "./types";
import { itemMeta } from "./media";

export function useFloatingBackVisible(resetKey: string) {
  const [backVisible, setBackVisible] = useState(true);
  const lastScrollTop = useRef(0);
  const backVisibleRef = useRef(true);

  useEffect(() => {
    const scroller = document.querySelector<HTMLElement>(".workspace");
    if (!scroller) return;
    scroller.scrollTo({ top: 0 });
    lastScrollTop.current = 0;
    backVisibleRef.current = true;
    setBackVisible(true);
  }, [resetKey]);

  useEffect(() => {
    const scroller = document.querySelector<HTMLElement>(".workspace");
    if (!scroller) return;
    const element = scroller;

    function handleScroll() {
      const nextScrollTop = element.scrollTop;
      const scrollingUp = nextScrollTop < lastScrollTop.current;
      const nextVisible = nextScrollTop < 24 || scrollingUp;
      if (backVisibleRef.current !== nextVisible) {
        backVisibleRef.current = nextVisible;
        setBackVisible(nextVisible);
      }
      lastScrollTop.current = nextScrollTop;
    }

    element.addEventListener("scroll", handleScroll, { passive: true });
    handleScroll();
    return () => element.removeEventListener("scroll", handleScroll);
  }, []);

  return backVisible;
}

export function LoadingPage() {
  return <div className="loading-page" aria-label="加载中" />;
}

export function EmptyState({ title, actionLabel, onAction }: { title: string; actionLabel?: string; onAction?: () => void }) {
  return (
    <div className="empty-panel">
      <strong>{title}</strong>
      {actionLabel && onAction && <button onClick={onAction}>{actionLabel}</button>}
    </div>
  );
}

export function ShelfHeader({
  title,
  libraryId,
  onOpenLibrary,
  onScrollLeft,
  onScrollRight,
  showControls = true,
}: {
  title: string;
  libraryId?: string;
  onOpenLibrary?: (id: string) => void;
  onScrollLeft?: () => void;
  onScrollRight?: () => void;
  showControls?: boolean;
}) {
  const titleContent = (
    <>
      {title}
      {libraryId && onOpenLibrary && <span className="enter-arrow" aria-hidden="true" />}
    </>
  );

  return (
    <div className="shelf-header">
      {libraryId && onOpenLibrary ? (
        <button className="shelf-title-button" onClick={() => onOpenLibrary(libraryId)}>
          {titleContent}
        </button>
      ) : (
        <h2>{titleContent}</h2>
      )}
      {showControls && (
        <div className="shelf-actions">
          <button className="shelf-arrow left" onClick={onScrollLeft} aria-label="向左滚动" />
          <button className="shelf-arrow right" onClick={onScrollRight} aria-label="向右滚动" />
        </div>
      )}
    </div>
  );
}

export function ScrollableStage({
  className = "",
  rowClassName,
  itemCount,
  floatingControls = true,
  scrollToIndex,
  scrollKey,
  children,
}: {
  className?: string;
  rowClassName: string;
  itemCount: number;
  floatingControls?: boolean;
  scrollToIndex?: number;
  scrollKey?: string;
  children: ReactNode;
}) {
  const rowRef = useRef<HTMLDivElement | null>(null);
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);
  const scrollStateRef = useRef({ left: false, right: false });

  function updateScrollState() {
    const row = rowRef.current;
    if (!row) return;
    const nextLeft = row.scrollLeft > 2;
    const nextRight = row.scrollLeft + row.clientWidth < row.scrollWidth - 2;
    if (scrollStateRef.current.left === nextLeft && scrollStateRef.current.right === nextRight) return;
    scrollStateRef.current = { left: nextLeft, right: nextRight };
    setCanScrollLeft(nextLeft);
    setCanScrollRight(nextRight);
  }

  function scrollByPage(direction: -1 | 1) {
    rowRef.current?.scrollBy({ left: direction * rowRef.current.clientWidth * 0.86, behavior: "smooth" });
  }

  useEffect(() => {
    if (scrollToIndex === undefined) return;
    const row = rowRef.current;
    const target = row?.children.item(scrollToIndex) as HTMLElement | null;
    target?.scrollIntoView({ behavior: "smooth", block: "nearest", inline: "center" });
  }, [scrollToIndex]);

  useEffect(() => {
    const row = rowRef.current;
    if (!row) return;
    if (scrollKey) {
      row.scrollLeft = Number(sessionStorage.getItem(`row-scroll:${scrollKey}`) ?? 0);
    }
    updateScrollState();
    function handleScroll() {
      const currentRow = rowRef.current;
      if (scrollKey && currentRow) sessionStorage.setItem(`row-scroll:${scrollKey}`, String(currentRow.scrollLeft));
      updateScrollState();
    }
    row.addEventListener("scroll", handleScroll, { passive: true });
    window.addEventListener("resize", updateScrollState);
    return () => {
      row.removeEventListener("scroll", handleScroll);
      window.removeEventListener("resize", updateScrollState);
    };
  }, [itemCount, scrollKey]);

  return (
    <div className={`row-stage ${className}`}>
      {floatingControls && canScrollLeft && <button className="row-float-arrow left" onClick={() => scrollByPage(-1)} aria-label="向左滚动" />}
      <div className={rowClassName} ref={rowRef} onMouseEnter={updateScrollState}>
        {children}
      </div>
      {floatingControls && canScrollRight && <button className="row-float-arrow right" onClick={() => scrollByPage(1)} aria-label="向右滚动" />}
    </div>
  );
}

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
});

export function Image({ src, alt }: { src?: string | null; alt: string }) {
  return src ? <img src={src} alt={alt} loading="lazy" decoding="async" /> : <div className="image-fallback">{alt.slice(0, 2)}</div>;
}
