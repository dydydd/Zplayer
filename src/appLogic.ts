import type { HomePayload, ItemDetailPayload, LibraryItemType, LibraryPayload, LibrarySortBy, LibrarySortOrder, MediaItem, MediaVersion } from "./types";

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
    ...(library?.items ?? []),
    ...(detail?.episodes ?? []),
    ...(detail?.children ?? []),
    ...(detail?.similar ?? []),
  ];
  return rows.find((item) => item.id === itemId);
}

export function streamLanguage(stream?: { language?: string | null }) {
  return stream?.language?.trim().toLowerCase() || undefined;
}

export function playbackPreferenceKey(itemId: string, seriesId?: string | null) {
  return seriesId ? `series:${seriesId}` : `item:${itemId}`;
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
  itemId: string,
  seriesId: string | null | undefined,
  source: MediaVersion | undefined,
  audioIndex: number | undefined,
  subtitleIndex: number | undefined,
) {
  const audio = source?.audioStreams.find((stream) => stream.index === audioIndex);
  const subtitle = source?.subtitleStreams.find((stream) => stream.index === subtitleIndex);
  return {
    itemId,
    seriesId,
    mediaSourceId: source?.id,
    audioStreamIndex: audioIndex,
    audioLanguage: streamLanguage(audio),
    subtitleStreamIndex: subtitleIndex,
    subtitleLanguage: streamLanguage(subtitle),
  };
}
