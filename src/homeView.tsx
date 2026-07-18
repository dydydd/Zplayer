import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import { useTranslation } from "react-i18next";
import type { HomePayload, MediaItem, SavedServer } from "./types";
import { UiIcon } from "./icons";
import { bg, itemMeta } from "./media";
import { ServerAvatar } from "./ServerAvatar";
import { serverIconCatalogUrls, useServerIconEntries } from "./serverIcons";
import { rotateDaily } from "./viewLogic";
import { EmptyState, Image, ScrollableStage, ShelfHeader } from "./viewParts";

const HOME_PRELOAD_IMAGE_LIMIT = 28;
const HOME_PRELOAD_BATCH_SIZE = 3;
const HOME_PRELOAD_START_DELAY = 500;
const HOME_ROW_EAGER_IMAGES = 2;
const HOME_EAGER_LIBRARY_ROW_LIMIT = 1;
const HERO_ROW_EAGER_IMAGES = 5;

type IdleDeadlineLike = {
  didTimeout?: boolean;
  timeRemaining: () => number;
};

type IdleWindow = Window & typeof globalThis & {
  requestIdleCallback?: (callback: (deadline: IdleDeadlineLike) => void, options?: { timeout: number }) => number;
  cancelIdleCallback?: (handle: number) => void;
};

