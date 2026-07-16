import { memo, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type { MediaItem } from "./types";
import { UiIcon } from "./icons";
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
  const { t } = useTranslation();
  return <div className="loading-page" aria-label={t("parts.loading")} />;
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
  const { t } = useTranslation();
  const titleContent = (
    <>
      {title}
      {libraryId && onOpenLibrary && <UiIcon name="chevron-right" className="enter-arrow" />}
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
          <button className="shelf-arrow left" onClick={onScrollLeft} aria-label={t("parts.scrollLeft")}><UiIcon name="chevron-left" /></button>
          <button className="shelf-arrow right" onClick={onScrollRight} aria-label={t("parts.scrollRight")}><UiIcon name="chevron-right" /></button>
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
  const { t } = useTranslation();
  const rowRef = useRef<HTMLDivElement | null>(null);
  const [canScrollLeft, setCanScrollLeft] = useState(false);
  const [canScrollRight, setCanScrollRight] = useState(false);
  const scrollStateRef = useRef({ left: false, right: false });
  const scrollFrameRef = useRef<number | null>(null);

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

  function scheduleScrollState(persistKey?: string) {
    if (scrollFrameRef.current !== null) return;
    scrollFrameRef.current = window.requestAnimationFrame(() => {
      scrollFrameRef.current = null;
      const row = rowRef.current;
      if (!row) return;
      if (persistKey) {
        sessionStorage.setItem(persistKey, String(row.scrollLeft));
      }
      updateScrollState();
    });
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
    const storageKey = scrollKey ? `row-scroll:${scrollKey}` : undefined;
    function handleScroll() {
      scheduleScrollState(storageKey);
    }
    row.addEventListener("scroll", handleScroll, { passive: true });
    window.addEventListener("resize", handleScroll);
    return () => {
      if (scrollFrameRef.current !== null) {
        window.cancelAnimationFrame(scrollFrameRef.current);
        scrollFrameRef.current = null;
      }
      row.removeEventListener("scroll", handleScroll);
      window.removeEventListener("resize", handleScroll);
    };
  }, [itemCount, scrollKey]);

  return (
    <div className={`row-stage ${className}`}>
      {floatingControls && canScrollLeft && <button className="row-float-arrow left" onClick={() => scrollByPage(-1)} aria-label={t("parts.scrollLeft")}><UiIcon name="chevron-left" /></button>}
      <div className={rowClassName} ref={rowRef} onMouseEnter={updateScrollState}>
        {children}
      </div>
      {floatingControls && canScrollRight && <button className="row-float-arrow right" onClick={() => scrollByPage(1)} aria-label={t("parts.scrollRight")}><UiIcon name="chevron-right" /></button>}
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
  onOpen: (id: string, serverId?: string | null) => void;
  hideMeta?: boolean;
  onToggleFavorite?: (item: MediaItem) => void;
  onTogglePlayed?: (item: MediaItem) => void;
}) {
  const { t } = useTranslation();
  const meta = [itemMeta(item), item.serverName].filter(Boolean).join(" / ");
  return (
    <article className="poster">
      <button className="poster-main" onClick={() => onOpen(item.id, item.serverId)}>
        <span className="poster-cover">
          <Image src={item.primaryImageUrl} alt={item.name} />
          {item.communityRating && <span className="score">{item.communityRating.toFixed(1)}</span>}
        </span>
        <strong>{item.name}</strong>
        {!hideMeta && <small>{meta}</small>}
      </button>
      {(onToggleFavorite || onTogglePlayed) && (
        <div className="poster-actions">
          {onToggleFavorite && <button type="button" onClick={() => onToggleFavorite(item)}>{item.favorite ? t("detail.favorited") : t("detail.favorite")}</button>}
          {onTogglePlayed && <button type="button" onClick={() => onTogglePlayed(item)}>{item.played ? t("detail.watched") : t("parts.posterMarked")}</button>}
        </div>
      )}
    </article>
  );
});

export function Image({
  src,
  alt,
  loading = "lazy",
  fetchPriority = "auto",
}: {
  src?: string | null;
  alt: string;
  loading?: "eager" | "lazy";
  fetchPriority?: "high" | "low" | "auto";
}) {
  const imageRef = useRef<HTMLImageElement | null>(null);
  const [readySrc, setReadySrc] = useState<string | null>(null);
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  const ready = !!src && readySrc === src;
  const failed = !!src && failedSrc === src;

  useEffect(() => {
    setReadySrc(null);
    setFailedSrc(null);
    if (imageRef.current?.complete) {
      setReadySrc(src ?? null);
    }
  }, [src]);

  if (!src || failed) {
    return <div className="image-fallback">{alt.slice(0, 2)}</div>;
  }

  return (
    <img
      ref={imageRef}
      className={`async-image ${ready ? "image-ready" : ""}`}
      src={src}
      alt={alt}
      loading={loading}
      decoding="async"
      fetchPriority={fetchPriority}
      onLoad={() => setReadySrc(src)}
      onError={() => setFailedSrc(src)}
    />
  );
}
