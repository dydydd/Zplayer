import assert from "node:assert/strict";
import test from "node:test";
import { collectionLibraryView, episodePlaybackContext, findKnownItem, preferredStreamIndex, relativeEpisodeId } from "./appLogic.ts";
import type { HomePayload, MediaItem } from "./types.ts";

test("builds collection-backed library view", () => {
  assert.deepEqual(collectionLibraryView("box-1", "Favorites box"), {
    name: "library",
    id: "",
    title: "Favorites box",
    filters: { collectionId: "box-1" },
  });
});

test("selects relative episode ids inside known context", () => {
  const context = episodePlaybackContext("episode-2", ["episode-1", "episode-2", "episode-3"]);
  assert.equal(relativeEpisodeId(context, -1), "episode-1");
  assert.equal(relativeEpisodeId(context, 1), "episode-3");
});

test("prefers exact stream index before language fallback", () => {
  const streams = [
    { index: 1, language: "jpn" },
    { index: 2, language: "eng" },
  ];
  assert.equal(preferredStreamIndex(streams, 2, "jpn"), 2);
  assert.equal(preferredStreamIndex(streams, 9, "eng"), 2);
});

test("finds items from the favorites home shelf", () => {
  const item = mediaItem({ id: "fav-1", name: "Favorite movie" });
  const home = {
    latest: [],
    recommendedMovies: [],
    recommendedShows: [],
    resumeItems: [],
    favoriteItems: [item],
    recentItems: [],
  } as HomePayload;

  assert.equal(findKnownItem("fav-1", home, null, null), item);
});

function mediaItem(item: Pick<MediaItem, "id" | "name">): MediaItem {
  return {
    ...item,
    itemType: "Movie",
    genres: [],
    studios: [],
    tags: [],
    played: false,
    favorite: false,
  };
}
