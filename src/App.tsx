import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { ask, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { useLayoutEffect } from "react";
import { useDeferredValue } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { ipc } from "./ipc";
import { applyLanguage } from "./i18n";
import { ServerModal } from "./ServerModal";
import { TopBar } from "./TopBar";
import { CalendarView, DetailView, HomeView, LibraryView, LoadingPage, PlayerView, SearchOverlay, ServerView, SettingsView } from "./views";
import type { AppSettings, HomePayload, ItemDetailPayload, LibraryFilters, LibraryItemType, LibraryPayload, LibrarySortBy, LibrarySortOrder, LinuxWindowDiagnostics, LoginResult, MediaItem, MediaVersion, PlaybackCommand, PlaybackPreference, PlaybackPreferenceInput, PlaybackState, PlayResult, ResolvedAppSettings, SavedServer, ServerForm, ServerIconSelection, View, WatchCalendarPayload } from "./types";
import { emptyForm, withAppSettingsDefaults } from "./types";
import { collectionLibraryView, episodePlaybackContext, findKnownItem, libraryKey, preferencePayload, preferredStreamIndex, relativeEpisodeId, scopedPlaybackPreferenceKey } from "./appLogic";
import "./App.css";

const HOME_CACHE_STORAGE_PREFIX = "zplayer:home-cache:v1:";
const HOME_CACHE_MAX_AGE_MS = 1000 * 60 * 60 * 24 * 14;

type HistoryEntry = {
  view: View;
  scrollTop: number;
};

type PlaybackStoppedEvent = {
  itemId: string;
  serverId?: string | null;
  playSessionId: string;
  failed: boolean;
  completed: boolean;
};

type SubtitleSelection = {
  subtitleStreamIndex?: number;
  subtitleStreamPosition?: number;
};

function homeCacheStorageKey(serverId: string) {
  return `${HOME_CACHE_STORAGE_PREFIX}${serverId}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isHomePayload(value: unknown): value is HomePayload {
  if (!isRecord(value) || !isRecord(value.server)) return false;
  return (
    typeof value.server.id === "string"
    && typeof value.server.name === "string"
    && Array.isArray(value.libraries)
    && Array.isArray(value.libraryLatest)
    && Array.isArray(value.latest)
    && Array.isArray(value.recommendedMovies)
    && Array.isArray(value.recommendedShows)
    && Array.isArray(value.resumeItems)
    && Array.isArray(value.favoriteItems)
    && Array.isArray(value.recentItems)
  );
}

function readStoredHomeCache(serverId: string): HomePayload | null {
  try {
    const raw = localStorage.getItem(homeCacheStorageKey(serverId));
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (!isRecord(parsed) || typeof parsed.savedAt !== "number" || !isHomePayload(parsed.payload)) {
      return null;
    }
    if (Date.now() - parsed.savedAt > HOME_CACHE_MAX_AGE_MS || parsed.payload.server.id !== serverId) {
      localStorage.removeItem(homeCacheStorageKey(serverId));
      return null;
    }
    return parsed.payload;
  } catch {
    return null;
  }
}

function writeStoredHomeCache(home: HomePayload) {
  try {
    localStorage.setItem(homeCacheStorageKey(home.server.id), JSON.stringify({
      savedAt: Date.now(),
      payload: home,
    }));
  } catch {
    // localStorage is an optimization; the in-memory cache still works.
  }
}

function clearStoredHomeCaches() {
  try {
    for (let index = localStorage.length - 1; index >= 0; index -= 1) {
      const key = localStorage.key(index);
      if (key?.startsWith(HOME_CACHE_STORAGE_PREFIX)) {
        localStorage.removeItem(key);
      }
    }
  } catch {
    // Storage can be unavailable in restricted environments.
  }
}

function samePlaybackState(left: PlaybackState | null, right: PlaybackState | null) {
  return left?.timePos === right?.timePos
    && left?.duration === right?.duration
    && left?.paused === right?.paused
    && left?.muted === right?.muted
    && left?.volume === right?.volume
    && left?.speed === right?.speed
    && left?.cacheSpeed === right?.cacheSpeed
    && left?.videoReady === right?.videoReady;
}

function optimisticPlaybackState(current: PlaybackState | null, command: PlaybackCommand, settings: ResolvedAppSettings): PlaybackState | null {
  if (!current) return current;
  if (command === "toggle_pause") return { ...current, paused: !current.paused };
  if (command === "toggle_mute") return { ...current, muted: !current.muted };
  if (command === "seek_back") return { ...current, timePos: Math.max((current.timePos ?? 0) - settings.seekBackSeconds, 0) };
  if (command === "seek_forward") {
    const next = (current.timePos ?? 0) + settings.seekForwardSeconds;
    return { ...current, timePos: current.duration ? Math.min(next, current.duration) : next };
  }
  if (command.startsWith("seek_absolute:")) {
    const target = Number(command.slice("seek_absolute:".length));
    return Number.isFinite(target) ? { ...current, timePos: target } : current;
  }
  if (command === "volume_down") return { ...current, volume: Math.max((current.volume ?? 100) - 5, 0) };
  if (command === "volume_up") return { ...current, volume: Math.min((current.volume ?? 100) + 5, 100) };
  if (command === "speed_down") return { ...current, speed: Math.max((current.speed ?? 1) - 0.1, 0.5) };
  if (command === "speed_up") return { ...current, speed: Math.min((current.speed ?? 1) + 0.1, 2) };
  if (command.startsWith("speed_set:")) {
    const speed = Number(command.slice("speed_set:".length));
    return Number.isFinite(speed) ? { ...current, speed: Math.min(Math.max(speed, 0.5), 2) } : current;
  }
  if (command.startsWith("volume_set:")) {
    const volume = Number(command.slice("volume_set:".length));
    return Number.isFinite(volume) ? { ...current, muted: volume === 0, volume: Math.min(Math.max(volume, 0), 100) } : current;
  }
  return current;
}

function itemCacheKey(itemId: string, serverId?: string | null) {
  return `${serverId ?? ""}:${itemId}`;
}

function mediaSourceForPlayback(sources: MediaVersion[] | undefined, mediaSourceId?: string, serverId?: string | null) {
  if (!sources?.length) return undefined;
  return sources.find((source) => source.id === mediaSourceId && (!serverId || !source.serverId || source.serverId === serverId))
    ?? sources.find((source) => !serverId || !source.serverId || source.serverId === serverId)
    ?? sources[0];
}

function isServerIconFormKey(key: keyof ServerForm) {
  return key === "iconUrl" || key === "iconName";
}

function sourceMatchesView(source: MediaVersion, mediaSourceId?: string | null, serverId?: string | null) {
  return source.id === mediaSourceId && (!serverId || !source.serverId || source.serverId === serverId);
}

function defaultSubtitleSelection(source?: MediaVersion): SubtitleSelection {
  if (!source?.subtitleStreams.length) return {};
  const streamIndex = source.subtitleStreams.findIndex((stream) => stream.isDefault);
  const position = streamIndex >= 0 ? streamIndex : 0;
  const stream = source.subtitleStreams[position];
  return {
    subtitleStreamIndex: stream.index ?? position + 1,
    subtitleStreamPosition: position + 1,
  };
}

function subtitleStreamPosition(source: MediaVersion | undefined, subtitleStreamIndex: number | undefined) {
  if (subtitleStreamIndex === undefined) return undefined;
  if (subtitleStreamIndex < 0) return -1;
  const streamIndex = source?.subtitleStreams.findIndex((stream) => stream.index === subtitleStreamIndex) ?? -1;
  if (streamIndex >= 0) return streamIndex + 1;
  if (source && subtitleStreamIndex >= 1 && subtitleStreamIndex <= source.subtitleStreams.length) {
    return subtitleStreamIndex;
  }
  return undefined;
}

function resolveSubtitleSelection(source: MediaVersion | undefined, subtitleStreamIndex: number | undefined, subtitleMode: ResolvedAppSettings["subtitleMode"]): SubtitleSelection {
  if (subtitleStreamIndex !== undefined) {
    return {
      subtitleStreamIndex,
      subtitleStreamPosition: subtitleStreamPosition(source, subtitleStreamIndex),
    };
  }
  if (subtitleMode === "off") {
    return {
      subtitleStreamIndex: -1,
      subtitleStreamPosition: -1,
    };
  }
  return defaultSubtitleSelection(source);
}

function App() {
  const { t } = useTranslation();
  const [view, setView] = useState<View>({ name: "home" });
  const [servers, setServers] = useState<SavedServer[]>([]);
  const [settings, setSettings] = useState<AppSettings>({});
  const [home, setHome] = useState<HomePayload | null>(null);
  const [calendar, setCalendar] = useState<WatchCalendarPayload | null>(null);
  const [library, setLibrary] = useState<LibraryPayload | null>(null);
  const [libraryLoadingMore, setLibraryLoadingMore] = useState(false);
  const [detail, setDetail] = useState<ItemDetailPayload | null>(null);
  const [loading, setLoading] = useState("");
  const [error, setError] = useState("");
  const [playbackState, setPlaybackState] = useState<PlaybackState | null>(null);
  const [playbackPreferences, setPlaybackPreferences] = useState<Record<string, PlaybackPreference>>({});
  const [linuxWindowDiagnostics, setLinuxWindowDiagnostics] = useState<LinuxWindowDiagnostics | null>(null);
  const [playerTransparent, setPlayerTransparent] = useState(false);
  const [playerSources, setPlayerSources] = useState<MediaVersion[]>([]);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const deferredSearchQuery = useDeferredValue(searchQuery);
  const [searchResults, setSearchResults] = useState<MediaItem[]>([]);
  const [searchLoading, setSearchLoading] = useState(false);
  const [recentSearchTerms, setRecentSearchTerms] = useState<string[]>(() => {
    try {
      const terms = JSON.parse(localStorage.getItem("zplayer:recent-searches") ?? "[]");
      return Array.isArray(terms) ? terms.slice(0, 8) : [];
    } catch {
      return [];
    }
  });
  const [lastPlayResult, setLastPlayResult] = useState<PlayResult | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [form, setForm] = useState<ServerForm>(emptyForm);
  const [testedLogin, setTestedLogin] = useState<LoginResult | null>(null);
  const [editingServerId, setEditingServerId] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [serverMenuOpen, setServerMenuOpen] = useState(false);
  const [chromeVisible, setChromeVisible] = useState(true);
  const viewHistory = useRef<HistoryEntry[]>([]);
  const viewRef = useRef<View>({ name: "servers" });
  const requestId = useRef(0);
  const lastScrollTop = useRef(0);
  const chromeScrollFrame = useRef<number | null>(null);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const homeCache = useRef(new Map<string, HomePayload>());
  const detailCache = useRef(new Map<string, ItemDetailPayload>());
  const libraryCache = useRef(new Map<string, LibraryPayload>());
  const exitingPlaybackSession = useRef("");
  const chromeVisibleRef = useRef(true);
  const optimisticUntil = useRef(0);
  const resumedPlaybackSession = useRef("");
  const playRelativeEpisodeRef = useRef<(offset: -1 | 1, stopCurrent?: boolean) => Promise<void>>(async () => {});
  const refreshAfterPlaybackStopRef = useRef<(itemId?: string | null, targetView?: View) => void>(() => {});

  const activeServer = useMemo(
    () => servers.find((server) => server.active) ?? null,
    [servers],
  );
  const resolvedSettings = useMemo(() => withAppSettingsDefaults(settings), [settings]);
  const resolvedSettingsRef = useRef<ResolvedAppSettings>(resolvedSettings);

  useEffect(() => {
    resolvedSettingsRef.current = resolvedSettings;
  }, [resolvedSettings]);

  useEffect(() => {
    void applyLanguage(resolvedSettings.language);
  }, [resolvedSettings.language]);

  useEffect(() => {
    if (resolvedSettings.metadataCacheEnabled) return;
    clearHomeMetadataCaches();
    detailCache.current.clear();
    libraryCache.current.clear();
  }, [resolvedSettings.metadataCacheEnabled]);

  useEffect(() => {
    if (!resolvedSettings.diagnosticsEnabled) setLastPlayResult(null);
  }, [resolvedSettings.diagnosticsEnabled]);

  useEffect(() => {
    if (!activeServer) {
      setPlaybackPreferences({});
      return;
    }
    ipc.loadPlaybackPreferences()
      .then((preferences) => {
        const scoped = Object.fromEntries(
          Object.entries(preferences).map(([key, value]) => [`${activeServer.id}:${key}`, value]),
        );
        setPlaybackPreferences(scoped);
      })
      .catch(() => setPlaybackPreferences({}));
  }, [activeServer?.id]);

  useEffect(() => {
    void refreshServers();
    void loadSettings();
    void ipc.linuxWindowDiagnostics().then(setLinuxWindowDiagnostics).catch(() => setLinuxWindowDiagnostics(null));
  }, []);

  useEffect(() => {
    viewRef.current = view;
  }, [view]);

  useEffect(() => {
    if (view.name === "player") {
      setSearchResults([]);
      setSearchLoading(false);
      return;
    }
    const query = deferredSearchQuery.trim();
    if (!query) {
      setSearchResults([]);
      setSearchLoading(false);
      return;
    }

    let cancelled = false;
    setSearchLoading(true);
    const timer = window.setTimeout(() => {
      ipc.searchItems(query)
        .then((result) => {
          if (!cancelled) {
            setSearchResults(result.items);
          }
        })
        .catch((err) => {
          if (!cancelled) {
            setError(String(err));
            setSearchResults([]);
          }
        })
        .finally(() => {
          if (!cancelled) {
            setSearchLoading(false);
          }
        });
    }, 300);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [deferredSearchQuery, view.name]);

  useEffect(() => {
    if (searchOpen) {
      searchInputRef.current?.focus();
    }
  }, [searchOpen]);

  useEffect(() => {
    const scroller = document.querySelector<HTMLElement>(".workspace");
    if (!scroller) return;
    const element = scroller;

    function updateChromeVisibility() {
      const nextScrollTop = element.scrollTop;
      const scrollingUp = nextScrollTop < lastScrollTop.current;
      const nextVisible = nextScrollTop < 24 || scrollingUp || searchOpen || serverMenuOpen;
      if (chromeVisibleRef.current !== nextVisible) {
        chromeVisibleRef.current = nextVisible;
        setChromeVisible(nextVisible);
      }
      lastScrollTop.current = nextScrollTop;
    }

    function scheduleChromeVisibility() {
      if (chromeScrollFrame.current !== null) return;
      chromeScrollFrame.current = window.requestAnimationFrame(() => {
        chromeScrollFrame.current = null;
        updateChromeVisibility();
      });
    }

    element.addEventListener("scroll", scheduleChromeVisibility, { passive: true });
    updateChromeVisibility();
    return () => {
      if (chromeScrollFrame.current !== null) {
        window.cancelAnimationFrame(chromeScrollFrame.current);
        chromeScrollFrame.current = null;
      }
      element.removeEventListener("scroll", scheduleChromeVisibility);
    };
  }, [searchOpen, serverMenuOpen]);

  useEffect(() => {
    if (view.name === "home") {
      void loadHome();
    }
    if (view.name === "calendar") {
      void loadWatchCalendar();
    }
    if (view.name === "library") {
      void loadLibrary(view.id, view.itemType ?? "", view.sortBy ?? "DateCreated", view.sortOrder ?? "Descending", view.filters ?? {});
    }
    if (view.name === "detail") {
      void loadDetail(view.id, view.serverId);
    }
  }, [view]);

  useLayoutEffect(() => {
    const playing = view.name === "player" && playerTransparent;
    document.documentElement.classList.toggle("playing-embedded", playing);
    document.body.classList.toggle("playing-embedded", playing);
    return () => {
      document.documentElement.classList.remove("playing-embedded");
      document.body.classList.remove("playing-embedded");
    };
  }, [view.name, playerTransparent]);

  useEffect(() => {
    setPlayerTransparent(view.name === "player" && !!view.playSessionId);
  }, [view.name, view.name === "player" ? view.playSessionId : null]);

  useEffect(() => {
    if (view.name !== "player" || !view.playSessionId || !playerTransparent) return;
    if (resumedPlaybackSession.current === view.playSessionId) return;
    resumedPlaybackSession.current = view.playSessionId;
    void ipc.controlPlayback(view.playSessionId, "resume").catch(() => {});
  }, [view.name, view.name === "player" ? view.playSessionId : null, playerTransparent]);

  useEffect(() => {
    if (view.name !== "player") {
      setPlaybackState(null);
      return;
    }
    if (!view.playSessionId) return;
    const playSessionId = view.playSessionId;
    let cancelled = false;
    let timer: number | undefined;
    const loadState = async () => {
      if (document.hidden) {
        if (!cancelled) {
          timer = window.setTimeout(() => void loadState(), 250);
        }
        return;
      }
      try {
        const state = await ipc.playbackState(playSessionId);
        if (!cancelled) {
          if (performance.now() >= optimisticUntil.current) {
            setPlaybackState((current) => samePlaybackState(current, state) ? current : state);
          }
        }
      } catch {
        if (!cancelled) setPlaybackState(null);
      } finally {
        if (!cancelled) {
          timer = window.setTimeout(() => void loadState(), 250);
        }
      }
    };
    void loadState();
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [view]);

  useEffect(() => {
    if (view.name !== "player") return;
    if (playerSources.length) return;
    const cached = detailCache.current.get(itemCacheKey(view.itemId, view.serverId)) ?? (detail?.item.id === view.itemId && (!view.serverId || detail.item.serverId === view.serverId) ? detail : null);
    if (cached?.mediaSources.length) {
      setPlayerSources(cached.mediaSources);
      return;
    }
    let cancelled = false;
    ipc.loadMediaSources(view.itemId, view.serverId)
      .then((sources) => {
        if (!cancelled) setPlayerSources(sources);
      })
      .catch(() => {
        if (!cancelled) setPlayerSources([]);
      });
    return () => {
      cancelled = true;
    };
  }, [view, detail?.item.id, detail?.item.serverId, playerSources.length]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    listen<PlaybackStoppedEvent>("playback-stopped", (event) => {
      if (cancelled) return;
      invalidatePlaybackCaches(event.payload.itemId, event.payload.serverId);
      const currentView = viewRef.current;
      if (event.payload.playSessionId === exitingPlaybackSession.current) {
        exitingPlaybackSession.current = "";
        refreshAfterPlaybackStopRef.current(event.payload.itemId);
        return;
      }
      if (currentView.name === "player" && event.payload.playSessionId === currentView.playSessionId) {
        if (exitingPlaybackSession.current === currentView.playSessionId) {
          exitingPlaybackSession.current = "";
          refreshAfterPlaybackStopRef.current(event.payload.itemId);
          return;
        }
        if (event.payload.failed) {
          setError(t("errors.mpvExited"));
        }
        if (
          event.payload.completed
          && resolvedSettingsRef.current.autoplayNextEpisode
          && relativeEpisodeId(episodeContextFromView(currentView), 1)
        ) {
          void playRelativeEpisodeRef.current(1, false);
          refreshAfterPlaybackStopRef.current(event.payload.itemId);
          return;
        }
        const previousView = viewHistory.current[viewHistory.current.length - 1]?.view ?? { name: "home" as const };
        goBack();
        refreshAfterPlaybackStopRef.current(event.payload.itemId, previousView);
      }
    }).then((dispose) => {
      if (cancelled) {
        dispose();
      } else {
        unlisten = dispose;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [t]);

  async function run<T>(label: string, action: () => Promise<T>) {
    setLoading(label);
    setError("");
    try {
      return await action();
    } catch (err) {
      const message = String(err);
      if (!message.includes("reading 'invoke'")) {
        setError(message);
      }
      return null;
    } finally {
      setLoading("");
    }
  }

  async function runContentRequest<T>(request: number, label: string, action: () => Promise<T>) {
    setLoading(label);
    setError("");
    try {
      return await action();
    } catch (err) {
      if (request === requestId.current) {
        setError(String(err));
      }
      return null;
    } finally {
      if (request === requestId.current) {
        setLoading("");
      }
    }
  }

  async function refreshServers() {
    const result = await run(t("loading.servers"), () =>
      ipc.listServers(),
    );
    if (result) {
      setServers(result);
      if (result.some((server) => server.active)) {
        replaceView({ name: "home" });
      }
    }
  }

  async function loadSettings() {
    const result = await run(t("loading.settings"), () => ipc.loadSettings());
    if (result) {
      setSettings(result);
    }
  }

  async function saveSettings(next: AppSettings) {
    const result = await run(t("common.save"), () => ipc.saveSettings(withAppSettingsDefaults(next)));
    if (result) {
      setSettings(result);
      if (viewRef.current.name === "calendar") {
        setCalendar(null);
        void loadWatchCalendar();
      }
    }
  }

  const openView = useCallback((nextView: View) => {
    const scroller = document.querySelector<HTMLElement>(".workspace");
    viewHistory.current = [...viewHistory.current, { view, scrollTop: scroller?.scrollTop ?? 0 }];
    setView(nextView);
  }, [view]);

  function replaceView(nextView: View) {
    viewHistory.current = [];
    setView(nextView);
  }

  function goBack() {
    const previous = viewHistory.current[viewHistory.current.length - 1] ?? {
      view: { name: "home" as const },
      scrollTop: 0,
    };
    viewHistory.current = viewHistory.current.slice(0, -1);
    setView(previous.view);
    window.setTimeout(() => {
      document.querySelector<HTMLElement>(".workspace")?.scrollTo({ top: previous.scrollTop });
    });
  }

  function invalidatePlaybackCaches(itemId?: string | null, serverId?: string | null) {
    clearHomeMetadataCaches();
    libraryCache.current.clear();
    if (itemId) detailCache.current.delete(itemCacheKey(itemId, serverId));
  }

  function resetServerScopedState() {
    requestId.current += 1;
    clearHomeMetadataCaches();
    detailCache.current.clear();
    libraryCache.current.clear();
    setHome(null);
    setCalendar(null);
    setLibrary(null);
    setLibraryLoadingMore(false);
    setDetail(null);
  }

  function rememberHomePayload(payload: HomePayload) {
    if (!resolvedSettings.metadataCacheEnabled) return;
    homeCache.current.set(payload.server.id, payload);
    writeStoredHomeCache(payload);
  }

  function clearHomeMetadataCaches() {
    homeCache.current.clear();
    clearStoredHomeCaches();
  }

  function refreshAfterPlaybackStop(itemId?: string | null, targetView: View = viewRef.current) {
    const currentView = targetView;
    if (currentView.name === "home") {
      void loadHome();
    } else if (currentView.name === "detail") {
      void loadDetail(currentView.id, currentView.serverId, true);
    } else if (itemId) {
      void loadHome();
    }
  }
  refreshAfterPlaybackStopRef.current = refreshAfterPlaybackStop;

  async function loadHome() {
    const activeServerId = servers.find((server) => server.active)?.id ?? "";
    const cached = resolvedSettings.metadataCacheEnabled && activeServerId ? homeCache.current.get(activeServerId) : null;
    if (cached) {
      setHome(cached);
      if (!cached.libraryLatest.length && !cached.recommendedMovies.length && !cached.recommendedShows.length && !cached.favoriteItems.length) {
        const currentRequest = ++requestId.current;
        void loadHomeMore(cached.server.id, currentRequest);
      }
      return;
    }
    const stored = resolvedSettings.metadataCacheEnabled && activeServerId ? readStoredHomeCache(activeServerId) : null;
    if (stored) {
      homeCache.current.set(stored.server.id, stored);
      setHome(stored);
      const currentRequest = ++requestId.current;
      void refreshHomeInBackground(stored.server.id, currentRequest);
      return;
    }
    const currentRequest = ++requestId.current;
    const result = await runContentRequest(currentRequest, t("loading.home"), () => ipc.loadHome());
    if (result && currentRequest === requestId.current) {
      rememberHomePayload(result);
      setHome(result);
      void loadHomeMore(result.server.id, currentRequest);
    }
  }

  async function refreshHomeInBackground(serverId: string, parentRequest: number) {
    try {
      const result = await ipc.loadHome();
      if (parentRequest !== requestId.current || result.server.id !== serverId) return;
      rememberHomePayload(result);
      setHome(result);
      void loadHomeMore(result.server.id, parentRequest);
    } catch {
      // Keep the stored home visible while the server catches up.
    }
  }

  async function loadHomeMore(serverId: string, parentRequest: number) {
    try {
      const more = await ipc.loadHomeMore();
      if (parentRequest !== requestId.current || more.serverId !== serverId) return;
      setHome((current) => {
        if (!current || current.server.id !== more.serverId) return current;
        const next = {
          ...current,
          libraryLatest: more.libraryLatest,
          recommendedMovies: more.recommendedMovies,
          recommendedShows: more.recommendedShows,
          resumeItems: more.resumeItems,
          favoriteItems: more.favoriteItems,
          recentItems: more.recentItems,
        };
        rememberHomePayload(next);
        return next;
      });
    } catch {
      // ponytail: non-critical shelves can stay empty; surface only the first-screen failure.
    }
  }

  async function loadWatchCalendar() {
    const currentRequest = ++requestId.current;
    const result = await runContentRequest(currentRequest, t("loading.calendar"), () => ipc.loadWatchCalendar());
    if (result && currentRequest === requestId.current) {
      setCalendar(result);
    }
  }

  async function loadLibrary(libraryId: string, itemType: LibraryItemType = "", sortBy: LibrarySortBy = "DateCreated", sortOrder: LibrarySortOrder = "Descending", filters: LibraryFilters = {}) {
    const key = libraryKey(libraryId, itemType, sortBy, sortOrder, filters);
    const cached = resolvedSettings.metadataCacheEnabled ? libraryCache.current.get(key) : null;
    if (cached) {
      setLibrary(cached);
      void refreshLibraryInBackground(key, libraryId, itemType, sortBy, sortOrder, filters);
      return;
    }
    const currentRequest = ++requestId.current;
    const result = await runContentRequest(currentRequest, t("loading.library"), () =>
      ipc.loadLibrary(libraryId, 0, 60, itemType, sortBy, sortOrder, filters),
    );
    if (result && currentRequest === requestId.current) {
      if (resolvedSettings.metadataCacheEnabled) libraryCache.current.set(key, result);
      setLibrary(result);
    }
  }

  async function refreshLibraryInBackground(key: string, libraryId: string, itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder, filters: LibraryFilters) {
    try {
      const result = await ipc.loadLibrary(libraryId, 0, 60, itemType, sortBy, sortOrder, filters);
      if (resolvedSettings.metadataCacheEnabled) libraryCache.current.set(key, result);
      if (viewRef.current.name === "library") {
        const currentKey = libraryKey(
          viewRef.current.id,
          viewRef.current.itemType ?? "",
          viewRef.current.sortBy ?? "DateCreated",
          viewRef.current.sortOrder ?? "Descending",
          viewRef.current.filters ?? {},
        );
        if (currentKey === key) setLibrary(result);
      }
    } catch {
      // ponytail: cached library remains usable during refresh failures.
    }
  }

  const loadMoreLibraryItems = useCallback(async () => {
    if (!library || libraryLoadingMore || !library.hasMore) return;
    const currentLibrary = library;
    const currentRequest = requestId.current;
    setLibraryLoadingMore(true);
    setError("");
    try {
      const result = await ipc.loadLibrary(
        currentLibrary.library.id,
        currentLibrary.items.length,
        currentLibrary.limit || 60,
        view.name === "library" ? view.itemType ?? "" : "",
        view.name === "library" ? view.sortBy ?? "DateCreated" : "DateCreated",
        view.name === "library" ? view.sortOrder ?? "Descending" : "Descending",
        view.name === "library" ? view.filters ?? {} : {},
      );
      setLibrary((existing) => {
        if (!existing || existing.library.id !== currentLibrary.library.id) return existing;
        const next = {
          ...result,
          library: existing.library,
          items: [...existing.items, ...result.items],
        };
        const itemType = view.name === "library" ? view.itemType ?? "" : "";
        const sortBy = view.name === "library" ? view.sortBy ?? "DateCreated" : "DateCreated";
        const sortOrder = view.name === "library" ? view.sortOrder ?? "Descending" : "Descending";
        const filters = view.name === "library" ? view.filters ?? {} : {};
        if (resolvedSettings.metadataCacheEnabled) libraryCache.current.set(libraryKey(next.library.id, itemType, sortBy, sortOrder, filters), next);
        return next;
      });
    } catch (err) {
      if (currentRequest === requestId.current) {
        setError(String(err));
      }
    } finally {
      if (currentRequest === requestId.current) {
        setLibraryLoadingMore(false);
      }
    }
  }, [library, libraryLoadingMore, resolvedSettings.metadataCacheEnabled, view]);

  async function loadDetail(itemId: string, serverId?: string | null, refresh = false) {
    const key = itemCacheKey(itemId, serverId);
    const cached = resolvedSettings.metadataCacheEnabled ? detailCache.current.get(key) : null;
    if (cached && !refresh) {
      setDetail(cached);
      if (!cached.people.length && !cached.art.length && !cached.similar.length) {
        const currentRequest = ++requestId.current;
        void loadDetailMore(itemId, serverId, currentRequest);
      }
      void refreshDetailInBackground(itemId, serverId);
      return;
    }
    if (refresh) {
      detailCache.current.delete(key);
    }
    const currentRequest = ++requestId.current;
    const result = await runContentRequest(currentRequest, t("loading.detail"), () =>
      ipc.loadItem(itemId, serverId),
    );
    if (result && currentRequest === requestId.current) {
      if (resolvedSettings.metadataCacheEnabled) detailCache.current.set(itemCacheKey(itemId, result.item.serverId ?? serverId), result);
      setDetail(result);
      void loadDetailMore(itemId, result.item.serverId ?? serverId, currentRequest);
    }
  }

  async function loadDetailMore(itemId: string, serverId: string | null | undefined, parentRequest: number) {
    try {
      const more = await ipc.loadItemMore(itemId, serverId);
      if (parentRequest !== requestId.current || more.itemId !== itemId) return;
      setDetail((current) => {
        if (!current || current.item.id !== itemId || (serverId && current.item.serverId !== serverId)) return current;
        const next = {
          ...current,
          people: more.people,
          art: more.art,
          similar: more.similar,
        };
        if (resolvedSettings.metadataCacheEnabled) detailCache.current.set(itemCacheKey(itemId, current.item.serverId ?? serverId), next);
        return next;
      });
    } catch {
      // ponytail: secondary detail shelves can stay empty; the playable detail is already loaded.
    }
  }

  async function refreshDetailInBackground(itemId: string, serverId?: string | null) {
    try {
      const result = await ipc.loadItem(itemId, serverId);
      if (resolvedSettings.metadataCacheEnabled) detailCache.current.set(itemCacheKey(itemId, result.item.serverId ?? serverId), result);
      if (viewRef.current.name === "detail" && viewRef.current.id === itemId && (!viewRef.current.serverId || viewRef.current.serverId === (result.item.serverId ?? serverId))) {
        setDetail(result);
        void loadDetailMore(itemId, result.item.serverId ?? serverId, requestId.current);
      }
    } catch {
      // ponytail: keep last detail while the server refresh catches up.
    }
  }

  async function testLogin() {
    const serverName = await fetchServerName();
    const input = serverName ? { ...form, name: serverName } : form;
    const result = await run(t("loading.loginTest"), () =>
      ipc.testServerLogin(input),
    );
    if (result) {
      setTestedLogin(result);
      setForm((current) => ({ ...current, name: result.name }));
    }
  }

  async function saveServer() {
    if (!testedLogin) {
      return;
    }
    const result = await run(t("loading.saveServer"), () =>
      ipc.saveServer({ ...testedLogin, ...serverIconSelectionFromForm() }),
    );
    if (result) {
      setModalOpen(false);
      setForm(emptyForm);
      setTestedLogin(null);
      setEditingServerId("");
      resetServerScopedState();
      await refreshServers();
    }
  }

  async function saveServerIcon() {
    if (!editingServerId) return;
    const result = await run(t("loading.saveServer"), () =>
      ipc.updateServerIcon({ serverId: editingServerId, ...serverIconSelectionFromForm() }),
    );
    if (result) {
      setServers((current) => current.map((server) => (
        server.id === result.id ? {
          ...server,
          ...result,
          movieCount: result.movieCount ?? server.movieCount,
          seriesCount: result.seriesCount ?? server.seriesCount,
          episodeCount: result.episodeCount ?? server.episodeCount,
        } : server
      )));
      setHome((current) => current && current.server.id === result.id ? {
        ...current,
        server: {
          ...current.server,
          ...result,
          movieCount: result.movieCount ?? current.server.movieCount,
          seriesCount: result.seriesCount ?? current.server.seriesCount,
          episodeCount: result.episodeCount ?? current.server.episodeCount,
        },
      } : current);
      setModalOpen(false);
      setForm(emptyForm);
      setTestedLogin(null);
      setEditingServerId("");
    }
  }

  async function activateServer(serverId: string) {
    const result = await run(t("loading.switchServer"), () =>
      ipc.setActiveServer(serverId),
    );
    if (result) {
      setServers((current) => current.map((server) => (
        server.id === result.id ? {
          ...server,
          ...result,
          active: true,
          movieCount: result.movieCount ?? server.movieCount,
          seriesCount: result.seriesCount ?? server.seriesCount,
          episodeCount: result.episodeCount ?? server.episodeCount,
        } : { ...server, active: false }
      )));
      resetServerScopedState();
      replaceView({ name: "home" });
    }
  }

  async function deleteServer(serverId: string) {
    const server = servers.find((entry) => entry.id === serverId);
    const confirmed = await ask(
      t("server.deleteConfirmMessage", { name: server?.name ?? t("server.thisServer") }),
      { title: t("server.deleteConfirmTitle"), kind: "warning" },
    );
    if (!confirmed) return;

    const result = await run(t("loading.deleteServer"), () =>
      ipc.deleteServer(serverId),
    );
    if (result !== null) {
      resetServerScopedState();
      await refreshServers();
    }
  }

  async function importServerInfo() {
    try {
      const selected = await open({
        title: t("server.importDialogTitle"),
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (typeof selected !== "string") return;
      const result = await run(t("loading.importServers"), () => ipc.importServers(selected));
      if (result) {
        resetServerScopedState();
        await refreshServers();
      }
    } catch (err) {
      setError(String(err));
    }
  }

  async function exportServerInfo() {
    try {
      const targetPath = await saveDialog({
        title: t("server.exportDialogTitle"),
        defaultPath: "zplayer-servers.json",
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!targetPath) return;
      await run(t("loading.exportServers"), () => ipc.exportServers(targetPath));
    } catch (err) {
      setError(String(err));
    }
  }

  function editServer(server: SavedServer) {
    setEditingServerId(server.id);
    setTestedLogin(null);
    setForm({
      ...emptyForm,
      name: server.name,
      url: server.url,
      username: server.username,
      password: "",
      useSystemProxy: server.useSystemProxy,
      iconUrl: server.iconUrl ?? "",
      iconName: server.iconName ?? "",
    });
    setModalOpen(true);
  }

  function updateLibraryOptions(itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder, filters: LibraryFilters) {
    if (view.name !== "library") return;
    setLibrary(null);
    setView({ ...view, itemType, sortBy, sortOrder, filters });
  }

  async function toggleItemFavorite(item: MediaItem) {
    const nextValue = !item.favorite;
    const result = await run(nextValue ? t("loading.favoriteMedia") : t("loading.unfavoriteMedia"), () => ipc.markFavorite(item.id, nextValue, item.serverId));
    if (result !== null) {
      invalidatePlaybackCaches(item.id, item.serverId);
      if (view.name === "library") void loadLibrary(view.id, view.itemType ?? "", view.sortBy ?? "DateCreated", view.sortOrder ?? "Descending", view.filters ?? {});
    }
  }

  async function toggleItemPlayed(item: MediaItem) {
    const nextValue = !item.played;
    const result = await run(nextValue ? t("loading.markPlayed") : t("loading.markUnplayed"), () => ipc.markPlayed(item.id, nextValue, item.serverId));
    if (result !== null) {
      invalidatePlaybackCaches(item.id, item.serverId);
      if (view.name === "library") void loadLibrary(view.id, view.itemType ?? "", view.sortBy ?? "DateCreated", view.sortOrder ?? "Descending", view.filters ?? {});
    }
  }

  async function play(itemId: string, serverId?: string | null, mediaSourceId?: string, audioStreamIndex?: number, subtitleStreamIndex?: number, sources?: MediaVersion[], episodeIds?: string[]) {
    const knownItem = findKnownItem(itemId, serverId, home, library, detail);
    const title = detail?.item.id === itemId
      ? detail.item.name
      : knownItem?.name ?? t("player.playingTitle");
    const requestedServerId = serverId ?? knownItem?.serverId ?? activeServer?.id;
    const preference = playbackPreferences[scopedPlaybackPreferenceKey(requestedServerId, itemId, knownItem?.seriesId)];
    const source = mediaSourceForPlayback(sources, mediaSourceId ?? preference?.mediaSourceId ?? undefined, requestedServerId);
    const playbackItemId = source?.itemId ?? itemId;
    const playbackServerId = source?.serverId ?? requestedServerId;
    const preferredAudioIndex = audioStreamIndex ?? preferredStreamIndex(source?.audioStreams ?? [], preference?.audioStreamIndex, preference?.audioLanguage);
    const preferredSubtitleIndex = subtitleStreamIndex ?? preferredStreamIndex(source?.subtitleStreams ?? [], preference?.subtitleStreamIndex, preference?.subtitleLanguage);
    const subtitleSelection = resolveSubtitleSelection(source, preferredSubtitleIndex, resolvedSettings.subtitleMode);
    setPlaybackState(null);
    if (sources?.length) {
      setPlayerSources(sources);
    } else {
      setPlayerSources([]);
    }
    setSearchQuery("");
    setSearchOpen(false);
    setServerMenuOpen(false);
    setModalOpen(false);
    setTestedLogin(null);
    setEditingServerId("");
    setForm(emptyForm);
    const result = await run(t("loading.startMpv"), () =>
      ipc.playItem(
        playbackItemId,
        playbackServerId,
        source?.id ?? mediaSourceId,
        preferredAudioIndex,
        subtitleSelection.subtitleStreamIndex,
        subtitleSelection.subtitleStreamPosition,
      ),
    );
    if (result) {
      if (resolvedSettings.diagnosticsEnabled) setLastPlayResult(result);
      const resultServerId = result.serverId ?? playbackServerId ?? null;
      const episodeContext = episodeIds && resultServerId === requestedServerId ? episodePlaybackContext(result.itemId, episodeIds) : null;
      const playerView: View = {
        name: "player",
        itemId: result.itemId,
        serverId: resultServerId,
        serverName: result.serverName ?? source?.serverName ?? knownItem?.serverName ?? null,
        title,
        playSessionId: result.playSessionId,
        mediaSourceId: result.mediaSourceId ?? source?.id ?? mediaSourceId ?? null,
        subtitleStreamIndex: subtitleSelection.subtitleStreamIndex ?? null,
        episodeIds: episodeContext?.episodeIds ?? null,
        episodeIndex: episodeContext?.episodeIndex ?? null,
      };
      if (viewRef.current.name === "player") {
        setView(playerView);
      } else {
        openView(playerView);
      }
      rememberPlaybackPreference(preferencePayload(
        resultServerId,
        result.itemId,
        findKnownItem(result.itemId, resultServerId, home, library, detail)?.seriesId,
        source,
        preferredAudioIndex,
        subtitleSelection.subtitleStreamIndex,
      ));
    }
  }

  function episodeContextFromView(targetView: View = viewRef.current) {
    if (targetView.name !== "player" || !targetView.episodeIds || targetView.episodeIndex === null || targetView.episodeIndex === undefined) return null;
    return { episodeIds: targetView.episodeIds, episodeIndex: targetView.episodeIndex };
  }

  async function playRelativeEpisode(offset: -1 | 1, stopCurrent = true) {
    const context = episodeContextFromView();
    const nextItemId = relativeEpisodeId(context, offset);
    if (!nextItemId) return;
    const currentView = viewRef.current;
    if (stopCurrent && currentView.name === "player" && currentView.playSessionId) {
      exitingPlaybackSession.current = currentView.playSessionId;
      void ipc.controlPlayback(currentView.playSessionId, "stop").catch(() => {});
    }
    await play(nextItemId, currentView.name === "player" ? currentView.serverId : undefined, undefined, undefined, undefined, undefined, context?.episodeIds);
  }
  playRelativeEpisodeRef.current = playRelativeEpisode;

  async function switchPlayerSource(sourceId?: string, sourceServerId?: string | null) {
    if (view.name !== "player") return;
    if (!view.playSessionId) return;
    const sourceDetail = detailCache.current.get(itemCacheKey(view.itemId, view.serverId)) ?? (detail?.item.id === view.itemId && (!view.serverId || detail.item.serverId === view.serverId) ? detail : null);
    const sources = playerSources.length ? playerSources : sourceDetail?.mediaSources ?? [];
    if (sources.length < 2) {
      setError(t("errors.noOtherVersion"));
      return;
    }

    const currentIndex = sources.findIndex((source) => sourceMatchesView(source, view.mediaSourceId, view.serverId));
    const nextSource = sourceId
      ? sources.find((source) => source.id === sourceId && (!sourceServerId || !source.serverId || source.serverId === sourceServerId))
      : sources[(currentIndex + 1 + sources.length) % sources.length];
    if (!nextSource) return;
    exitingPlaybackSession.current = view.playSessionId;
    invalidatePlaybackCaches(view.itemId, view.serverId);
    setPlaybackState(null);
    setPlayerTransparent(false);
    void ipc.controlPlayback(view.playSessionId, "stop").catch(() => {});
    const subtitleSelection = resolveSubtitleSelection(nextSource, undefined, resolvedSettings.subtitleMode);
    const result = await run(t("loading.switchSource"), () => ipc.playItem(
      nextSource.itemId ?? view.itemId,
      nextSource.serverId ?? view.serverId,
      nextSource.id,
      undefined,
      subtitleSelection.subtitleStreamIndex,
      subtitleSelection.subtitleStreamPosition,
    ));
    if (result) {
      if (resolvedSettings.diagnosticsEnabled) setLastPlayResult(result);
      const switchedServerId = result.serverId ?? nextSource.serverId ?? view.serverId ?? null;
      setView({
        name: "player",
        itemId: result.itemId,
        serverId: switchedServerId,
        serverName: result.serverName ?? nextSource.serverName ?? view.serverName ?? null,
        title: view.title,
        playSessionId: result.playSessionId,
        mediaSourceId: result.mediaSourceId ?? nextSource.id,
        subtitleStreamIndex: subtitleSelection.subtitleStreamIndex ?? null,
        episodeIds: switchedServerId === view.serverId ? view.episodeIds ?? null : null,
        episodeIndex: switchedServerId === view.serverId ? view.episodeIndex ?? null : null,
      });
      rememberPlaybackPreference(preferencePayload(
        switchedServerId,
        result.itemId,
        findKnownItem(result.itemId, switchedServerId, home, library, detail)?.seriesId,
        nextSource,
        undefined,
        subtitleSelection.subtitleStreamIndex,
      ));
    }
  }

  function rememberPlaybackPreference(input: PlaybackPreferenceInput) {
    setPlaybackPreferences((current) => ({
      ...current,
      [scopedPlaybackPreferenceKey(input.serverId, input.itemId, input.seriesId)]: {
        mediaSourceId: input.mediaSourceId,
        audioStreamIndex: input.audioStreamIndex,
        audioLanguage: input.audioLanguage,
        subtitleStreamIndex: input.subtitleStreamIndex,
        subtitleLanguage: input.subtitleLanguage,
      },
    }));
    void ipc.savePlaybackPreference(input).catch(() => {});
  }

  async function controlPlayback(playSessionId: string | null | undefined, command: PlaybackCommand) {
    if (!playSessionId) return;
    optimisticUntil.current = performance.now() + 250;
    window.setTimeout(() => {
      setPlaybackState((current) => optimisticPlaybackState(current, command, resolvedSettings));
    }, 20);
    try {
      await ipc.controlPlayback(playSessionId, command);
    } catch (err) {
      const message = String(err);
      setError(message);
      if (message.includes("Playback session is not active") && view.name === "player" && view.playSessionId === playSessionId) {
        goBack();
      }
    }
  }

  async function toggleFullscreen() {
    try {
      const window = getCurrentWindow();
      await window.setFullscreen(!(await window.isFullscreen()));
    } catch (err) {
      setError(String(err));
    }
  }

  function exitPlayer(playSessionId: string | null | undefined) {
    if (!playSessionId) {
      goBack();
      return;
    }
    exitingPlaybackSession.current = playSessionId;
    if (view.name === "player") invalidatePlaybackCaches(view.itemId);
    goBack();
    void ipc.controlPlayback(playSessionId, "stop").catch((err) => {
      const message = String(err);
      if (!message.includes("Playback session is not active")) {
        setError(message);
      }
    });
  }
  function updateForm<K extends keyof ServerForm>(key: K, value: ServerForm[K]) {
    if (!isServerIconFormKey(key)) {
      setTestedLogin(null);
    }
    setForm((current) => ({ ...current, [key]: value }));
  }

  function serverIconSelectionFromForm(): ServerIconSelection {
    const clean = (value: string) => value.trim() || null;
    return {
      iconUrl: clean(form.iconUrl),
      iconName: clean(form.iconName),
    };
  }

  function canSaveEditingIconWithoutLogin() {
    if (!editingServerId || testedLogin) return false;
    const server = servers.find((entry) => entry.id === editingServerId);
    if (!server) return false;
    return form.name === server.name
      && form.url === server.url
      && form.username === server.username
      && form.password === ""
      && form.useSystemProxy === server.useSystemProxy;
  }

  async function autoFetchServerName() {
    const url = form.url.trim();
    if (!url || form.name.trim()) return;
    await fetchServerName();
  }

  async function fetchServerName() {
    const url = form.url.trim();
    if (!url) return "";
    try {
      const result = await ipc.fetchServerName({ url, serverType: form.serverType, useSystemProxy: form.useSystemProxy });
      if (result.name) {
        updateForm("name", result.name);
        return result.name;
      }
    } catch {
      // Keep manual server entry usable if name probing fails.
    }
    return "";
  }

  const openServers = useCallback(() => openView({ name: "servers" }), [openView]);
  const openSettings = useCallback(() => openView({ name: "settings" }), [openView]);
  const openCalendar = useCallback(() => openView({ name: "calendar" }), [openView]);
  const openLibrary = useCallback((id: string) => openView({ name: "library", id }), [openView]);
  const openFavorites = useCallback(() => {
    setLibrary(null);
    openView({
      name: "library",
      id: "",
      filters: { favorite: true },
    });
  }, [openView]);
  const openGenre = useCallback((genre: string) => {
    setLibrary(null);
    openView({
      name: "library",
      id: "",
      title: genre,
      filters: { genre },
    });
  }, [openView]);
  const openPerson = useCallback((personId: string, name: string) => {
    setLibrary(null);
    openView({
      name: "library",
      id: "",
      title: name,
      filters: { personId },
    });
  }, [openView]);
  const openCollection = useCallback((collectionId: string, title: string) => {
    setLibrary(null);
    openView(collectionLibraryView(collectionId, title));
  }, [openView]);
  const openDetail = useCallback((id: string, serverId?: string | null) => openView({ name: "detail", id, serverId }), [openView]);

  function rememberSearchTerm(query: string) {
    const term = query.trim();
    if (!term) return;
    setRecentSearchTerms((current) => {
      const next = [term, ...current.filter((value) => value !== term)].slice(0, 8);
      try {
        localStorage.setItem("zplayer:recent-searches", JSON.stringify(next));
      } catch {
        // ponytail: recent searches are convenience state; keep UI usable if storage is blocked.
      }
      return next;
    });
  }

  const openSearchResult = useCallback((itemId: string, serverId?: string | null) => {
    rememberSearchTerm(searchQuery);
    setSearchQuery("");
    setSearchOpen(false);
    openDetail(itemId, serverId);
  }, [openDetail, searchQuery]);

  const detailMatchesView = view.name === "detail" && !!detail && (
    (!view.serverId || detail.item.serverId === view.serverId)
    && (detail.item.id === view.id || detail.episodes.some((episode) => episode.id === view.id))
  );
  const showContent = !searchOpen && !searchQuery.trim();
  const waylandCompositor = !!linuxWindowDiagnostics?.waylandRequired && linuxWindowDiagnostics.waylandDisplaySet;

  return (
    <main className={`app theme-${resolvedSettings.theme} ${waylandCompositor ? "platform-wayland" : ""}`}>
      <section className="workspace">
        <div className="drag-strip" data-tauri-drag-region />
        {view.name !== "player" && (
          <TopBar
            searchOpen={searchOpen}
            searchQuery={searchQuery}
            chromeVisible={chromeVisible}
            searchInputRef={searchInputRef}
            onSearchQueryChange={setSearchQuery}
            onToggleSearch={() => {
              if (searchOpen) {
                setSearchQuery("");
                setSearchOpen(false);
              } else {
                setSearchOpen(true);
              }
            }}
          />
        )}

        {loading && <div className="status">{loading}...</div>}
        {error && <div className="error">{error}</div>}

        {view.name !== "player" && searchOpen && (
          <SearchOverlay
            results={searchResults}
            query={searchQuery}
            loading={searchLoading}
            posterDensity={resolvedSettings.posterDensity}
            recentTerms={recentSearchTerms}
            onUseRecentTerm={(term) => {
              setSearchQuery(term);
              setSearchOpen(true);
            }}
            onOpen={openSearchResult}
          />
        )}

        {showContent && view.name === "servers" && (
          <ServerView
            servers={servers}
            serverIconCatalogUrls={resolvedSettings.serverIconCatalogUrls}
            onAdd={() => setModalOpen(true)}
            onImport={importServerInfo}
            onExport={exportServerInfo}
            onActivate={activateServer}
            onEdit={editServer}
            onDelete={deleteServer}
            onBack={goBack}
          />
        )}
        {showContent && view.name === "settings" && (
          <SettingsView
            settings={settings}
            lastPlayResult={resolvedSettings.diagnosticsEnabled ? lastPlayResult : null}
            linuxWindowDiagnostics={linuxWindowDiagnostics}
            onBack={goBack}
            onSaveSettings={saveSettings}
          />
        )}
        {showContent && view.name === "calendar" && (
          <CalendarView
            payload={calendar}
            onBack={goBack}
            onOpenSettings={openSettings}
            onOpenSeries={openDetail}
          />
        )}
        {showContent && view.name === "home" && (
          <HomeView
            home={home}
            activeServer={activeServer}
            servers={servers}
            serverIconCatalogUrls={resolvedSettings.serverIconCatalogUrls}
            onAddServer={() => setModalOpen(true)}
            onOpenServers={openServers}
            onOpenSettings={openSettings}
            onOpenCalendar={openCalendar}
            onOpenFavorites={openFavorites}
            onActivateServer={activateServer}
            onOpenLibrary={openLibrary}
            onOpenItem={openDetail}
            onPlay={play}
            serverMenuOpen={serverMenuOpen}
            setServerMenuOpen={setServerMenuOpen}
            chromeVisible={chromeVisible}
          />
        )}
        {showContent && view.name === "library" && library?.library.id === view.id && (
          <LibraryView
            payload={library}
            title={view.filters?.favorite && !view.id ? t("home.favorites") : view.title}
            loadingMore={libraryLoadingMore}
            onBack={goBack}
            onOpenItem={openDetail}
            onLoadMore={loadMoreLibraryItems}
            itemType={view.itemType ?? ""}
            sortBy={view.sortBy ?? "DateCreated"}
            sortOrder={view.sortOrder ?? "Descending"}
            filters={view.filters ?? {}}
            posterDensity={resolvedSettings.posterDensity}
            onOptionsChange={updateLibraryOptions}
            onToggleFavorite={(item) => void toggleItemFavorite(item)}
            onTogglePlayed={(item) => void toggleItemPlayed(item)}
          />
        )}
        {showContent && view.name === "library" && library?.library.id !== view.id && <LoadingPage />}
        {showContent && detailMatchesView && detail && (
          <DetailView
            payload={detail}
            entryItemId={view.name === "detail" ? view.id : detail.item.id}
            onBack={goBack}
            onOpenItem={openDetail}
            onPlay={play}
            onRefresh={() => loadDetail(detail.item.id, detail.item.serverId, true)}
            onError={setError}
            onOpenGenre={openGenre}
            onOpenPerson={openPerson}
            onOpenCollection={openCollection}
          />
        )}
        {showContent && view.name === "detail" && !detailMatchesView && <LoadingPage />}
        {view.name === "player" && (
          <PlayerView
            title={view.title}
            state={playbackState}
            ready={!!playbackState?.videoReady && playerTransparent}
            onExit={() => exitPlayer(view.playSessionId)}
            onMinimize={() => void getCurrentWindow().minimize()}
            onToggleMaximize={() => void getCurrentWindow().toggleMaximize()}
            onToggleFullscreen={() => void toggleFullscreen()}
            onClose={() => void getCurrentWindow().close()}
            onCommand={(command) => controlPlayback(view.playSessionId, command)}
            onError={setError}
            canPlayPrevious={!!relativeEpisodeId(episodeContextFromView(view), -1)}
            canPlayNext={!!relativeEpisodeId(episodeContextFromView(view), 1)}
            onPlayPrevious={() => playRelativeEpisode(-1)}
            onPlayNext={() => playRelativeEpisode(1)}
            seekBackSeconds={resolvedSettings.seekBackSeconds}
            seekForwardSeconds={resolvedSettings.seekForwardSeconds}
            sources={playerSources}
            currentSourceId={view.mediaSourceId ?? null}
            currentServerId={view.serverId ?? null}
            initialSubtitleIndex={view.subtitleStreamIndex ?? undefined}
            onSwitchSource={switchPlayerSource}
            onPreferenceChange={(audioIndex, subtitleIndex) => {
              const source = playerSources.find((entry) => sourceMatchesView(entry, view.mediaSourceId, view.serverId)) ?? playerSources[0];
              rememberPlaybackPreference(preferencePayload(
                view.serverId,
                view.itemId,
                findKnownItem(view.itemId, view.serverId, home, library, detail)?.seriesId,
                source,
                audioIndex,
                subtitleIndex,
              ));
            }}
          />
        )}
      </section>

      {modalOpen && (
        <ServerModal
          editingServerId={editingServerId}
          form={form}
          testedLogin={testedLogin}
          showPassword={showPassword}
          canSaveWithoutLogin={canSaveEditingIconWithoutLogin()}
          iconCatalogUrls={resolvedSettings.serverIconCatalogUrls}
          onClose={() => {
            setModalOpen(false);
            setEditingServerId("");
            setForm(emptyForm);
            setTestedLogin(null);
          }}
          onSubmit={() => {
            if (testedLogin) {
              void saveServer();
            } else if (canSaveEditingIconWithoutLogin()) {
              void saveServerIcon();
            } else {
              void testLogin();
            }
          }}
          onTestLogin={() => void testLogin()}
          onAutoFetchServerName={() => void autoFetchServerName()}
          onTogglePassword={() => setShowPassword((value) => !value)}
          onUpdateForm={updateForm}
        />
      )}
    </main>
  );
}


export default App;
