import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useLayoutEffect } from "react";
import { useDeferredValue } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ipc } from "./ipc";
import { ServerModal } from "./ServerModal";
import { TopBar } from "./TopBar";
import { DetailView, HomeView, LibraryView, LoadingPage, PlayerView, SearchOverlay, ServerView, SettingsView } from "./views";
import type { AppSettings, HomePayload, ItemDetailPayload, LibraryFilters, LibraryItemType, LibraryPayload, LibrarySortBy, LibrarySortOrder, LoginResult, MediaItem, MediaVersion, PlaybackCommand, PlaybackState, PlayResult, ResolvedAppSettings, SavedServer, ServerForm, View } from "./types";
import { emptyForm, withAppSettingsDefaults } from "./types";
import { findKnownItem, libraryKey } from "./appLogic";
import "./App.css";

type HistoryEntry = {
  view: View;
  scrollTop: number;
};

type PlaybackStoppedEvent = {
  itemId: string;
  playSessionId: string;
  failed: boolean;
};

type SubtitleSelection = {
  subtitleStreamIndex?: number;
  subtitleStreamPosition?: number;
};

function samePlaybackState(left: PlaybackState | null, right: PlaybackState | null) {
  return left?.timePos === right?.timePos
    && left?.duration === right?.duration
    && left?.paused === right?.paused
    && left?.muted === right?.muted
    && left?.volume === right?.volume
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
  if (command.startsWith("volume_set:")) {
    const volume = Number(command.slice("volume_set:".length));
    return Number.isFinite(volume) ? { ...current, muted: volume === 0, volume: Math.min(Math.max(volume, 0), 100) } : current;
  }
  return current;
}

