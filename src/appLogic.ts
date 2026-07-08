import type { HomePayload, ItemDetailPayload, LibraryItemType, LibraryPayload, LibrarySortBy, LibrarySortOrder, MediaItem } from "./types";

export function libraryKey(libraryId: string, itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder) {
  return `${libraryId}:${itemType}:${sortBy}:${sortOrder}`;
}

export function findKnownItem(
  itemId: string,
  home: HomePayload | null,
  library: LibraryPayload | null,
  detail: ItemDetailPayload | null,
) {
  const rows: MediaItem[] = [
    ...(home?.latest ?? []),
    ...(home?.recommendedMovies ?? []),
    ...(home?.recommendedShows ?? []),
    ...(home?.resumeItems ?? []),
    ...(home?.recentItems ?? []),
    ...(library?.items ?? []),
    ...(detail?.episodes ?? []),
    ...(detail?.children ?? []),
    ...(detail?.similar ?? []),
  ];
  return rows.find((item) => item.id === itemId);
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
