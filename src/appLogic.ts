import type { HomePayload, ItemDetailPayload, LibraryFilters, LibraryItemType, LibraryPayload, LibrarySortBy, LibrarySortOrder, MediaItem } from "./types";

export function libraryKey(libraryId: string, itemType: LibraryItemType, sortBy: LibrarySortBy, sortOrder: LibrarySortOrder, filters: LibraryFilters = {}) {
  return `${libraryId}:${itemType}:${sortBy}:${sortOrder}:${JSON.stringify(filters)}`;
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
    ...(library?.items ?? []),
    ...(detail?.episodes ?? []),
    ...(detail?.children ?? []),
    ...(detail?.similar ?? []),
  ];
  return rows.find((item) => item.id === itemId);
}