export function HomeView({
  home,
  activeServer,
  servers,
  serverIconCatalogUrls: iconCatalogUrls,
  onAddServer,
  onOpenServers,
  onOpenSettings,
  onOpenCalendar,
  onOpenFavorites,
  onActivateServer,
  onOpenLibrary,
  onOpenItem,
  onPlay,
  serverMenuOpen,
  setServerMenuOpen,
  chromeVisible,
}: {
  home: HomePayload | null;
  activeServer: SavedServer | null;
  servers: SavedServer[];
  serverIconCatalogUrls: string;
  onAddServer: () => void;
  onOpenServers: () => void;
  onOpenSettings: () => void;
  onOpenCalendar: () => void;
  onOpenFavorites: () => void;
  onActivateServer: (id: string) => Promise<void>;
  onOpenLibrary: (id: string) => void;
  onOpenItem: (id: string, serverId?: string | null) => void;
  onPlay: (id: string, serverId?: string | null) => Promise<void>;
  serverMenuOpen: boolean;
  setServerMenuOpen: (open: boolean) => void;
  chromeVisible: boolean;
}) {
  const { t } = useTranslation();
  const serverIcons = useServerIconEntries(serverIconCatalogUrls(iconCatalogUrls));
  const [heroIndex, setHeroIndex] = useState(0);
  const [visibleFeaturedImage, setVisibleFeaturedImage] = useState<string | null | undefined>(undefined);
  const longPressTimer = useRef<number | null>(null);
  const preloadedImages = useRef<HTMLImageElement[]>([]);
  const currentServer = activeServer ?? home?.server ?? null;
  const heroItems = useMemo(() => {
    const recommended = [...(home?.recommendedMovies ?? []), ...(home?.recommendedShows ?? [])];
    return rotateDaily(
      recommended.filter((item) => item.backdropUrl || item.primaryImageUrl),
      home?.server.id ?? "",
    ).slice(0, 7);
  }, [home?.recommendedMovies, home?.recommendedShows, home?.server.id]);
  const featured = heroItems[heroIndex % Math.max(heroItems.length, 1)] ?? home?.latest[0];
  const featuredImage = featured?.backdropUrl ?? featured?.primaryImageUrl;

  function startServerLongPress() {
    longPressTimer.current = window.setTimeout(() => setServerMenuOpen(true), 420);
  }

  function endServerLongPress() {
    if (longPressTimer.current) {
      window.clearTimeout(longPressTimer.current);
      longPressTimer.current = null;
    }
  }

  useEffect(() => {
    setHeroIndex(0);
  }, [home?.server.id]);

  useEffect(() => {
    if (!featuredImage) {
      setVisibleFeaturedImage(undefined);
      return;
    }

    let cancelled = false;
    const image = new window.Image();
    const showImage = () => {
      if (!cancelled) setVisibleFeaturedImage(featuredImage);
    };

    image.decoding = "async";
    image.onload = showImage;
    image.onerror = showImage;
    image.src = featuredImage;
    if (image.complete) showImage();

    return () => {
      cancelled = true;
      image.onload = null;
      image.onerror = null;
    };
  }, [featuredImage]);

  useEffect(() => {
    if (!home) {
      preloadedImages.current = [];
      return;
    }

    return scheduleHomeImagePreloads(collectHomePreloadUrls(home, heroItems), preloadedImages);
  }, [heroItems, home]);

  useEffect(() => {
    if (heroItems.length < 2) {
      return;
    }
    const timer = window.setInterval(() => {
      setHeroIndex((index) => (index + 1) % heroItems.length);
    }, 6000);
    return () => window.clearInterval(timer);
  }, [heroItems.length]);

  if (!currentServer) {
    return (
      <div className="empty">
        <h1>Zplayer</h1>
        <p>{t("home.emptyServer")}</p>
        <button onClick={onAddServer}>{t("home.addServer")}</button>
      </div>
    );
  }

  if (!home) {
    return <HomeSkeleton server={currentServer} serverIcons={serverIcons} chromeVisible={chromeVisible} />;
  }

  return (
    <div className="page home-page" aria-busy={false}>
      <div className={`home-server-switch chrome-float ${chromeVisible ? "" : "hidden"}`}>
        <button
          onClick={() => setServerMenuOpen(!serverMenuOpen)}
          onMouseDown={startServerLongPress}
          onMouseUp={endServerLongPress}
          onMouseLeave={endServerLongPress}
          onContextMenu={(event) => {
            event.preventDefault();
            setServerMenuOpen(true);
          }}
          title={t("home.serverSwitchTitle")}
        >
          <ServerAvatar server={currentServer} icons={serverIcons} className="home-icon" />
          <span className="home-server-name">{currentServer.name}</span>
        </button>
        {serverMenuOpen && (
          <div className="server-popover">
            {servers.map((server) => (
              <button
                key={server.id}
                className={server.active ? "active" : ""}
                onClick={() => {
                  setServerMenuOpen(false);
                  void onActivateServer(server.id);
                }}
              >
                <ServerAvatar server={server} icons={serverIcons} className="server-popover-icon" />
                <span>{server.name}</span>
              </button>
            ))}
            <button onClick={() => {
              setServerMenuOpen(false);
              onAddServer();
            }}>{t("home.addServer")}</button>
            <button onClick={() => {
              setServerMenuOpen(false);
              onOpenServers();
            }}>{t("home.serverManage")}</button>
            <button onClick={() => {
              setServerMenuOpen(false);
              onOpenCalendar();
            }}>{t("home.calendar")}</button>
            <button onClick={() => {
              setServerMenuOpen(false);
              onOpenFavorites();
            }}>{t("home.favorites")}</button>
            <button onClick={() => {
              setServerMenuOpen(false);
              onOpenSettings();
            }}>{t("home.settings")}</button>
          </div>
        )}
      </div>
      <section className="feature-banner">
        {visibleFeaturedImage && <img key={visibleFeaturedImage} className="feature-art active" src={visibleFeaturedImage} alt="" loading="eager" decoding="async" fetchPriority="high" />}
        {featured && (
          <div key={featured.id} className="feature-copy">
            {featured.logoUrl ? (
              <img className="feature-logo" src={featured.logoUrl} alt={featured.name} loading="eager" decoding="async" fetchPriority="high" />
            ) : (
              <strong>{featured.name}</strong>
            )}
            <small>{itemMeta(featured)}</small>
            {featured.overview && <p>{featured.overview}</p>}
            <div className="feature-actions">
              <button className="feature-play" onClick={() => void onPlay(featured.id, featured.serverId)}><UiIcon name="play" className="play-glyph" />{t("home.play")}</button>
              <button className="round-icon info-icon" onClick={() => onOpenItem(featured.id, featured.serverId)} aria-label={t("home.detail")}><UiIcon name="info" /></button>
              <button className="round-icon add-icon" onClick={onOpenFavorites} aria-label={t("home.openFavorites")}><UiIcon name="heart" /></button>
              <button className="round-icon next-icon" onClick={() => setHeroIndex((index) => (index + 1) % Math.max(heroItems.length, 1))} aria-label={t("home.nextRecommendation")}><UiIcon name="chevron-right" /></button>
            </div>
          </div>
        )}
          <div className="hero-dots">
            {heroItems.slice(0, 7).map((item, index) => (
              <button
                key={`${item.serverId ?? ""}:${item.id}`}
                className={index === heroIndex % Math.max(heroItems.length, 1) ? "active" : ""}
                onClick={() => setHeroIndex(index)}
                aria-label={t("home.recommendationDot", { index: index + 1 })}
              />
            ))}
          </div>
          <MediaShelf
            title={t("home.continueWatching")}
            items={home?.resumeItems ?? []}
            onOpenItem={onOpenItem}
            className="hero-shelf"
            showProgress
            eagerImageCount={HERO_ROW_EAGER_IMAGES}
          />
      </section>
      {home && !home.resumeItems.length && !home.libraries.length && !home.libraryLatest.length && (
        <EmptyState title={t("home.noMedia")} onAction={onOpenServers} actionLabel={t("home.serverManage")} />
      )}
      <div className="home-shelves">
        {home?.libraries.length ? (
          <LibraryShelf libraries={home.libraries} onOpenLibrary={onOpenLibrary} />
        ) : null}
        {(home?.libraryLatest ?? []).map((row, rowIndex) => (
          <MediaShelf
            key={row.library.id}
            title={row.library.name}
            items={row.items}
            onOpenItem={onOpenItem}
            libraryId={row.library.id}
            onOpenLibrary={onOpenLibrary}
            floatingControls
            poster
            eagerImageCount={rowIndex < HOME_EAGER_LIBRARY_ROW_LIMIT ? HOME_ROW_EAGER_IMAGES : 0}
          />
        ))}
      </div>
    </div>
  );
}

