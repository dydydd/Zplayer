import type { HomePayload, ItemDetailPayload, LibraryFilters, LibraryItemType, LibraryPayload, LibrarySortBy, LibrarySortOrder, MediaItem, MediaVersion, View } from "./types";

export function libraryKey(libraryId: string, itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder, filters: LibraryFilters = {}) {
  return `${libraryId}:${itemType}:${sortBy}:${sortOrder}:${JSON.stringify(filters)}`;
}

export function collectionLibraryView(collectionId: string, title: string): View {
  return {
    name: "library",
    id: "",
    title,
    filters: { collectionId },
  };
}

export function findKnownItem(
  itemId: string,
  serverId: string | null | undefined,
  home: HomePayload | null,
  library: LibraryPayload | null,
  detail: ItemDetailPayload | null,
) {
  const rows: MediaItem[] = [
    ...(detail?.item ? [detail.item] : []),
    ...(home?.latest ?? []),
    ...(home?.recommendedMovies ?? []),
    ...(home?.recommendedShows ?? []),
    ...(home?.resumeItems ?? []),
    ...(home?.favoriteItems ?? []),
    ...(home?.recentItems ?? []),
    ...(library?.items ?? []),
    ...(detail?.episodes ?? []),
    ...(detail?.children ?? []),
    ...(detail?.similar ?? []),
  ];
  return rows.find((item) => item.id === itemId && (!serverId || !item.serverId || item.serverId === serverId));
}

export type EpisodePlaybackContext = {
  episodeIds: string[];
  episodeIndex: number;
};

export function episodePlaybackContext(itemId: string, episodeIds: string[]): EpisodePlaybackContext | null {
  const episodeIndex = episodeIds.indexOf(itemId);
  return episodeIndex >= 0 ? { episodeIds, episodeIndex } : null;
}

export function relativeEpisodeId(context: EpisodePlaybackContext | null | undefined, offset: -1 | 1) {
  if (!context) return undefined;
  return context.episodeIds[context.episodeIndex + offset];
}

export function streamLanguage(stream?: { language?: string | null }) {
  return stream?.language?.trim().toLowerCase() || undefined;
}

export function playbackPreferenceKey(itemId: string, seriesId?: string | null) {
  return seriesId ? `series:${seriesId}` : `item:${itemId}`;
}

export function scopedPlaybackPreferenceKey(serverId: string | null | undefined, itemId: string, seriesId?: string | null) {
  return `${serverId ?? ""}:${playbackPreferenceKey(itemId, seriesId)}`;
}

export function preferredStreamIndex(
  streams: { index?: number | null; language?: string | null }[],
  preferredIndex?: number | null,
  preferredLanguage?: string | null,
) {
  if (preferredIndex !== undefined && preferredIndex !== null && (preferredIndex < 0 || streams.some((stream) => stream.index === preferredIndex))) {
    return preferredIndex;
  }
  const language = preferredLanguage?.trim().toLowerCase();
  return language ? streams.find((stream) => streamLanguage(stream) === language)?.index ?? undefined : undefined;
}

export function preferencePayload(
  serverId: string | null | undefined,
  itemId: string,
  seriesId: string | null | undefined,
  source: MediaVersion | undefined,
  audioIndex: number | undefined,
  subtitleIndex: number | undefined,
) {
  const audio = source?.audioStreams.find((stream) => stream.index === audioIndex);
  const subtitle = source?.subtitleStreams.find((stream) => stream.index === subtitleIndex);
  return {
    serverId,
    itemId,
    seriesId,
    mediaSourceId: source?.id,
    audioStreamIndex: audioIndex,
    audioLanguage: streamLanguage(audio),
    subtitleStreamIndex: subtitleIndex,
    subtitleLanguage: streamLanguage(subtitle),
  };
}
