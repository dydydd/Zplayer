import { useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useTranslation } from "react-i18next";
import { ipc } from "./ipc";
import i18n from "./i18n";
import { UiIcon } from "./icons";
import type { ItemDetailPayload, MediaVersion, PlaybackCommand, PlaybackState, StreamInfo } from "./types";
import { bg, episodeLabel, runtimeLabel } from "./media";
import { subtitleDialogFilters } from "./subtitleDialog";
import { formatTime, mediaVersionFacts } from "./viewLogic";
import { Image, Poster, ScrollableStage, useFloatingBackVisible } from "./viewParts";

type DetailPicker = "source" | "quality" | "audio" | "subtitle";

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
  onOpenCollection,
}: {
  payload: ItemDetailPayload;
  entryItemId: string;
  onBack: () => void;
  onOpenItem: (id: string, serverId?: string | null) => void;
  onPlay: (id: string, serverId?: string | null, mediaSourceId?: string, audioStreamIndex?: number, subtitleStreamIndex?: number, sources?: MediaVersion[], episodeIds?: string[]) => Promise<void>;
  onRefresh: () => Promise<void>;
  onError: (message: string) => void;
  onOpenGenre: (genre: string) => void;
  onOpenPerson: (personId: string, name: string) => void;
  onOpenCollection: (collectionId: string, title: string) => void;
}) {
  const { t } = useTranslation();
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
  const fallbackPlayableChildren = useMemo(
    () => payload.children.filter(isPlayableDetailItem),
    [payload.children],
  );
  const episodes = useMemo(() => {
    if (!payload.episodes.length) return fallbackPlayableChildren;
    if (!seasonId) return payload.episodes;
    const season = payload.seasons.find((entry) => entry.id === seasonId);
    return payload.episodes.filter((episode) => episode.seasonId === seasonId || episode.seasonName === season?.name);
  }, [fallbackPlayableChildren, payload.episodes, payload.seasons, seasonId]);
  const selectedEpisode = episodes.find((episode) => episode.id === selectedEpisodeId) ?? episodes[0];
  const selectedPlayableId = selectedEpisode?.id ?? item.id;
  const currentMediaSources = mediaSourcesByItem.get(selectedPlayableId) ?? [];
  const selectedSource = currentMediaSources[sourceIndex % Math.max(currentMediaSources.length, 1)];
  const selectedFacts = mediaVersionFacts(selectedSource);
  const selectedAudio = selectedSource?.audioStreams.find((stream) => stream.index === audioStreamIndex) ?? selectedSource?.audioStreams.find((stream) => stream.isDefault) ?? selectedSource?.audioStreams[0];
  const selectedSubtitle = selectedSource?.subtitleStreams.find((stream) => stream.index === subtitleStreamIndex) ?? selectedSource?.subtitleStreams.find((stream) => stream.isDefault) ?? selectedSource?.subtitleStreams[0];
  const currentSeason = payload.seasons.find((season) => season.id === seasonId);
  const runtime = runtimeLabel(item.runTimeTicks);
  const collectionLike = item.itemType === "BoxSet" || item.itemType === "CollectionFolder";

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
      const serverId = episodes.find((episode) => episode.id === itemId)?.serverId ?? item.serverId;
      ipc.loadMediaSources(itemId, serverId).then((sources) => {
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
  }, [episodes, item.serverId, selectedPlayableId]);

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
      await (command === "mark_favorite" ? ipc.markFavorite(item.id, value, item.serverId) : ipc.markPlayed(item.id, value, item.serverId));
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
      <button className={`back floating-back ${backVisible ? "" : "hidden"}`} onClick={onBack} aria-label={t("common.back")}>
        <UiIcon name="chevron-left" />
      </button>
      <section className="hero" style={bg(item.backdropUrl ?? item.primaryImageUrl)}>
        <div className="hero-copy">
          {item.logoUrl ? <img className="detail-logo" src={item.logoUrl} alt={item.name} /> : <h1 className="text-logo">{item.name}</h1>}
          <div className="chips">
            {item.year && <span>{item.year}</span>}
            {runtime && <span>{runtime}</span>}
            {item.officialRating && <span>{item.officialRating}</span>}
            {item.communityRating && <span>{t("common.score", { value: item.communityRating.toFixed(1) })}</span>}
            {item.genres.slice(0, 3).map((genre) => (
              <button key={genre} className="chip-button" onClick={() => onOpenGenre(genre)}>{genre}</button>
            ))}
          </div>
          <p>{item.overview ?? t("detail.noOverview")}</p>
          {item.playedPercentage ? (
            <div className="detail-progress">
              <span style={{ width: `${Math.min(Math.max(item.playedPercentage, 0), 100)}%` }} />
            </div>
          ) : null}
          <div className="hero-actions">
            <button className="play" onClick={() => void onPlay(selectedSource?.itemId ?? selectedPlayableId, selectedSource?.serverId ?? selectedEpisode?.serverId ?? item.serverId, selectedSource?.id, audioStreamIndex, subtitleStreamIndex, currentMediaSources, episodes.map((episode) => episode.id))}>
              <UiIcon name="play" />
              <span>{t("detail.play")}</span>
            </button>
            <button className="mark-round" onClick={() => void mark("mark_favorite", !item.favorite)} title={item.favorite ? t("detail.favorited") : t("detail.favorite")}>
              <UiIcon name="heart" />
            </button>
            <button className="mark-text" onClick={() => void mark("mark_played", !item.played)}>
              <UiIcon name="check" />
              <span>{item.played ? t("detail.watched") : t("detail.markWatched")}</span>
            </button>
            {collectionLike && (
              <button className="mark-text" onClick={() => onOpenCollection(item.id, item.name)}>
                <span>{t("detail.openCollection")}</span>
              </button>
            )}
          </div>
          {selectedSource && (
            <div className="hero-option-row">
              <button onClick={() => setDetailPicker("source")}><span>{t("detail.version")}</span><strong>{versionName(selectedSource, sourceIndex)}</strong></button>
              <button onClick={() => setDetailPicker("quality")}><span>{t("detail.quality")}</span><strong>{qualityLabel(selectedSource)}</strong></button>
              <button onClick={() => setDetailPicker("audio")}><span>{t("detail.audio")}</span><strong>{streamLabel(selectedAudio) ?? t("common.default")}</strong></button>
              <button onClick={() => setDetailPicker("subtitle")}><span>{t("detail.subtitle")}</span><strong>{subtitleStreamIndex === -1 ? t("detail.noSubtitle") : streamLabel(selectedSubtitle) ?? t("detail.noSubtitle")}</strong></button>
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
                      <span>{currentSeason?.name ?? t("detail.season")}</span>
                      <UiIcon name="chevron-right" />
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
                  <h2>{t("detail.episodes")}</h2>
                )}
                <span className="episode-count">{t("common.episodesCount", { count: episodes.length })}</span>
              </div>
              <div className="episode-tools">
                <div className="episode-jump-wrap" onMouseEnter={keepJumpPickerOpen} onMouseLeave={closeJumpPickerSoon}>
                  <button className="episode-tool-button active" onClick={() => setJumpPickerOpen((open) => !open)}>{t("detail.jump")}</button>
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
                      <button onClick={jumpToEpisode}>{t("detail.jump")}</button>
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
            <button className="close" onClick={() => setDetailPicker(null)} title={t("common.close")}><UiIcon name="x" /></button>
            <h2>{pickerTitle(detailPicker)}</h2>
            <div className="source-list">
              {detailPicker === "source" && currentMediaSources.map((source, index) => (
                <button key={`${source.serverId ?? ""}:${source.itemId ?? ""}:${source.id}`} className={index === sourceIndex ? "active" : ""} onClick={() => {
                  setSourceIndex(index);
                  setAudioStreamIndex(undefined);
                  setSubtitleStreamIndex(undefined);
                  setDetailPicker(null);
                }}>
                  <strong>{versionName(source, index)}</strong>
                  <span>{versionFacts(source, t("detail.noSourceInfo"))}</span>
                  {source.path && <small>{source.path}</small>}
                </button>
              ))}
              {detailPicker === "quality" && (
                <button className="active" onClick={() => setDetailPicker(null)}>
                  <strong>{qualityLabel(selectedSource)}</strong>
                  <span>{qualityFacts(selectedSource).join(" / ") || t("detail.originalQuality")}</span>
                </button>
              )}
              {detailPicker === "audio" && selectedSource.audioStreams.map((stream, index) => (
                <button key={stream.index ?? index} className={stream.index === selectedAudio?.index ? "active" : ""} onClick={() => {
                  setAudioStreamIndex(stream.index ?? index + 1);
                  setDetailPicker(null);
                }}>
                  <strong>{streamLabel(stream) ?? t("detail.audioTrackN", { index: index + 1 })}</strong>
                  <span>{streamFacts(stream).join(" / ") || t("detail.defaultAudio")}</span>
                </button>
              ))}
              {detailPicker === "subtitle" && (
                <>
                  <button className={subtitleStreamIndex === -1 ? "active" : ""} onClick={() => {
                    setSubtitleStreamIndex(-1);
                    setDetailPicker(null);
                  }}>
                    <strong>{t("detail.noSubtitleTitle")}</strong>
                    <span>{t("detail.subtitleOff")}</span>
                  </button>
                  {selectedSource.subtitleStreams.map((stream, index) => (
                    <button key={stream.index ?? index} className={stream.index === selectedSubtitle?.index && subtitleStreamIndex !== -1 ? "active" : ""} onClick={() => {
                      setSubtitleStreamIndex(stream.index ?? index + 1);
                      setDetailPicker(null);
                    }}>
                      <strong>{streamLabel(stream) ?? t("detail.subtitleN", { index: index + 1 })}</strong>
                      <span>{streamFacts(stream).join(" / ") || t("detail.defaultSubtitle")}</span>
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
          <h2>{t("detail.cast")}</h2>
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
          <h2>{t("detail.art")}</h2>
          <div className="art-row">{payload.art.slice(0, 3).map((art) => <img key={art.url} src={art.url} alt={art.imageType} />)}</div>
        </section>
      )}
      {payload.similar.length > 0 && (
        <section className="detail-block">
          <h2>{t("detail.similar")}</h2>
      <div className="poster-grid detail-posters">{payload.similar.slice(0, 3).map((similar) => <Poster key={`${similar.serverId ?? ""}:${similar.id}`} item={similar} onOpen={onOpenItem} />)}</div>
        </section>
      )}
    </div>
  );
}

function pickerTitle(picker: DetailPicker) {
  return picker === "source"
    ? i18n.t("detail.sourceTitle")
    : picker === "quality"
      ? i18n.t("detail.qualityTitle")
      : picker === "audio"
        ? i18n.t("detail.audioTitle")
        : i18n.t("detail.subtitleTitle");
}

function qualityLabel(source: ItemDetailPayload["mediaSources"][number]) {
  return [source.resolution, source.videoRange, source.bitDepth ? `${source.bitDepth}bit` : ""].filter(Boolean).join(" ") || source.videoDisplayTitle || i18n.t("detail.originalQuality");
}

function isPlayableDetailItem(item: { itemType: string }) {
  return ["Movie", "Episode", "Video"].includes(item.itemType);
}

function versionName(source: MediaVersion, index: number) {
  const name = source.name || i18n.t("detail.sourceN", { index: index + 1 });
  return source.serverName ? `${source.serverName} · ${name}` : name;
}

function versionFacts(source: MediaVersion, fallback: string) {
  return [source.serverName, ...mediaVersionFacts(source)].filter(Boolean).join(" / ") || fallback;
}

function formatLoadSpeed(bytesPerSecond?: number | null) {
  const speed = bytesPerSecond ?? 0;
  if (!Number.isFinite(speed) || speed <= 0) return "0 KB/s";
  const units = ["B/s", "KB/s", "MB/s", "GB/s"];
  let value = speed;
  let unitIndex = 0;
  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }
  const precision = unitIndex === 0 ? 0 : value < 10 ? 1 : 0;
  return `${value.toFixed(precision)} ${units[unitIndex]}`;
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
    stream.channels ? i18n.t("detail.channels", { count: stream.channels }) : "",
    stream.isDefault ? i18n.t("common.default") : "",
    stream.isExternal ? i18n.t("detail.external") : "",
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
  onError,
  canPlayPrevious,
  canPlayNext,
  onPlayPrevious,
  onPlayNext,
  seekBackSeconds,
  seekForwardSeconds,
  sources,
  currentSourceId,
  currentServerId,
  initialSubtitleIndex,
  onSwitchSource,
  onPreferenceChange,
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
  onError: (message: string) => void;
  canPlayPrevious: boolean;
  canPlayNext: boolean;
  onPlayPrevious: () => Promise<void>;
  onPlayNext: () => Promise<void>;
  seekBackSeconds: number;
  seekForwardSeconds: number;
  sources: MediaVersion[];
  currentSourceId: string | null;
  currentServerId?: string | null;
  initialSubtitleIndex?: number;
  onSwitchSource: (sourceId?: string, sourceServerId?: string | null) => Promise<void>;
  onPreferenceChange: (audioIndex?: number, subtitleIndex?: number) => void;
}) {
  const { t } = useTranslation();
  const time = state?.timePos ?? 0;
  const duration = state?.duration ?? 0;
  const percent = duration > 0 ? Math.min(Math.max((time / duration) * 100, 0), 100) : 0;
  const volume = Math.round(state?.volume ?? 100);
  const speed = state?.speed ?? 1;
  const currentSource = sources.find((source) => source.id === currentSourceId && (!currentServerId || !source.serverId || source.serverId === currentServerId)) ?? sources[0];
  const [visible, setVisible] = useState(true);
  const [menu, setMenu] = useState<"source" | "audio" | "subtitle" | "settings" | null>(null);
  const [audioIndex, setAudioIndex] = useState<number>();
  const [subtitleIndex, setSubtitleIndex] = useState<number | undefined>(initialSubtitleIndex);
  const [subtitleDelay, setSubtitleDelay] = useState(0);
  const [audioDelay, setAudioDelay] = useState(0);
  const [externalSubtitle, setExternalSubtitle] = useState("");
  const hideTimer = useRef<number | undefined>(undefined);
  const lastControlsShownAt = useRef(0);
  const onCommandRef = useRef(onCommand);
  const onExitRef = useRef(onExit);
  const onToggleFullscreenRef = useRef(onToggleFullscreen);
  const onPlayPreviousRef = useRef(onPlayPrevious);
  const onPlayNextRef = useRef(onPlayNext);
  const selectedAudio = currentSource?.audioStreams.find((stream) => stream.index === audioIndex) ?? currentSource?.audioStreams[0];
  const selectedSubtitle = currentSource?.subtitleStreams.find((stream) => stream.index === subtitleIndex) ?? currentSource?.subtitleStreams[0];
  const statusLabel = ready ? (state?.paused ? t("player.paused") : t("player.loadingSpeed", { speed: formatLoadSpeed(state?.cacheSpeed) })) : t("player.initializing");
  const currentTimeLabel = duration ? formatTime(time) : "0:00";
  const durationLabel = duration ? formatTime(duration) : "0:00";

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

  async function chooseExternalSubtitle() {
    try {
      const selected = await open({
        multiple: false,
        filters: subtitleDialogFilters,
      });
      if (typeof selected === "string") setExternalSubtitle(selected);
    } catch (err) {
      onError(String(err));
    }
  }

  return (
    <div className={`player-page ${ready ? "ready" : ""} ${visible || menu ? "" : "controls-hidden"}`} onMouseMove={showControls} onPointerDown={showControls}>
      <div className="player-drag-region" data-tauri-drag-region />
      <div className="player-top">
        <button className="player-round player-back" onClick={onExit} aria-label={t("common.back")}><UiIcon name="chevron-left" /></button>
        <div className="player-title-stack" data-tauri-drag-region>
          <strong>{title}</strong>
          <span>{currentSource ? qualityLabel(currentSource) : t("player.playingTitle")}</span>
        </div>
        <div className="player-top-spacer" data-tauri-drag-region />
        <div className="player-state-pill">{statusLabel}</div>
        <div className="player-window-actions">
          <button className="icon-btn" onClick={onMinimize} title={t("topbar.minimize")}><UiIcon name="minus" /></button>
          <button className="icon-btn" onClick={onToggleMaximize} title={t("topbar.maximize")}><UiIcon name="square" /></button>
          <button className="icon-btn" onClick={onClose} title={t("common.close")}><UiIcon name="x" /></button>
        </div>
      </div>
      {!ready && (
        <div className="player-center-status">
          <span className="player-spinner" aria-hidden="true" />
          <strong>{statusLabel}</strong>
        </div>
      )}
      <aside className="player-volume-rail">
        <button className="player-round" onClick={() => void onCommand("toggle_mute")} aria-label={state?.muted ? t("player.unmute") : t("player.mute")}><UiIcon name="volume" /></button>
        <input
          type="range"
          min="0"
          max="100"
          value={state?.muted ? 0 : volume}
          onChange={(event) => void onCommand(`volume_set:${Number(event.currentTarget.value)}`)}
          aria-label={t("player.volumeLabel")}
        />
        <span>{state?.muted ? "0" : volume}</span>
      </aside>
      <div className="player-bottom">
        <div className="player-main-actions">
          <button className="player-round" disabled={!canPlayPrevious} onClick={() => void onPlayPrevious()} aria-label={t("player.previousEpisode")}><UiIcon name="skip-back" /></button>
          <button className="player-round seek-back" onClick={() => void onCommand("seek_back")} aria-label={t("player.seekBack", { seconds: seekBackSeconds })} title={t("player.seekBack", { seconds: seekBackSeconds })}><UiIcon name="rewind" /></button>
          <button className="player-round pause-toggle" onClick={() => void onCommand("toggle_pause")} aria-label={t("player.pauseResume")}><UiIcon name={state?.paused ? "play" : "pause"} /></button>
          <button className="player-round seek-forward" onClick={() => void onCommand("seek_forward")} aria-label={t("player.seekForward", { seconds: seekForwardSeconds })} title={t("player.seekForward", { seconds: seekForwardSeconds })}><UiIcon name="fast-forward" /></button>
          <button className="player-round" disabled={!canPlayNext} onClick={() => void onPlayNext()} aria-label={t("player.nextEpisode")}><UiIcon name="skip-forward" /></button>
        </div>
        <div className="player-progress-row">
          <span>{currentTimeLabel}</span>
          <button
            className="player-progress-track"
            onClick={(event) => {
              if (!duration) return;
              const rect = event.currentTarget.getBoundingClientRect();
              const ratio = (event.clientX - rect.left) / rect.width;
              void onCommand(`seek_absolute:${Math.max(0, Math.min(duration, duration * ratio))}`);
            }}
            aria-label={t("player.seekProgress")}
          >
            <i style={{ width: `${percent}%` }} />
          </button>
          <span>{durationLabel}</span>
        </div>
        <div className="player-secondary-actions">
          <button className={`player-round ${menu === "audio" ? "active" : ""}`} onClick={() => setMenu(menu === "audio" ? null : "audio")} aria-label={t("player.audio")}><UiIcon name="music" /></button>
          <button className={`player-round ${menu === "subtitle" ? "active" : ""}`} onClick={() => setMenu(menu === "subtitle" ? null : "subtitle")} aria-label={t("player.captions")}><UiIcon name="captions" /></button>
          <button className="player-round" onClick={() => void onCommand("toggle_mute")} aria-label={state?.muted ? t("player.unmute") : t("player.mute")}><UiIcon name="volume" /></button>
          <button className={`player-round ${menu === "settings" ? "active" : ""}`} onClick={() => setMenu(menu === "settings" ? null : "settings")} aria-label={t("player.settings")}><UiIcon name="settings" /></button>
          <button className="player-round fullscreen-toggle" onClick={onToggleFullscreen} aria-label={t("player.fullscreen")}><UiIcon name="fullscreen" /></button>
        </div>
        {menu === "settings" && (
          <div className="player-option-grid player-settings-panel" onMouseEnter={() => window.clearTimeout(hideTimer.current)}>
            <button onClick={() => setMenu("source")}><span>{t("player.sourceQuality")}</span><strong>{currentSource ? qualityLabel(currentSource) : t("player.selectSource")}</strong></button>
            <button onClick={() => void onCommand(`speed_set:${Math.max(0.5, speed - 0.1)}`)}><span>{t("player.speed")}</span><strong>-</strong></button>
            <button onClick={() => void onCommand("speed_set:1")}><span>{speed.toFixed(2)}x</span><strong>{t("player.reset")}</strong></button>
            <button onClick={() => void onCommand(`speed_set:${Math.min(2, speed + 0.1)}`)}><span>{t("player.speed")}</span><strong>+</strong></button>
            <button onClick={() => {
              const next = Number((subtitleDelay - 0.25).toFixed(2));
              setSubtitleDelay(next);
              void onCommand(`subtitle_delay_set:${next}`);
            }}><span>{t("player.subtitleDelay")}</span><strong>-0.25</strong></button>
            <button onClick={() => {
              const next = Number((subtitleDelay + 0.25).toFixed(2));
              setSubtitleDelay(next);
              void onCommand(`subtitle_delay_set:${next}`);
            }}><span>{subtitleDelay.toFixed(2)}s</span><strong>+0.25</strong></button>
            <button onClick={() => {
              const next = Number((audioDelay + 0.25).toFixed(2));
              setAudioDelay(next);
              void onCommand(`audio_delay_set:${next}`);
            }}><span>{t("player.audioDelay")}</span><strong>{audioDelay.toFixed(2)}s</strong></button>
            <form className="external-subtitle-form" onSubmit={(event) => {
              event.preventDefault();
              const target = externalSubtitle.trim();
              if (target) void onCommand(`external_subtitle:${target}`);
            }}>
              <input value={externalSubtitle} onChange={(event) => setExternalSubtitle(event.currentTarget.value)} aria-label={t("player.subtitlePath")} />
              <button type="button" onClick={() => void chooseExternalSubtitle()}>{t("player.chooseSubtitle")}</button>
              <button>{t("player.loadSubtitle")}</button>
            </form>
          </div>
        )}
        {menu && menu !== "settings" && (
          <div className="player-select-menu" onMouseEnter={() => window.clearTimeout(hideTimer.current)}>
            {menu === "source" && !sources.length && <button><strong>{t("player.noSources")}</strong><span>{t("player.noSourcesNote")}</span></button>}
            {menu === "source" && sources.map((source, index) => (
              <button key={`${source.serverId ?? ""}:${source.itemId ?? ""}:${source.id}`} className={source.id === currentSourceId && (!currentServerId || !source.serverId || source.serverId === currentServerId) ? "active" : ""} onClick={() => {
                setMenu(null);
                void onSwitchSource(source.id, source.serverId);
              }}>
                <strong>{versionName(source, index)}</strong>
                <span>{versionFacts(source, t("player.defaultSource"))}</span>
              </button>
            ))}
            {menu === "audio" && !(currentSource?.audioStreams.length) && <button><strong>{t("player.noAudio")}</strong><span>{t("player.noAudioNote")}</span></button>}
            {menu === "audio" && (currentSource?.audioStreams ?? []).map((stream, index) => (
              <button key={stream.index ?? index} className={stream.index === selectedAudio?.index ? "active" : ""} onClick={() => {
                const nextIndex = stream.index ?? index + 1;
                setAudioIndex(nextIndex);
                setMenu(null);
                void onCommand(`audio_set:${nextIndex}`);
                onPreferenceChange(nextIndex, subtitleIndex);
              }}>
                <strong>{streamLabel(stream) ?? t("detail.audioTrackN", { index: index + 1 })}</strong>
                <span>{streamFacts(stream).join(" / ") || t("detail.defaultAudio")}</span>
              </button>
            ))}
            {menu === "subtitle" && (
              <>
                <button className={subtitleIndex === -1 ? "active" : ""} onClick={() => {
                  setSubtitleIndex(-1);
                  setMenu(null);
                  void onCommand("subtitle_set:-1");
                  onPreferenceChange(audioIndex, -1);
                }}><strong>{t("detail.noSubtitleTitle")}</strong><span>{t("detail.subtitleOff")}</span></button>
                {(currentSource?.subtitleStreams ?? []).map((stream, index) => (
                  <button key={stream.index ?? index} className={stream.index === selectedSubtitle?.index && subtitleIndex !== -1 ? "active" : ""} onClick={() => {
                    const nextIndex = stream.index ?? index + 1;
                    setSubtitleIndex(nextIndex);
                    setMenu(null);
                    void onCommand(`subtitle_set:${index + 1}`);
                    onPreferenceChange(audioIndex, nextIndex);
                  }}>
                    <strong>{streamLabel(stream) ?? t("detail.subtitleN", { index: index + 1 })}</strong>
                    <span>{streamFacts(stream).join(" / ") || t("detail.defaultSubtitle")}</span>
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