const LibraryShelf = memo(function LibraryShelf({
  libraries,
  onOpenLibrary,
}: {
  libraries: HomePayload["libraries"];
  onOpenLibrary: (id: string) => void;
}) {
  const { t } = useTranslation();

  return (
    <section className="home-shelf">
      <ShelfHeader title={t("home.libraries")} showControls={false} />
      <ScrollableStage rowClassName="media-row" itemCount={libraries.length} scrollKey="home:libraries">
          {libraries.map((library) => (
            <button key={library.id} className="library-tile" style={bg(library.imageUrl)} onClick={() => onOpenLibrary(library.id)}>
              {library.name}
            </button>
          ))}
      </ScrollableStage>
    </section>
  );
});

function HomeSkeleton({
  server,
  serverIcons,
  chromeVisible,
}: {
  server: SavedServer;
  serverIcons: ReturnType<typeof useServerIconEntries>;
  chromeVisible: boolean;
}) {
  const { t } = useTranslation();
  return (
    <div className="page home-page home-loading" aria-busy="true">
      <div className={`home-server-switch chrome-float ${chromeVisible ? "" : "hidden"}`}>
        <button type="button" disabled title={t("home.serverSwitchTitle")}>
          <ServerAvatar server={server} icons={serverIcons} className="home-icon" />
          <span className="home-server-name">{server.name}</span>
        </button>
      </div>
      <section className="feature-banner home-hero-skeleton">
        <div className="feature-copy home-skeleton-copy">
          <span className="home-skeleton-line title" />
          <span className="home-skeleton-line meta" />
          <span className="home-skeleton-line body" />
          <span className="home-skeleton-line body short" />
          <div className="feature-actions home-skeleton-actions">
            <span className="home-skeleton-pill" />
            <span className="home-skeleton-circle" />
            <span className="home-skeleton-circle" />
          </div>
        </div>
        <div className="hero-dots home-skeleton-dots" aria-hidden="true">
          {Array.from({ length: 5 }).map((_, index) => (
            <span key={index} />
          ))}
        </div>
        <SkeletonShelf className="hero-shelf" itemCount={5} />
      </section>
      <div className="home-shelves">
        <SkeletonShelf itemCount={6} />
        <SkeletonShelf itemCount={6} poster />
      </div>
    </div>
  );
}

function SkeletonShelf({
  className = "",
  itemCount,
  poster = false,
}: {
  className?: string;
  itemCount: number;
  poster?: boolean;
}) {
  return (
    <section className={`home-shelf home-skeleton-shelf ${className}`}>
      <div className="shelf-header">
        <span className="home-skeleton-line shelf-title" />
      </div>
      <div className="row-stage">
        <div className="media-row" aria-hidden="true">
          {Array.from({ length: itemCount }).map((_, index) => (
            <span key={index} className={`home-skeleton-card ${poster ? "poster-card" : "apple-card"}`} />
          ))}
        </div>
      </div>
    </section>
  );
}

