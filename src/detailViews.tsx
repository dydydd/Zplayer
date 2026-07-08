import { useEffect, useMemo, useRef, useState } from "react";
import { ipc } from "./ipc";
import type { ItemDetailPayload, MediaVersion, PlaybackCommand, PlaybackState, StreamInfo } from "./types";
import { bg, episodeLabel } from "./media";
import { formatTime, mediaVersionFacts } from "./viewLogic";
import { Image, Poster, ScrollableStage, useFloatingBackVisible } from "./viewParts";

type IconName = "play" | "heart" | "check" | "back" | "close" | "min" | "max" | "fullscreen" | "pause" | "next" | "captions" | "more" | "volume";
type DetailPicker = "source" | "quality" | "audio" | "subtitle";

function SvgIcon({ name }: { name: IconName }) {
  return (
    <svg className="ui-svg" viewBox="0 0 24 24" aria-hidden="true">
      {name === "play" && <path d="M8 5.5v13l10-6.5-10-6.5Z" fill="currentColor" />}
      {name === "heart" && <path d="M12 20s-7-4.4-8.8-9.1C1.9 7.5 4.2 4.5 7.5 4.5c1.9 0 3.4 1 4.5 2.5 1.1-1.5 2.6-2.5 4.5-2.5 3.3 0 5.6 3 4.3 6.4C19 15.6 12 20 12 20Z" fill="currentColor" />}
      {name === "check" && <path d="m5 12.2 4.1 4.1L19.5 6.5" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />}
      {name === "back" && <path d="m15 5-7 7 7 7" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" />}
      {name === "close" && <path d="m6.5 6.5 11 11m0-11-11 11" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" />}
      {name === "min" && <path d="M7 12h10" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" />}
      {name === "max" && <path d="M7.5 7.5h9v9h-9z" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinejoin="round" />}
      {name === "fullscreen" && <path d="M8.5 4.5h-4v4m11-4h4v4m0 7v4h-4m-7 0h-4v-4" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" />}
      {name === "pause" && <path d="M8 6h3v12H8zm5 0h3v12h-3z" fill="currentColor" />}
      {name === "next" && <path d="m9 5 7 7-7 7" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" />}
      {name === "captions" && <path d="M5 6.5h14a1.5 1.5 0 0 1 1.5 1.5v8a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 16V8A1.5 1.5 0 0 1 5 6.5Zm2.5 4h3m-3 3h5m3.5 0h.5" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" />}
      {name === "more" && <path d="M6.5 12h.01M12 12h.01M17.5 12h.01" fill="none" stroke="currentColor" strokeWidth="3.4" strokeLinecap="round" />}
      {name === "volume" && <path d="M4.5 9.5h3.2L12 6v12l-4.3-3.5H4.5v-5Zm10.4-.8a5 5 0 0 1 0 6.6m2.6-9.2a9 9 0 0 1 0 11.8" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />}
    </svg>
  );
}

export function DetailView({
  payload,
  entryItemId,
  onBack,
  onOpenItem,
  onPlay,
  onRefresh,
  onError,
  onOpenGenre,
  onOpenPerson,
}: {
  payload: ItemDetailPayload;
  entryItemId: string;
  onBack: () => void;
  onOpenItem: (id: string) => void;
  onPlay: (id: string, mediaSourceId?: string, audioStreamIndex?: number, subtitleStreamIndex?: number, sources?: MediaVersion[], episodeIds?: string[]) => Promise<void>;
  onRefresh: () => Promise<void>;
  onError: (message: string) => void;
  onOpenGenre: (genre: string) => void;
  onOpenPerson: (personId: string, name: string) => void;
}) {
  const item = payload.item;
  const backVisible = useFloatingBackVisible(item.id);
  const entryEpisode = payload.episodes.find((episode) => episode.id === entryItemId);
  const initialSeasonId = entryEpisode?.seasonId ?? payload.seasons[0]?.id ?? "";
  const initialEpisodeId = entryEpisode?.id ?? "";
  const [seasonId, setSeasonId] = useState(initialSeasonId);
  const [selectedEpisodeId, setSelectedEpisodeId] = useState(initialEpisodeId);
  const [sourceIndex, setSourceIndex] = useState(0);
  const [detailPicker, setDetailPicker] = useState<DetailPicker | null>(null);
  const [scrollToEpisodeIndex, setScrollToEpisodeIndex] = useState<number>();
  const [jumpPickerOpen, setJumpPickerOpen] = useState(false);
  const [jumpValue, setJumpValue] = useState("");
  const [seasonPickerOpen, setSeasonPickerOpen] = useState(false);
  const [seasonChanged, setSeasonChanged] = useState(false);
  const [audioStreamIndex, setAudioStreamIndex] = useState<number>();
  const [subtitleStreamIndex, setSubtitleStreamIndex] = useState<number>();
  const closeSeasonTimer = useRef<number | undefined>(undefined);
  const closeJumpTimer = useRef<number | undefined>(undefined);
  const loadingMediaSourceIds = useRef(new Set<string>());
  const initialMediaSourceItemId = initialEpisodeId || payload.episodes[0]?.id || item.id;
  const [mediaSourcesByItem, setMediaSourcesByItem] = useState(() => new Map([[initialMediaSourceItemId, payload.mediaSources]]));
  const mediaSourcesByItemRef = useRef(mediaSourcesByItem);
  const episodes = useMemo(() => {
    if (!payload.episodes.length) return payload.children;
    if (!seasonId) return payload.episodes;
    const season = payload.seasons.find((entry) => entry.id === seasonId);
    return payload.episodes.filter((episode) => episode.seasonId === seasonId || episode.seasonName === season?.name);
  }, [payload.children, payload.episodes, payload.seasons, seasonId]);
  const selectedEpisode = episodes.find((episode) => episode.id === selectedEpisodeId) ?? episodes[0];
  const selectedPlayableId = selectedEpisode?.id ?? item.id;
  const currentMediaSources = mediaSourcesByItem.get(selectedPlayableId) ?? [];
  const selectedSource = currentMediaSources[sourceIndex % Math.max(currentMediaSources.length, 1)];
  const selectedFacts = mediaVersionFacts(selectedSource);
  const selectedAudio = selectedSource?.audioStreams.find((stream) => stream.index === audioStreamIndex) ?? selectedSource?.audioStreams.find((stream) => stream.isDefault) ?? selectedSource?.audioStreams[0];
  const selectedSubtitle = selectedSource?.subtitleStreams.find((stream) => stream.index === subtitleStreamIndex) ?? selectedSource?.subtitleStreams.find((stream) => stream.isDefault) ?? selectedSource?.subtitleStreams[0];
  const currentSeason = payload.seasons.find((season) => season.id === seasonId);

  useEffect(() => {
    mediaSourcesByItemRef.current = mediaSourcesByItem;
  }, [mediaSourcesByItem]);

  useEffect(() => {
    setSeasonId(initialSeasonId);
    setSelectedEpisodeId(initialEpisodeId);
    setSourceIndex(0);
    setDetailPicker(null);
    setJumpPickerOpen(false);
    setJumpValue("");
    setSeasonPickerOpen(false);
    setSeasonChanged(false);
    setAudioStreamIndex(undefined);
    setSubtitleStreamIndex(undefined);
    loadingMediaSourceIds.current.clear();
    setMediaSourcesByItem(new Map([[payload.episodes[0]?.id ?? item.id, payload.mediaSources], [initialEpisodeId || item.id, payload.mediaSources]]));
  }, [entryItemId, initialEpisodeId, initialSeasonId, item.id]);

  useEffect(() => {
    if (!selectedPlayableId) return;
    const selectedIndex = episodes.findIndex((episode) => episode.id === selectedPlayableId);
    const nextPlayableId = selectedIndex >= 0 ? episodes[selectedIndex + 1]?.id : undefined;
    let cancelled = false;

    function cacheMediaSources(itemId: string) {
      if (mediaSourcesByItemRef.current.has(itemId) || loadingMediaSourceIds.current.has(itemId)) return;
      loadingMediaSourceIds.current.add(itemId);
      ipc.loadMediaSources(itemId).then((sources) => {
        if (cancelled) return;
        setMediaSourcesByItem((current) => current.has(itemId) ? current : new Map(current).set(itemId, sources));
      }).catch(() => {
        if (!cancelled) setMediaSourcesByItem((current) => current.has(itemId) ? current : new Map(current).set(itemId, []));
      }).finally(() => {
        loadingMediaSourceIds.current.delete(itemId);
      });
    }

    cacheMediaSources(selectedPlayableId);
    if (nextPlayableId) cacheMediaSources(nextPlayableId);
    return () => {
      cancelled = true;
    };
  }, [episodes, selectedPlayableId]);

  useEffect(() => () => {
    window.clearTimeout(closeSeasonTimer.current);
    window.clearTimeout(closeJumpTimer.current);
  }, []);

  function keepSeasonPickerOpen() {
    window.clearTimeout(closeSeasonTimer.current);
  }

  function closeSeasonPickerSoon() {
    window.clearTimeout(closeSeasonTimer.current);
    closeSeasonTimer.current = window.setTimeout(() => setSeasonPickerOpen(false), 300);
  }

  function keepJumpPickerOpen() {
    window.clearTimeout(closeJumpTimer.current);
  }

  function closeJumpPickerSoon() {
    window.clearTimeout(closeJumpTimer.current);
    closeJumpTimer.current = window.setTimeout(() => setJumpPickerOpen(false), 300);
  }

  async function mark(command: "mark_favorite" | "mark_played", value: boolean) {
    try {
      await (command === "mark_favorite" ? ipc.markFavorite(item.id, value) : ipc.markPlayed(item.id, value));
      await onRefresh();
    } catch (err) {
      onError(String(err));
    }
  }

  function jumpToEpisode() {
    const index = Number(jumpValue) - 1;
    if (!Number.isInteger(index) || index < 0 || index >= episodes.length) return;
    setSelectedEpisodeId(episodes[index].id);
    setSourceIndex(0);
    setDetailPicker(null);
    setJumpPickerOpen(false);
    setJumpValue("");
    setScrollToEpisodeIndex(index);
  }

  return (
    <div className="detail-page">
      <button className={`back floating-back ${backVisible ? "" : "hidden"}`} onClick={onBack} aria-label="返回">
        <SvgIcon name="back" />
      </button>
      <section className="hero" style={bg(item.backdropUrl ?? item.primaryImageUrl)}>
        <div className="hero-copy">
          {item.logoUrl ? <img className="detail-logo" src={item.logoUrl} alt={item.name} /> : <h1 className="text-logo">{item.name}</h1>}
          <div className="chips">
            {item.year && <span>{item.year}</span>}
            {item.genres.slice(0, 3).map((genre) => (
              <button key={genre} className="chip-button" onClick={() => onOpenGenre(genre)}>{genre}</button>
            ))}
            {item.communityRating && <span>评分 {item.communityRating.toFixed(1)}</span>}
          </div>
          <p>{item.overview ?? "暂无简介"}</p>
          <div className="hero-actions">
            <button className="play" onClick={() => void onPlay(selectedPlayableId, selectedSource?.id, audioStreamIndex, subtitleStreamIndex, currentMediaSources, episodes.map((episode) => episode.id))}>
              <SvgIcon name="play" />
              <span>播放</span>
            </button>
            <button className="mark-round" onClick={() => void mark("mark_favorite", !item.favorite)} title={item.favorite ? "已收藏" : "收藏"}>
              <SvgIcon name="heart" />
            </button>
            <button className="mark-text" onClick={() => void mark("mark_played", !item.played)}>
              <SvgIcon name="check" />
              <span>{item.played ? "已看" : "标记已看"}</span>
            </button>
          </div>
          {selectedSource && (
            <div className="hero-option-row">
              <button onClick={() => setDetailPicker("source")}><span>版本</span><strong>{selectedSource.name || `资源 ${sourceIndex + 1}`}</strong></button>
              <button onClick={() => setDetailPicker("quality")}><span>画质</span><strong>{qualityLabel(selectedSource)}</strong></button>
              <button onClick={() => setDetailPicker("audio")}><span>音轨</span><strong>{streamLabel(selectedAudio) ?? "默认"}</strong></button>
              <button onClick={() => setDetailPicker("subtitle")}><span>字幕</span><strong>{subtitleStreamIndex === -1 ? "无" : streamLabel(selectedSubtitle) ?? "无"}</strong></button>
              {selectedFacts.length > 0 && <small>{selectedFacts.slice(0, 5).join(" / ")}</small>}
            </div>
          )}
        </div>
        {episodes.length > 0 && (
          <div className="detail-episode-shelf">
            <div className="detail-section-head">
              <div className="episode-title-row">
                {payload.seasons.length > 1 ? (
                  <div className={`season-switcher ${seasonPickerOpen ? "open" : ""}`} onMouseEnter={keepSeasonPickerOpen} onMouseLeave={closeSeasonPickerSoon}>
                    <button className={`season-toggle ${seasonChanged ? "changed" : ""}`} onAnimationEnd={() => setSeasonChanged(false)} onClick={() => setSeasonPickerOpen((open) => !open)}>
                      <span>{currentSeason?.name ?? "季度"}</span>
                      <SvgIcon name="next" />
                    </button>
                    {seasonPickerOpen && (
                      <div className="season-menu">
                        {payload.seasons.filter((season) => season.id !== seasonId).map((season) => (
                          <button key={season.id} onClick={() => {
                            setSeasonId(season.id);
                            setSelectedEpisodeId("");
                            setSourceIndex(0);
                            setAudioStreamIndex(undefined);
                            setSubtitleStreamIndex(undefined);
                            setScrollToEpisodeIndex(0);
                            window.clearTimeout(closeSeasonTimer.current);
                            setSeasonPickerOpen(false);
                            setSeasonChanged(true);
                          }}>{season.name}</button>
                        ))}
                      </div>
                    )}
                  </div>
                ) : (
                  <h2>选集</h2>
                )}
                <span className="episode-count">共 {episodes.length} 集</span>
              </div>
              <div className="episode-tools">
                <div className="episode-jump-wrap" onMouseEnter={keepJumpPickerOpen} onMouseLeave={closeJumpPickerSoon}>
                  <button className="episode-tool-button active" onClick={() => setJumpPickerOpen((open) => !open)}>跳转</button>
                  {jumpPickerOpen && (
                    <div className="episode-jump-popover">
                      <input
                        autoFocus
                        inputMode="numeric"
                        value={jumpValue}
                        onChange={(event) => setJumpValue(event.target.value.replace(/\D/g, ""))}
                        onKeyDown={(event) => {
                          if (event.key === "Enter") jumpToEpisode();
                          if (event.key === "Escape") setJumpPickerOpen(false);
                        }}
                        placeholder={`1-${episodes.length}`}
                      />
                      <button onClick={jumpToEpisode}>跳转</button>
                    </div>
                  )}
                </div>
              </div>
            </div>
            <ScrollableStage className="episode-stage" rowClassName="episode-row" itemCount={episodes.length} scrollToIndex={scrollToEpisodeIndex}>
              {episodes.map((episode, index) => (
                <button key={episode.id} className={`episode-card ${episode.id === selectedPlayableId ? "active" : ""}`} onClick={() => {
                  setSelectedEpisodeId(episode.id);
                  setSourceIndex(0);
                  setAudioStreamIndex(undefined);
                  setSubtitleStreamIndex(undefined);
                  setDetailPicker(null);
                  setScrollToEpisodeIndex(index);
                }}>
                  <Image src={episode.primaryImageUrl ?? episode.backdropUrl} alt={episode.name} />
                  <strong>{episode.name}</strong>
                  <span>{episodeLabel(episode)}</span>
                </button>
              ))}
            </ScrollableStage>
          </div>
        )}
      </section>
      {detailPicker && selectedSource && (
        <div className="modal-backdrop">
          <div className="source-modal">
            <button className="close" onClick={() => setDetailPicker(null)} title="关闭"><SvgIcon name="close" /></button>
            <h2>{pickerTitle(detailPicker)}</h2>
            <div className="source-list">
              {detailPicker === "source" && currentMediaSources.map((source, index) => (
                <button key={source.id} className={index === sourceIndex ? "active" : ""} onClick={() => {
                  setSourceIndex(index);
                  setAudioStreamIndex(undefined);
                  setSubtitleStreamIndex(undefined);
                  setDetailPicker(null);
                }}>
                  <strong>{source.name || `资源 ${index + 1}`}</strong>
                  <span>{mediaVersionFacts(source).join(" / ") || "暂无资源信息"}</span>
                  {source.path && <small>{source.path}</small>}
                </button>
              ))}
              {detailPicker === "quality" && (
                <button className="active" onClick={() => setDetailPicker(null)}>
                  <strong>{qualityLabel(selectedSource)}</strong>
                  <span>{qualityFacts(selectedSource).join(" / ") || "原画"}</span>
                </button>
              )}
              {detailPicker === "audio" && selectedSource.audioStreams.map((stream, index) => (
                <button key={stream.index ?? index} className={stream.index === selectedAudio?.index ? "active" : ""} onClick={() => {
                  setAudioStreamIndex(stream.index ?? index + 1);
                  setDetailPicker(null);
                }}>
                  <strong>{streamLabel(stream) ?? `音轨 ${index + 1}`}</strong>
                  <span>{streamFacts(stream).join(" / ") || "默认音轨"}</span>
                </button>
              ))}
              {detailPicker === "subtitle" && (
                <>
                  <button className={subtitleStreamIndex === -1 ? "active" : ""} onClick={() => {
                    setSubtitleStreamIndex(-1);
                    setDetailPicker(null);
                  }}>
                    <strong>无字幕</strong>
                    <span>关闭字幕</span>
                  </button>
                  {selectedSource.subtitleStreams.map((stream, index) => (
                    <button key={stream.index ?? index} className={stream.index === selectedSubtitle?.index && subtitleStreamIndex !== -1 ? "active" : ""} onClick={() => {
                      setSubtitleStreamIndex(stream.index ?? index + 1);
                      setDetailPicker(null);
                    }}>
                      <strong>{streamLabel(stream) ?? `字幕 ${index + 1}`}</strong>
                      <span>{streamFacts(stream).join(" / ") || "默认字幕"}</span>
                    </button>
                  ))}
                </>
              )}
            </div>
          </div>
        </div>
      )}
      {payload.people.length > 0 && (
        <section className="detail-block">
          <h2>演职人员</h2>
          <div className="people-row">
            {payload.people.slice(0, 3).map((person) => (
              <button
                key={`${person.id ?? person.name}-${person.role ?? ""}`}
                className="person-card"
                onClick={() => person.id && onOpenPerson(person.id, person.name)}
                disabled={!person.id}
              >
                <Image src={person.imageUrl} alt={person.name} />
                <strong>{person.name}</strong>
                <span>{person.role ?? person.personType ?? ""}</span>
              </button>
            ))}
          </div>
        </section>
      )}
      {payload.art.length > 0 && (
        <section className="detail-block">
          <h2>艺术图</h2>
          <div className="art-row">{payload.art.slice(0, 3).map((art) => <img key={art.url} src={art.url} alt={art.imageType} />)}</div>
        </section>
      )}
      {payload.similar.length > 0 && (
        <section className="detail-block">
          <h2>相关推荐</h2>
          <div className="poster-grid detail-posters">{payload.similar.slice(0, 3).map((similar) => <Poster key={similar.id} item={similar} onOpen={onOpenItem} />)}</div>
        </section>
      )}
    </div>
  );
}

function pickerTitle(picker: DetailPicker) {
  return picker === "source" ? "选择版本" : picker === "quality" ? "画质信息" : picker === "audio" ? "切换音轨" : "切换字幕";
}

function qualityLabel(source: ItemDetailPayload["mediaSources"][number]) {
  return [source.resolution, source.videoRange, source.bitDepth ? `${source.bitDepth}bit` : ""].filter(Boolean).join(" ") || source.videoDisplayTitle || "原画";
}

function qualityFacts(source: ItemDetailPayload["mediaSources"][number]) {
  return [
    source.videoDisplayTitle,
    source.videoCodec,
    source.videoRange,
    source.videoProfile,
    source.bitDepth ? `${source.bitDepth} bit` : "",
    source.pixelFormat,
  ].filter(Boolean) as string[];
}

function streamLabel(stream?: StreamInfo) {
  if (!stream) return undefined;
  return stream.displayTitle ?? stream.title ?? stream.language ?? stream.codec ?? undefined;
}

function streamFacts(stream: StreamInfo) {
  return [
    stream.language,
    stream.codec,
    stream.channelLayout,
    stream.channels ? `${stream.channels} 声道` : "",
    stream.isDefault ? "默认" : "",
    stream.isExternal ? "外挂" : "",
  ].filter(Boolean) as string[];
}

export function PlayerView({
  title,
  state,
  ready,
  onExit,
  onMinimize,
  onToggleMaximize,
  onToggleFullscreen,
  onClose,
  onCommand,
  canPlayPrevious,
  canPlayNext,
  onPlayPrevious,
  onPlayNext,
  seekBackSeconds,
  seekForwardSeconds,
  sources,
  currentSourceId,
  initialSubtitleIndex,
  onSwitchSource,
}: {
  title: string;
  state: PlaybackState | null;
  ready: boolean;
  onExit: () => void;
  onMinimize: () => void;
  onToggleMaximize: () => void;
  onToggleFullscreen: () => void;
  onClose: () => void;
  onCommand: (command: PlaybackCommand) => Promise<void>;
  canPlayPrevious: boolean;
  canPlayNext: boolean;
  onPlayPrevious: () => Promise<void>;
  onPlayNext: () => Promise<void>;
  seekBackSeconds: number;
  seekForwardSeconds: number;
  sources: MediaVersion[];
  currentSourceId: string | null;
  initialSubtitleIndex?: number;
  onSwitchSource: (sourceId?: string) => Promise<void>;
}) {
  const time = state?.timePos ?? 0;
  const duration = state?.duration ?? 0;
  const percent = duration > 0 ? Math.min(Math.max((time / duration) * 100, 0), 100) : 0;
  const volume = Math.round(state?.volume ?? 100);
  const currentSource = sources.find((source) => source.id === currentSourceId) ?? sources[0];
  const [visible, setVisible] = useState(true);
  const [menu, setMenu] = useState<"source" | "audio" | "subtitle" | null>(null);
  const [audioIndex, setAudioIndex] = useState<number>();
  const [subtitleIndex, setSubtitleIndex] = useState<number | undefined>(initialSubtitleIndex);
  const hideTimer = useRef<number | undefined>(undefined);
  const lastControlsShownAt = useRef(0);
  const onCommandRef = useRef(onCommand);
  const onExitRef = useRef(onExit);
  const onToggleFullscreenRef = useRef(onToggleFullscreen);
  const onPlayPreviousRef = useRef(onPlayPrevious);
  const onPlayNextRef = useRef(onPlayNext);
  const selectedAudio = currentSource?.audioStreams.find((stream) => stream.index === audioIndex) ?? currentSource?.audioStreams[0];
  const selectedSubtitle = currentSource?.subtitleStreams.find((stream) => stream.index === subtitleIndex) ?? currentSource?.subtitleStreams[0];

  useEffect(() => {
    onCommandRef.current = onCommand;
    onExitRef.current = onExit;
    onToggleFullscreenRef.current = onToggleFullscreen;
    onPlayPreviousRef.current = onPlayPrevious;
    onPlayNextRef.current = onPlayNext;
  });

  useEffect(() => {
    setSubtitleIndex(initialSubtitleIndex);
  }, [currentSourceId, initialSubtitleIndex]);

  function showControls() {
    if (menu) return;
    const now = performance.now();
    if (visible && now - lastControlsShownAt.current < 250) return;
    lastControlsShownAt.current = now;
    window.clearTimeout(hideTimer.current);
    if (!visible) setVisible(true);
    hideTimer.current = window.setTimeout(() => {
      if (!menu) setVisible(false);
    }, 2600);
  }

  useEffect(() => {
    function handleKeyDown(event: KeyboardEvent) {
      const target = event.target as HTMLElement | null;
      if (target?.closest("input, textarea, select")) return;
      if (event.key === "f" || event.key === "F") {
        event.preventDefault();
        onToggleFullscreenRef.current();
        return;
      }
      if ((event.key === "ArrowLeft" || event.key === "ArrowRight") && event.shiftKey) {
        event.preventDefault();
        void (event.key === "ArrowLeft" ? onPlayPreviousRef.current() : onPlayNextRef.current());
        return;
      }
      const commands: Record<string, PlaybackCommand> = {
        " ": "toggle_pause",
        ArrowLeft: "seek_back",
        ArrowRight: "seek_forward",
        ArrowDown: "volume_down",
        ArrowUp: "volume_up",
        a: "audio_next",
        A: "audio_next",
        s: "subtitle_next",
        S: "subtitle_next",
        m: "toggle_mute",
        M: "toggle_mute",
        "[": "speed_down",
        "]": "speed_up",
      };
      const command = commands[event.key];
      if (command) {
        event.preventDefault();
        void onCommandRef.current(command);
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        onExitRef.current();
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  useEffect(() => {
    if (menu) {
      window.clearTimeout(hideTimer.current);
      if (!visible) setVisible(true);
      return;
    }
    showControls();
    return () => window.clearTimeout(hideTimer.current);
  }, [menu]);

  return (
    <div className={`player-page ${ready ? "ready" : ""} ${visible || menu ? "" : "controls-hidden"}`} onMouseMove={showControls} onPointerDown={showControls}>
      <div className="player-drag-region" data-tauri-drag-region />
      <div className="player-top">
        <button className="player-round player-back" onClick={onExit} aria-label="返回"><SvgIcon name="back" /></button>
        <div className="player-top-spacer" data-tauri-drag-region />
        <div className="player-window-actions">
          <button className="icon-btn" onClick={onMinimize} title="最小化"><SvgIcon name="min" /></button>
          <button className="icon-btn" onClick={onToggleMaximize} title="最大化"><SvgIcon name="max" /></button>
          <button className="icon-btn" onClick={onClose} title="关闭"><SvgIcon name="close" /></button>
        </div>
      </div>
      <div className="player-controls">
        <div className="player-heading">
          <div>
            <strong>{title}</strong>
            <span>{state ? (state.paused ? "已暂停" : "正在播放") : "连接播放器..."}</span>
          </div>
          <span>{duration ? `${formatTime(time)} / ${formatTime(duration)}` : "正在连接进度"}</span>
        </div>
        <div className="player-progress">
          <button
            className="player-progress-track"
            onClick={(event) => {
              if (!duration) return;
              const rect = event.currentTarget.getBoundingClientRect();
              const ratio = (event.clientX - rect.left) / rect.width;
              void onCommand(`seek_absolute:${Math.max(0, Math.min(duration, duration * ratio))}`);
            }}
            aria-label="跳转播放进度"
          >
            <i style={{ width: `${percent}%` }} />
          </button>
        </div>
        <div className="player-main-actions">
          <button className="player-round caption-toggle" onClick={() => setMenu(menu === "subtitle" ? null : "subtitle")} aria-label="字幕"><SvgIcon name="captions" /></button>
          {canPlayPrevious && <button className="player-round" onClick={() => void onPlayPrevious()} aria-label="上一集"><SvgIcon name="back" /></button>}
          <button className="player-round seek-back" onClick={() => void onCommand("seek_back")} aria-label={`后退 ${seekBackSeconds} 秒`} title={`后退 ${seekBackSeconds} 秒`}><SvgIcon name="back" /></button>
          <button className="player-round pause-toggle" onClick={() => void onCommand("toggle_pause")} aria-label="暂停或继续"><SvgIcon name={state?.paused ? "play" : "pause"} /></button>
          <button className="player-round seek-forward" onClick={() => void onCommand("seek_forward")} aria-label={`前进 ${seekForwardSeconds} 秒`} title={`前进 ${seekForwardSeconds} 秒`}><SvgIcon name="next" /></button>
          {canPlayNext && <button className="player-round" onClick={() => void onPlayNext()} aria-label="下一集"><SvgIcon name="next" /></button>}
          <button className="player-round more-toggle" onClick={() => void onCommand("toggle_mute")} aria-label={state?.muted ? "取消静音" : "静音"}><SvgIcon name="volume" /></button>
          <button className="player-round fullscreen-toggle" onClick={onToggleFullscreen} aria-label="全屏"><SvgIcon name="fullscreen" /></button>
        </div>
        <div className="player-option-grid">
          <button className={menu === "audio" ? "active" : ""} onClick={() => setMenu(menu === "audio" ? null : "audio")}><span>音轨</span><strong>{streamLabel(selectedAudio) ?? "选择音轨"}</strong></button>
          <button className={menu === "subtitle" ? "active" : ""} onClick={() => setMenu(menu === "subtitle" ? null : "subtitle")}><span>字幕</span><strong>{subtitleIndex === -1 ? "无字幕" : streamLabel(selectedSubtitle) ?? "选择字幕"}</strong></button>
          <button className={menu === "source" ? "active" : ""} onClick={() => setMenu(menu === "source" ? null : "source")}><span>版本 / 画质</span><strong>{currentSource ? qualityLabel(currentSource) : "选择版本"}</strong></button>
          <div className="player-volume-control">
            <button onClick={() => void onCommand("toggle_mute")}><span>声音</span><strong>{state?.muted ? "已静音" : `${volume}%`}</strong></button>
            <input
              type="range"
              min="0"
              max="100"
              value={state?.muted ? 0 : volume}
              onChange={(event) => void onCommand(`volume_set:${Number(event.currentTarget.value)}`)}
              aria-label="音量"
            />
          </div>
        </div>
        {menu && (
          <div className="player-select-menu" onMouseEnter={() => window.clearTimeout(hideTimer.current)}>
            {menu === "source" && !sources.length && <button><strong>暂无版本</strong><span>当前播放项没有更多版本</span></button>}
            {menu === "source" && sources.map((source) => (
              <button key={source.id} className={source.id === currentSourceId ? "active" : ""} onClick={() => {
                setMenu(null);
                void onSwitchSource(source.id);
              }}>
                <strong>{source.name || qualityLabel(source)}</strong>
                <span>{mediaVersionFacts(source).join(" / ") || "默认版本"}</span>
              </button>
            ))}
            {menu === "audio" && !(currentSource?.audioStreams.length) && <button><strong>暂无音轨</strong><span>播放器未返回可选音轨</span></button>}
            {menu === "audio" && (currentSource?.audioStreams ?? []).map((stream, index) => (
              <button key={stream.index ?? index} className={stream.index === selectedAudio?.index ? "active" : ""} onClick={() => {
                const nextIndex = stream.index ?? index + 1;
                setAudioIndex(nextIndex);
                setMenu(null);
                void onCommand(`audio_set:${nextIndex}`);
              }}>
                <strong>{streamLabel(stream) ?? `音轨 ${index + 1}`}</strong>
                <span>{streamFacts(stream).join(" / ") || "默认音轨"}</span>
              </button>
            ))}
            {menu === "subtitle" && (
              <>
                <button className={subtitleIndex === -1 ? "active" : ""} onClick={() => {
                  setSubtitleIndex(-1);
                  setMenu(null);
                  void onCommand("subtitle_set:-1");
                }}><strong>无字幕</strong><span>关闭字幕</span></button>
                {(currentSource?.subtitleStreams ?? []).map((stream, index) => (
                  <button key={stream.index ?? index} className={stream.index === selectedSubtitle?.index && subtitleIndex !== -1 ? "active" : ""} onClick={() => {
                    const nextIndex = stream.index ?? index + 1;
                    setSubtitleIndex(nextIndex);
                    setMenu(null);
                    void onCommand(`subtitle_set:${index + 1}`);
                  }}>
                    <strong>{streamLabel(stream) ?? `字幕 ${index + 1}`}</strong>
                    <span>{streamFacts(stream).join(" / ") || "默认字幕"}</span>
                  </button>
                ))}
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
