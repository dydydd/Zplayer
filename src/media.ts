import i18n from "./i18n";
import type { MediaItem } from "./types";

export function itemMeta(item: MediaItem) {
  const chunks = [];
  if (item.year) {
    chunks.push(String(item.year));
  }
  if (item.childCount) {
    chunks.push(i18n.t("media.episodeCount", { count: item.childCount }));
  }
  if (item.itemType && !["folder", "movie", "series", "season"].includes(item.itemType.toLowerCase())) {
    chunks.push(itemTypeLabel(item.itemType));
  }
  return chunks.join(" / ");
}

export function episodeLabel(item: MediaItem) {
  if (item.seasonNumber && item.episodeNumber) {
    return `S${item.seasonNumber}E${item.episodeNumber}`;
  }
  return itemMeta(item);
}

export function runtimeLabel(runTimeTicks?: number | null) {
  if (!runTimeTicks) return "";
  const minutes = Math.round(runTimeTicks / 600_000_000);
  if (minutes < 60) return i18n.t("media.minute", { count: minutes });
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest
    ? i18n.t("media.hourMinute", { hours, minutes: rest })
    : i18n.t("media.hour", { count: hours });
}

export function bg(url?: string | null) {
  return url ? { backgroundImage: `linear-gradient(90deg, rgba(24,74,180,.82), rgba(0,0,0,.12)), url("${url}")` } : undefined;
}

function itemTypeLabel(itemType: string) {
  const normalized = itemType.toLowerCase();
  if (normalized === "episode") return i18n.t("library.episode");
  if (normalized === "video") return i18n.t("library.video");
  return itemType;
}
