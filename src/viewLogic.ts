import type { ItemDetailPayload } from "./types";
import i18n from "./i18n";

export function formatTime(seconds: number) {
  const value = Math.max(0, Math.floor(seconds || 0));
  const hours = Math.floor(value / 3600);
  const minutes = Math.floor((value % 3600) / 60);
  const rest = value % 60;
  return hours
    ? `${hours}:${minutes.toString().padStart(2, "0")}:${rest.toString().padStart(2, "0")}`
    : `${minutes}:${rest.toString().padStart(2, "0")}`;
}

export function mediaVersionFacts(source?: ItemDetailPayload["mediaSources"][number]) {
  if (!source) return [];
  return [
    source.container?.toUpperCase(),
    source.resolution,
    source.videoCodec,
    source.frameRate ? i18n.t("media.fps", { value: source.frameRate.toFixed(2) }) : "",
    source.audioCodec,
    source.audioChannels ? i18n.t("detail.channels", { count: source.audioChannels }) : "",
    source.bitrate ? i18n.t("media.bitrate", { value: Math.round(source.bitrate / 1000000) }) : "",
    source.size ? formatBytes(source.size) : "",
    source.subtitleCount ? i18n.t("media.subtitles", { count: source.subtitleCount }) : "",
    source.protocol,
  ].filter(Boolean) as string[];
}

export function rotateDaily<T>(items: T[], seed: string) {
  if (items.length < 2) return items;
  const today = Math.floor(Date.now() / 86400000);
  const seedValue = [...seed].reduce((total, char) => total + char.charCodeAt(0), today);
  const start = seedValue % items.length;
  return [...items.slice(start), ...items.slice(0, start)];
}

function formatBytes(value: number) {
  if (value >= 1024 ** 3) return `${(value / 1024 ** 3).toFixed(1)} GB`;
  if (value >= 1024 ** 2) return `${Math.round(value / 1024 ** 2)} MB`;
  return `${value} B`;
}
