import { normalizeLanguage, type AppLanguage } from "./i18nLogic";
export type { AppLanguage } from "./i18nLogic";

export type View =
  | { name: "servers" }
  | { name: "settings" }
  | { name: "home" }
  | { name: "library"; id: string; title?: string; itemType?: LibraryItemType; sortBy?: LibrarySortBy; sortOrder?: LibrarySortOrder; filters?: LibraryFilters }
  | { name: "detail"; id: string; serverId?: string | null }
  | {
      name: "player";
      itemId: string;
      serverId?: string | null;
      serverName?: string | null;
      title: string;
      playSessionId?: string | null;
      mediaSourceId?: string | null;
      subtitleStreamIndex?: number | null;
      episodeIds?: string[] | null;
      episodeIndex?: number | null;
    };

export type LibraryItemType = "" | "Movie" | "Series" | "Episode" | "Video";
export type LibrarySortBy = "DateCreated" | "SortName" | "PremiereDate" | "CommunityRating";
export type LibrarySortOrder = "Ascending" | "Descending";
export type LibraryPlayedFilter = "" | "played" | "unplayed";
export type LibraryFilters = {
  played?: LibraryPlayedFilter;
  favorite?: boolean;
  genre?: string;
  personId?: string;
  collectionId?: string;
};
export type AppTheme = "dark" | "midnight";
export type SubtitleMode = "auto" | "off";
export type PosterDensity = "comfortable" | "compact";

export type SavedServer = {
  id: string;
  name: string;
  url: string;
  username: string;
  active: boolean;
  useSystemProxy: boolean;
  movieCount?: number | null;
  seriesCount?: number | null;
};

export type ServerImportResult = {
  imported: number;
  added: number;
  updated: number;
};

export type AppSettings = {
  mpvPath?: string | null;
  defaultVolume?: number;
  seekBackSeconds?: number;
  seekForwardSeconds?: number;
  subtitleMode?: SubtitleMode;
  posterDensity?: PosterDensity;
  metadataCacheEnabled?: boolean;
  theme?: AppTheme;
  diagnosticsEnabled?: boolean;
  autoplayNextEpisode?: boolean;
  language?: AppLanguage;
};

export type LinuxWindowDiagnostics = {
  xdgSessionType?: string | null;
  waylandDisplaySet: boolean;
  gdkBackend?: string | null;
  winitUnixBackend?: string | null;
  webkitDisableDmabufRenderer: boolean;
  renderGpuPreference?: string | null;
  nvidiaDriverAvailable: boolean;
  nvidiaPrimeRenderOffload: boolean;
  glxVendorLibraryName?: string | null;
  vulkanOptimusLayer?: string | null;
  waylandRequired: boolean;
  gdkBackendWayland: boolean;
  winitBackendWayland: boolean;
  nativeVideoOverlay: boolean;
  nativeVideoRenderCount: number;
  nativeVideoRenderWidth: number;
  nativeVideoRenderHeight: number;
  nativeVideoRenderFramebuffer: number;
  nativeVideoRenderStatus: number;
  nativeVideoRenderContext: boolean;
  opaqueWindow: boolean;
};

export type ResolvedAppSettings = {
  mpvPath: string;
  defaultVolume: number;
  seekBackSeconds: number;
  seekForwardSeconds: number;
  subtitleMode: SubtitleMode;
  posterDensity: PosterDensity;
  metadataCacheEnabled: boolean;
  theme: AppTheme;
  diagnosticsEnabled: boolean;
  autoplayNextEpisode: boolean;
  language: AppLanguage;
};

export const defaultAppSettings: ResolvedAppSettings = {
  mpvPath: "",
  defaultVolume: 100,
  seekBackSeconds: 10,
  seekForwardSeconds: 30,
  subtitleMode: "auto",
  posterDensity: "comfortable",
  metadataCacheEnabled: true,
  theme: "dark",
  diagnosticsEnabled: false,
  autoplayNextEpisode: true,
  language: "auto",
};

export function withAppSettingsDefaults(settings: AppSettings = {}): ResolvedAppSettings {
  return {
    ...defaultAppSettings,
    ...settings,
    mpvPath: settings.mpvPath ?? "",
    defaultVolume: settings.defaultVolume ?? defaultAppSettings.defaultVolume,
    seekBackSeconds: settings.seekBackSeconds ?? defaultAppSettings.seekBackSeconds,
    seekForwardSeconds: settings.seekForwardSeconds ?? defaultAppSettings.seekForwardSeconds,
    subtitleMode: settings.subtitleMode ?? defaultAppSettings.subtitleMode,
    posterDensity: settings.posterDensity ?? defaultAppSettings.posterDensity,
    metadataCacheEnabled: settings.metadataCacheEnabled ?? defaultAppSettings.metadataCacheEnabled,
    theme: settings.theme ?? defaultAppSettings.theme,
    diagnosticsEnabled: settings.diagnosticsEnabled ?? defaultAppSettings.diagnosticsEnabled,
    autoplayNextEpisode: settings.autoplayNextEpisode ?? defaultAppSettings.autoplayNextEpisode,
    language: normalizeLanguage(settings.language),
  };
}

export type LoginResult = {
  id: string;
  name: string;
  url: string;
  username: string;
  userId: string;
  accessToken: string;
  useSystemProxy: boolean;
};

export type MediaLibrary = {
  id: string;
  name: string;
  collectionType?: string | null;
  imageUrl?: string | null;
};

