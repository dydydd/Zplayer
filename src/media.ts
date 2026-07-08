import type { MediaItem } from "./types";

export function itemMeta(item: MediaItem) {
  const chunks = [];
  if (item.year) {
    chunks.push(String(item.year));
  }
  if (item.childCount) {
    chunks.push(`${item.childCount} 集`);
  }
  if (item.itemType && !["folder", "movie", "series", "season"].includes(item.itemType.toLowerCase())) {
    chunks.push(item.itemType);
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
  if (minutes < 60) return `${minutes} 分钟`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest ? `${hours} 小时 ${rest} 分钟` : `${hours} 小时`;
}

export function bg(url?: string | null) {
  return url ? { backgroundImage: `linear-gradient(90deg, rgba(24,74,180,.82), rgba(0,0,0,.12)), url("${url}")` } : undefined;
}
