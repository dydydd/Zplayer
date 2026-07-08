import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { HomePayload, MediaItem, SavedServer } from "./types";
import { bg, itemMeta } from "./media";
import { rotateDaily } from "./viewLogic";
import { EmptyState, Image, ScrollableStage, ShelfHeader } from "./viewParts";

export function HomeView({
  home,
  activeServer,
  servers,
  onAddServer,
  onOpenServers,
  onOpenSettings,
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
  onAddServer: () => void;
  onOpenServers: () => void;
  onOpenSettings: () => void;
  onActivateServer: (id: string) => Promise<void>;
  onOpenLibrary: (id: string) => void;
  onOpenItem: (id: string) => void;
  onPlay: (id: string) => Promise<void>;
  serverMenuOpen: boolean;
  setServerMenuOpen: (open: boolean) => void;
  chromeVisible: boolean;
}) {
  const [heroIndex, setHeroIndex] = useState(0);
  const longPressTimer = useRef<number | null>(null);
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
    if (heroItems.length < 2) {
      return;
    }
    const timer = window.setInterval(() => {
      setHeroIndex((index) => (index + 1) % heroItems.length);
    }, 6000);
    return () => window.clearInterval(timer);
  }, [heroItems.length]);

  if (!activeServer) {
    return (
      <div className="empty">
        <h1>Zplayer</h1>
        <p>先添加一个 Emby 或 Jellyfin 服务器。</p>
        <button onClick={onAddServer}>添加服务器</button>
      </div>
    );
  }

  return (
    <div className="page home-page">
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
          title="切换媒体服务器"
        >
          <span className="home-icon play-icon" />
          {activeServer.name}
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
                {server.name}
              </button>
            ))}
            <button onClick={() => {
              setServerMenuOpen(false);
              onAddServer();
            }}>添加服务器</button>
            <button onClick={() => {
              setServerMenuOpen(false);
              onOpenServers();
            }}>服务器管理</button>
            <button onClick={() => {
              setServerMenuOpen(false);
              onOpenSettings();
            }}>设置</button>
          </div>
        )}
      </div>
      <section className="feature-banner">
        {featuredImage && <img key={featured?.id} className="feature-art active" src={featuredImage} alt="" loading="eager" decoding="async" />}
        {featured && (
          <div key={featured.id} className="feature-copy">
            {featured.logoUrl ? (
              <img className="feature-logo" src={featured.logoUrl} alt={featured.name} />
            ) : (
              <strong>{featured.name}</strong>
            )}
            <small>{itemMeta(featured)}</small>
            {featured.overview && <p>{featured.overview}</p>}
            <div className="feature-actions">
              <button className="feature-play" onClick={() => void onPlay(featured.id)}><span className="play-glyph" />播放</button>
              <button className="round-icon info-icon" onClick={() => onOpenItem(featured.id)} aria-label="查看详情" />
              <button className="round-icon add-icon" />
              <button className="round-icon next-icon" onClick={() => setHeroIndex((index) => (index + 1) % Math.max(heroItems.length, 1))} aria-label="下一张推荐" />
            </div>
          </div>
        )}
          <div className="hero-dots">
            {heroItems.slice(0, 7).map((item, index) => (
              <button
                key={item.id}
                className={index === heroIndex % Math.max(heroItems.length, 1) ? "active" : ""}
                onClick={() => setHeroIndex(index)}
                aria-label={`切换到第 ${index + 1} 张推荐`}
              />
            ))}
          </div>
          <MediaShelf
            title="继续播放"
            items={home?.resumeItems ?? []}
            onOpenItem={onOpenItem}
            className="hero-shelf"
            showProgress
          />
      </section>
      {home && !home.resumeItems.length && !home.libraries.length && !home.libraryLatest.length && (
        <EmptyState title="这个服务器暂时没有可显示的媒体" onAction={onOpenServers} actionLabel="服务器管理" />
      )}
      <div className="home-shelves">
        {home?.libraries.length ? (
          <LibraryShelf libraries={home.libraries} onOpenLibrary={onOpenLibrary} />
        ) : null}
        {(home?.libraryLatest ?? []).map((row) => (
          <MediaShelf
            key={row.library.id}
            title={row.library.name}
            items={row.items}
            onOpenItem={onOpenItem}
            libraryId={row.library.id}
            onOpenLibrary={onOpenLibrary}
            floatingControls
            poster
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
  return (
    <section className="home-shelf">
      <ShelfHeader title="媒体库" showControls={false} />
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
}: {
  title: string;
  items: MediaItem[];
  onOpenItem: (id: string) => void;
  libraryId?: string;
  onOpenLibrary?: (id: string) => void;
  className?: string;
  floatingControls?: boolean;
  poster?: boolean;
  showProgress?: boolean;
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
          {items.map((item) => (
            <button key={item.id} className={`apple-card ${poster ? "poster-card" : ""} ${showProgress ? "with-progress" : ""}`} onClick={() => onOpenItem(item.id)}>
              <Image src={poster ? item.primaryImageUrl : item.backdropUrl ?? item.primaryImageUrl} alt={item.name} />
              {item.communityRating && <span className="score">{item.communityRating.toFixed(1)}</span>}
              <strong>{item.name}</strong>
              {showProgress && <ProgressBar item={item} />}
            </button>
          ))}
      </ScrollableStage>
    </section>
  );
});

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