export type MediaItem = {
  id: string;
  serverId?: string | null;
  serverName?: string | null;
  name: string;
  itemType: string;
  year?: number | null;
  overview?: string | null;
  communityRating?: number | null;
  runTimeTicks?: number | null;
  playbackPositionTicks?: number | null;
  playedPercentage?: number | null;
  childCount?: number | null;
  seasonNumber?: number | null;
  episodeNumber?: number | null;
  seriesName?: string | null;
  seriesId?: string | null;
  seasonName?: string | null;
  seasonId?: string | null;
  genres: string[];
  officialRating?: string | null;
  studios: string[];
  tags: string[];
  played: boolean;
  favorite: boolean;
  primaryImageUrl?: string | null;
  backdropUrl?: string | null;
  logoUrl?: string | null;
};

export type HomePayload = {
  server: SavedServer;
  libraries: MediaLibrary[];
  libraryLatest: LibraryLatestPayload[];
  latest: MediaItem[];
  recommendedMovies: MediaItem[];
  recommendedShows: MediaItem[];
  resumeItems: MediaItem[];
  favoriteItems: MediaItem[];
  recentItems: MediaItem[];
};

export type HomeMorePayload = {
  serverId: string;
  libraryLatest: LibraryLatestPayload[];
  recommendedMovies: MediaItem[];
  recommendedShows: MediaItem[];
  resumeItems: MediaItem[];
  favoriteItems: MediaItem[];
  recentItems: MediaItem[];
};

export type SearchPayload = {
  items: MediaItem[];
};

export type LibraryPayload = {
  library: MediaLibrary;
  items: MediaItem[];
  totalCount: number;
  startIndex: number;
  limit: number;
  hasMore: boolean;
};

export type LibraryLatestPayload = {
  library: MediaLibrary;
  items: MediaItem[];
};

export type ItemDetailPayload = {
  item: MediaItem;
  children: MediaItem[];
  seasons: MediaItem[];
  episodes: MediaItem[];
  episodeTotalCount?: number | null;
  mediaSources: MediaVersion[];
  people: MediaPerson[];
  art: MediaArt[];
  similar: MediaItem[];
};

export type ItemMorePayload = {
  itemId: string;
  people: MediaPerson[];
  art: MediaArt[];
  similar: MediaItem[];
};

export type MediaVersion = {
  id: string;
  itemId?: string | null;
  serverId?: string | null;
  serverName?: string | null;
  name: string;
  container?: string | null;
  path?: string | null;
  protocol?: string | null;
  bitrate?: number | null;
  size?: number | null;
  videoCodec?: string | null;
  videoDisplayTitle?: string | null;
  videoRange?: string | null;
  videoProfile?: string | null;
  videoLevel?: number | null;
  aspectRatio?: string | null;
  interlaced?: boolean | null;
  bitDepth?: number | null;
  pixelFormat?: string | null;
  resolution?: string | null;
  frameRate?: number | null;
  audioCodec?: string | null;
  audioDisplayTitle?: string | null;
  audioTitle?: string | null;
  audioLanguage?: string | null;
  channelLayout?: string | null;
  audioChannels?: number | null;
  audioBitrate?: number | null;
  sampleRate?: number | null;
  audioExternal?: boolean | null;
  audioDefault?: boolean | null;
  audioStreams: StreamInfo[];
  subtitleCount: number;
  subtitleLanguages: string[];
  subtitleStreams: StreamInfo[];
};

export type StreamInfo = {
  index?: number | null;
  displayTitle?: string | null;
  title?: string | null;
  language?: string | null;
  codec?: string | null;
  channelLayout?: string | null;
  channels?: number | null;
  bitrate?: number | null;
  sampleRate?: number | null;
  isExternal?: boolean | null;
  isDefault?: boolean | null;
};

export type MediaPerson = {
  id?: string | null;
  name: string;
  role?: string | null;
  personType?: string | null;
  imageUrl?: string | null;
};

export type MediaArt = {
  imageType: string;
  url: string;
};

export type PlayResult = {
  itemId: string;
  serverId?: string | null;
  serverName?: string | null;
  mediaSourceId?: string | null;
  playSessionId: string;
  url: string;
  logPath: string;
  logTail: string;
};

export type PlaybackCommand = "toggle_pause" | "seek_back" | "seek_forward" | `seek_absolute:${number}` | "volume_down" | "volume_up" | `volume_set:${number}` | "toggle_mute" | "audio_next" | "subtitle_next" | `audio_set:${number}` | `subtitle_set:${number}` | "speed_down" | "speed_up" | `speed_set:${number}` | `audio_delay_set:${number}` | `subtitle_delay_set:${number}` | `external_subtitle:${string}` | "resume" | "stop";

export type PlaybackState = {
  timePos?: number | null;
  duration?: number | null;
  paused: boolean;
  muted: boolean;
  volume?: number | null;
  speed?: number | null;
  cacheSpeed?: number | null;
  videoReady: boolean;
};

export type PlaybackPreferenceInput = {
  serverId?: string | null;
  itemId: string;
  seriesId?: string | null;
  mediaSourceId?: string | null;
  audioStreamIndex?: number | null;
  audioLanguage?: string | null;
  subtitleStreamIndex?: number | null;
  subtitleLanguage?: string | null;
};

export type PlaybackPreference = Omit<PlaybackPreferenceInput, "serverId" | "itemId" | "seriesId">;

export type ServerForm = {
  serverType: "emby" | "jellyfin";
  name: string;
  url: string;
  username: string;
  password: string;
  useSystemProxy: boolean;
};

export const emptyForm: ServerForm = {
  serverType: "emby",
  name: "",
  url: "http://127.0.0.1:8096",
  username: "",
  password: "",
  useSystemProxy: true,
};