function mediaSourceForPlayback(sources: MediaVersion[] | undefined, mediaSourceId?: string) {
  if (!sources?.length) return undefined;
  return sources.find((source) => source.id === mediaSourceId) ?? sources[0];
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
  const [view, setView] = useState<View>({ name: "home" });
  const [servers, setServers] = useState<SavedServer[]>([]);
  const [settings, setSettings] = useState<AppSettings>({});
  const [home, setHome] = useState<HomePayload | null>(null);
  const [library, setLibrary] = useState<LibraryPayload | null>(null);
  const [libraryLoadingMore, setLibraryLoadingMore] = useState(false);
  const [detail, setDetail] = useState<ItemDetailPayload | null>(null);
  const [loading, setLoading] = useState("");
  const [error, setError] = useState("");
  const [playbackState, setPlaybackState] = useState<PlaybackState | null>(null);
  const [playerTransparent, setPlayerTransparent] = useState(false);
  const [playerSources, setPlayerSources] = useState<MediaVersion[]>([]);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const deferredSearchQuery = useDeferredValue(searchQuery);
  const [searchResults, setSearchResults] = useState<MediaItem[]>([]);
  const [searchLoading, setSearchLoading] = useState(false);
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
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const homeCache = useRef(new Map<string, HomePayload>());
  const detailCache = useRef(new Map<string, ItemDetailPayload>());
  const libraryCache = useRef(new Map<string, LibraryPayload>());
  const exitingPlaybackSession = useRef("");
  const chromeVisibleRef = useRef(true);
  const optimisticUntil = useRef(0);
  const resumedPlaybackSession = useRef("");
  const refreshAfterPlaybackStopRef = useRef<(itemId?: string | null, targetView?: View) => void>(() => {});

  const activeServer = useMemo(
    () => servers.find((server) => server.active) ?? null,
    [servers],
  );
  const resolvedSettings = useMemo(() => withAppSettingsDefaults(settings), [settings]);

  useEffect(() => {
    if (resolvedSettings.metadataCacheEnabled) return;
    homeCache.current.clear();
    detailCache.current.clear();
    libraryCache.current.clear();
  }, [resolvedSettings.metadataCacheEnabled]);

  useEffect(() => {
    if (!resolvedSettings.diagnosticsEnabled) setLastPlayResult(null);
  }, [resolvedSettings.diagnosticsEnabled]);
  useEffect(() => {
    void refreshServers();
    void loadSettings();
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

    function handleScroll() {
      const nextScrollTop = element.scrollTop;
      const scrollingUp = nextScrollTop < lastScrollTop.current;
      const nextVisible = nextScrollTop < 24 || scrollingUp || searchOpen || serverMenuOpen;
      if (chromeVisibleRef.current !== nextVisible) {
        chromeVisibleRef.current = nextVisible;
        setChromeVisible(nextVisible);
      }
      lastScrollTop.current = nextScrollTop;
    }

    element.addEventListener("scroll", handleScroll, { passive: true });
    handleScroll();
    return () => element.removeEventListener("scroll", handleScroll);
  }, [searchOpen, serverMenuOpen]);

  useEffect(() => {
    if (view.name === "home") {
      void loadHome();
    }
    if (view.name === "library") {
      void loadLibrary(view.id, view.itemType ?? "", view.sortBy ?? "DateCreated", view.sortOrder ?? "Descending", view.filters ?? {});
    }
    if (view.name === "detail") {
      void loadDetail(view.id);
    }
  }, [view]);

  useLayoutEffect(() => {
    const playing = view.name === "player" && playbackState?.videoReady && playerTransparent;
    document.documentElement.classList.toggle("playing-embedded", playing);
    document.body.classList.toggle("playing-embedded", playing);
    return () => {
      document.documentElement.classList.remove("playing-embedded");
      document.body.classList.remove("playing-embedded");
    };
  }, [view.name, playbackState?.videoReady, playerTransparent]);

  useEffect(() => {
    setPlayerTransparent(false);
    if (view.name !== "player" || !view.playSessionId || !playbackState?.videoReady) return;
    setPlayerTransparent(true);
  }, [view.name, view.name === "player" ? view.playSessionId : null, playbackState?.videoReady]);

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
    const cached = detailCache.current.get(view.itemId) ?? (detail?.item.id === view.itemId ? detail : null);
    if (cached?.mediaSources.length) {
      setPlayerSources(cached.mediaSources);
      return;
    }
    let cancelled = false;
    ipc.loadMediaSources(view.itemId)
      .then((sources) => {
        if (!cancelled) setPlayerSources(sources);
      })
      .catch(() => {
        if (!cancelled) setPlayerSources([]);
      });
    return () => {
      cancelled = true;
    };
  }, [view, detail?.item.id, playerSources.length]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | null = null;

    listen<PlaybackStoppedEvent>("playback-stopped", (event) => {
      if (cancelled) return;
      invalidatePlaybackCaches(event.payload.itemId);
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
          setError("mpv 播放异常退出。");
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
  }, []);

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

  async function refreshServers() {
    const result = await run("加载服务器", () =>
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
    const result = await run("加载设置", () => ipc.loadSettings());
    if (result) {
      setSettings(result);
    }
  }

  async function saveSettings(next: AppSettings) {
    const result = await run("保存设置", () => ipc.saveSettings(withAppSettingsDefaults(next)));
    if (result) {
      setSettings(result);
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

  function invalidatePlaybackCaches(itemId?: string | null) {
    homeCache.current.clear();
    if (itemId) detailCache.current.delete(itemId);
  }

  function refreshAfterPlaybackStop(itemId?: string | null, targetView: View = viewRef.current) {
    const currentView = targetView;
    if (currentView.name === "home") {
      void loadHome();
    } else if (currentView.name === "detail") {
      void loadDetail(currentView.id, true);
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
      if (!cached.libraryLatest.length && !cached.recommendedMovies.length && !cached.recommendedShows.length) {
        const currentRequest = ++requestId.current;
        void loadHomeMore(cached.server.id, currentRequest);
      }
      return;
    }
    const currentRequest = ++requestId.current;
    const result = await run("加载首页", () => ipc.loadHome());
    if (result && currentRequest === requestId.current) {
      if (resolvedSettings.metadataCacheEnabled) homeCache.current.set(result.server.id, result);
      setHome(result);
      void loadHomeMore(result.server.id, currentRequest);
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
        };
        if (resolvedSettings.metadataCacheEnabled) homeCache.current.set(more.serverId, next);
        return next;
      });
    } catch {
      // ponytail: non-critical shelves can stay empty; surface only the first-screen failure.
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
    const result = await run("加载媒体库", () =>
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
      setError(String(err));
    } finally {
      setLibraryLoadingMore(false);
    }
  }, [library, libraryLoadingMore, resolvedSettings.metadataCacheEnabled, view]);

  async function loadDetail(itemId: string, refresh = false) {
    const cached = resolvedSettings.metadataCacheEnabled ? detailCache.current.get(itemId) : null;
    if (cached && !refresh) {
      setDetail(cached);
      if (!cached.people.length && !cached.art.length && !cached.similar.length) {
        const currentRequest = ++requestId.current;
        void loadDetailMore(itemId, currentRequest);
      }
      void refreshDetailInBackground(itemId);
      return;
    }
    if (refresh) {
      detailCache.current.delete(itemId);
    }
    const currentRequest = ++requestId.current;
    const result = await run("加载详情", () =>
      ipc.loadItem(itemId),
    );
    if (result && currentRequest === requestId.current) {
      if (resolvedSettings.metadataCacheEnabled) detailCache.current.set(itemId, result);
      setDetail(result);
      void loadDetailMore(itemId, currentRequest);
    }
  }

  async function loadDetailMore(itemId: string, parentRequest: number) {
    try {
      const more = await ipc.loadItemMore(itemId);
      if (parentRequest !== requestId.current || more.itemId !== itemId) return;
      setDetail((current) => {
        if (!current || current.item.id !== itemId) return current;
        const next = {
          ...current,
          people: more.people,
          art: more.art,
          similar: more.similar,
        };
        if (resolvedSettings.metadataCacheEnabled) detailCache.current.set(itemId, next);
        return next;
      });
    } catch {
      // ponytail: secondary detail shelves can stay empty; the playable detail is already loaded.
    }
  }

  async function refreshDetailInBackground(itemId: string) {
    try {
      const result = await ipc.loadItem(itemId);
      if (resolvedSettings.metadataCacheEnabled) detailCache.current.set(itemId, result);
      if (viewRef.current.name === "detail" && viewRef.current.id === itemId) {
        setDetail(result);
        void loadDetailMore(itemId, requestId.current);
      }
    } catch {
      // ponytail: keep last detail while the server refresh catches up.
    }
  }

  async function testLogin() {
    const serverName = await fetchServerName();
    const input = serverName ? { ...form, name: serverName } : form;
    const result = await run("登录检测", () =>
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
    const result = await run("保存服务器", () =>
      ipc.saveServer(testedLogin),
    );
    if (result) {
      setModalOpen(false);
      setForm(emptyForm);
      setTestedLogin(null);
      setEditingServerId("");
      homeCache.current.clear();
      detailCache.current.clear();
      libraryCache.current.clear();
      setHome(null);
      setLibrary(null);
      setDetail(null);
      await refreshServers();
    }
  }

  async function activateServer(serverId: string) {
    const result = await run("切换服务器", () =>
      ipc.setActiveServer(serverId),
    );
    if (result) {
      setServers((current) => current.map((server) => (
        server.id === result.id ? { ...server, ...result, active: true } : { ...server, active: false }
      )));
      homeCache.current.clear();
      detailCache.current.clear();
      libraryCache.current.clear();
      setHome(null);
      setLibrary(null);
      setDetail(null);
      replaceView({ name: "home" });
    }
  }

  async function deleteServer(serverId: string) {
    const result = await run("删除服务器", () =>
      ipc.deleteServer(serverId),
    );
    if (result !== null) {
      homeCache.current.clear();
      detailCache.current.clear();
      libraryCache.current.clear();
      setHome(null);
      setLibrary(null);
      setDetail(null);
      await refreshServers();
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
    });
    setModalOpen(true);
  }

  function updateLibraryOptions(itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder, filters: LibraryFilters) {
    if (view.name !== "library") return;
    setLibrary(null);
    setView({ ...view, itemType, sortBy, sortOrder, filters });
  }

  async function play(itemId: string, mediaSourceId?: string, audioStreamIndex?: number, subtitleStreamIndex?: number, sources?: MediaVersion[]) {
    const title = detail?.item.id === itemId
      ? detail.item.name
      : findKnownItem(itemId, home, library, detail)?.name ?? "正在播放";
    const source = mediaSourceForPlayback(sources, mediaSourceId);
    const subtitleSelection = resolveSubtitleSelection(source, subtitleStreamIndex, resolvedSettings.subtitleMode);
    setPlaybackState(null);
    if (sources?.length) {
      setPlayerSources(sources);
    }
    setSearchQuery("");
    setSearchOpen(false);
    setServerMenuOpen(false);
    setModalOpen(false);
    setTestedLogin(null);
    setEditingServerId("");
    setForm(emptyForm);
    const result = await run("启动 mpv", () =>
      ipc.playItem(
        itemId,
        mediaSourceId,
        audioStreamIndex,
        subtitleSelection.subtitleStreamIndex,
        subtitleSelection.subtitleStreamPosition,
      ),
    );
    if (result) {
      if (resolvedSettings.diagnosticsEnabled) setLastPlayResult(result);
      openView({
        name: "player",
        itemId: result.itemId,
        title,
        playSessionId: result.playSessionId,
        mediaSourceId: result.mediaSourceId ?? mediaSourceId ?? null,
        subtitleStreamIndex: subtitleSelection.subtitleStreamIndex ?? null,
      });
    }
  }

  async function switchPlayerSource(sourceId?: string) {
    if (view.name !== "player") return;
    if (!view.playSessionId) return;
    const sourceDetail = detailCache.current.get(view.itemId) ?? (detail?.item.id === view.itemId ? detail : null);
    const sources = playerSources.length ? playerSources : sourceDetail?.mediaSources ?? [];
    if (sources.length < 2) {
      setError("当前条目没有其他版本可切换。");
      return;
    }

    const currentIndex = sources.findIndex((source) => source.id === view.mediaSourceId);
    const nextSource = sourceId
      ? sources.find((source) => source.id === sourceId)
      : sources[(currentIndex + 1 + sources.length) % sources.length];
    if (!nextSource) return;
    exitingPlaybackSession.current = view.playSessionId;
    invalidatePlaybackCaches(view.itemId);
    setPlaybackState(null);
    setPlayerTransparent(false);
    void ipc.controlPlayback(view.playSessionId, "stop").catch(() => {});
    const subtitleSelection = resolveSubtitleSelection(nextSource, undefined, resolvedSettings.subtitleMode);
    const result = await run("切换版本", () => ipc.playItem(
      view.itemId,
      nextSource.id,
      undefined,
      subtitleSelection.subtitleStreamIndex,
      subtitleSelection.subtitleStreamPosition,
    ));
    if (result) {
      if (resolvedSettings.diagnosticsEnabled) setLastPlayResult(result);
      setView({
        name: "player",
        itemId: result.itemId,
        title: view.title,
        playSessionId: result.playSessionId,
        mediaSourceId: result.mediaSourceId ?? nextSource.id,
        subtitleStreamIndex: subtitleSelection.subtitleStreamIndex ?? null,
      });
    }
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
    setTestedLogin(null);
    setForm((current) => ({ ...current, [key]: value }));
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
      // 静默失败，不影响用户继续操作
    }
    return "";
  }

  const openServers = useCallback(() => openView({ name: "servers" }), [openView]);
  const openSettings = useCallback(() => openView({ name: "settings" }), [openView]);
  const openLibrary = useCallback((id: string) => openView({ name: "library", id }), [openView]);
  const openFavorites = useCallback(() => {
    setLibrary(null);
    openView({
      name: "library",
      id: "",
      title: "收藏",
      filters: { favorite: true },
    });
  }, [openView]);
  const openDetail = useCallback((id: string) => openView({ name: "detail", id }), [openView]);

  const openSearchResult = useCallback((itemId: string) => {
    setSearchQuery("");
    setSearchOpen(false);
    openDetail(itemId);
  }, [openDetail]);

  const detailMatchesView = view.name === "detail" && !!detail && (
    detail.item.id === view.id || detail.episodes.some((episode) => episode.id === view.id)
  );

  return (
    <main className={`app theme-${resolvedSettings.theme}`}>
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

        {view.name !== "player" && searchQuery.trim() && (
          <SearchOverlay
            results={searchResults}
            query={searchQuery}
            loading={searchLoading}
            posterDensity={resolvedSettings.posterDensity}
            onOpen={openSearchResult}
          />
        )}

        {!searchQuery.trim() && view.name === "servers" && (
          <ServerView
            servers={servers}
            onAdd={() => setModalOpen(true)}
            onActivate={activateServer}
            onEdit={editServer}
            onDelete={deleteServer}
            onBack={goBack}
          />
        )}
        {!searchQuery.trim() && view.name === "settings" && (
          <SettingsView
            settings={settings}
            lastPlayResult={resolvedSettings.diagnosticsEnabled ? lastPlayResult : null}
            onBack={goBack}
            onSaveSettings={saveSettings}
          />
        )}
        {!searchQuery.trim() && view.name === "home" && (
          <HomeView
            home={home}
            activeServer={activeServer}
            servers={servers}
            onAddServer={() => setModalOpen(true)}
            onOpenServers={openServers}
            onOpenSettings={openSettings}
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
        {!searchQuery.trim() && view.name === "library" && library?.library.id === view.id && (
          <LibraryView
            payload={library}
            title={view.title}
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
          />
        )}
        {!searchQuery.trim() && view.name === "library" && library?.library.id !== view.id && <LoadingPage />}
        {!searchQuery.trim() && detailMatchesView && detail && (
          <DetailView
            payload={detail}
            entryItemId={view.name === "detail" ? view.id : detail.item.id}
            onBack={goBack}
            onOpenItem={openDetail}
            onPlay={play}
            onRefresh={() => loadDetail(detail.item.id, true)}
            onError={setError}
          />
        )}
        {!searchQuery.trim() && view.name === "detail" && !detailMatchesView && <LoadingPage />}
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
            seekBackSeconds={resolvedSettings.seekBackSeconds}
            seekForwardSeconds={resolvedSettings.seekForwardSeconds}
            sources={playerSources}
            currentSourceId={view.mediaSourceId ?? null}
            initialSubtitleIndex={view.subtitleStreamIndex ?? undefined}
            onSwitchSource={switchPlayerSource}
          />
        )}
      </section>

      {modalOpen && (
        <ServerModal
          editingServerId={editingServerId}
          form={form}
          testedLogin={testedLogin}
          showPassword={showPassword}
          onClose={() => {
            setModalOpen(false);
            setEditingServerId("");
            setForm(emptyForm);
            setTestedLogin(null);
          }}
          onSubmit={() => {
            if (testedLogin) {
              void saveServer();
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