const MediaShelf = memo(function MediaShelf({
  title,
  items,
  onOpenItem,
  libraryId,
  onOpenLibrary,
  className = "",
  floatingControls = true,
  poster = false,
  showProgress = false,
  eagerImageCount = 0,
}: {
  title: string;
  items: MediaItem[];
  onOpenItem: (id: string, serverId?: string | null) => void;
  libraryId?: string;
  onOpenLibrary?: (id: string) => void;
  className?: string;
  floatingControls?: boolean;
  poster?: boolean;
  showProgress?: boolean;
  eagerImageCount?: number;
}) {
  if (!items.length) return null;

  return (
    <section className={`home-shelf ${className}`}>
      <ShelfHeader
        title={title}
        libraryId={libraryId}
        onOpenLibrary={onOpenLibrary}
        showControls={!floatingControls}
      />
      <ScrollableStage rowClassName="media-row" itemCount={items.length} floatingControls={floatingControls} scrollKey={`home:${libraryId ?? title}`}>
          {items.map((item, index) => (
            <button key={`${item.serverId ?? ""}:${item.id}`} className={`apple-card ${poster ? "poster-card" : ""} ${showProgress ? "with-progress" : ""}`} onClick={() => onOpenItem(item.id, item.serverId)}>
              <Image
                src={poster ? item.primaryImageUrl : item.backdropUrl ?? item.primaryImageUrl}
                alt={item.name}
                loading={index < eagerImageCount ? "eager" : "lazy"}
                fetchPriority={index < Math.min(eagerImageCount, 4) ? "high" : "low"}
              />
              {item.communityRating && <span className="score">{item.communityRating.toFixed(1)}</span>}
              <strong>{item.name}</strong>
              {showProgress && <ProgressBar item={item} />}
            </button>
          ))}
      </ScrollableStage>
    </section>
  );
});

function collectHomePreloadUrls(home: HomePayload, heroItems: MediaItem[]) {
  const urls: string[] = [];
  const seen = new Set<string>();
  const add = (url?: string | null) => {
    if (!url || seen.has(url) || urls.length >= HOME_PRELOAD_IMAGE_LIMIT) return;
    seen.add(url);
    urls.push(url);
  };

  heroItems.slice(0, 7).forEach((item) => {
    add(item.backdropUrl ?? item.primaryImageUrl);
    add(item.logoUrl);
  });
  home.latest.slice(0, 4).forEach((item) => add(item.backdropUrl ?? item.primaryImageUrl));
  home.resumeItems.slice(0, 10).forEach((item) => add(item.backdropUrl ?? item.primaryImageUrl));
  home.libraries.slice(0, 8).forEach((library) => add(library.imageUrl));
  home.libraryLatest.slice(0, 5).forEach((row, rowIndex) => {
    row.items.slice(0, rowIndex < 2 ? 8 : 4).forEach((item) => add(item.primaryImageUrl ?? item.backdropUrl));
  });

  return urls;
}

function scheduleHomeImagePreloads(urls: string[], target: MutableRefObject<HTMLImageElement[]>) {
  const idleWindow = window as IdleWindow;
  let cancelled = false;
  let cursor = 0;
  let startTimer: number | null = null;
  let idleHandle: number | null = null;
  let fallbackTimer: number | null = null;

  target.current = [];

  const preloadOne = () => {
    const src = urls[cursor];
    cursor += 1;
    const image = new window.Image();
    image.decoding = "async";
    image.fetchPriority = "low";
    image.loading = "lazy";
    image.src = src;
    target.current.push(image);
  };

  const queueNextBatch = () => {
    if (cancelled || cursor >= urls.length) return;
    if (idleWindow.requestIdleCallback) {
      idleHandle = idleWindow.requestIdleCallback(runBatch, { timeout: 1600 });
      return;
    }
    fallbackTimer = window.setTimeout(() => runBatch({ timeRemaining: () => 8 }), 140);
  };

  const runBatch = (deadline: IdleDeadlineLike) => {
    idleHandle = null;
    fallbackTimer = null;
    if (cancelled) return;

    let count = 0;
    while (
      cursor < urls.length
      && count < HOME_PRELOAD_BATCH_SIZE
      && (deadline.didTimeout || deadline.timeRemaining() > 2 || count === 0)
    ) {
      preloadOne();
      count += 1;
    }
    queueNextBatch();
  };

  startTimer = window.setTimeout(queueNextBatch, HOME_PRELOAD_START_DELAY);

  return () => {
    cancelled = true;
    target.current = [];
    if (startTimer !== null) window.clearTimeout(startTimer);
    if (idleHandle !== null) idleWindow.cancelIdleCallback?.(idleHandle);
    if (fallbackTimer !== null) window.clearTimeout(fallbackTimer);
  };
}

function ProgressBar({ item }: { item: MediaItem }) {
  const percent = item.playedPercentage ?? (
    item.playbackPositionTicks && item.runTimeTicks
      ? (item.playbackPositionTicks / item.runTimeTicks) * 100
      : 0
  );
  const width = Math.min(Math.max(percent, 0), 100);
  if (!width) return null;
  return <span className="watch-progress"><span style={{ width: `${width}%` }} /></span>;
}
