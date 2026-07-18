import { useEffect, useState } from "react";

export type ServerIconEntry = {
  name: string;
  url: string;
};

type ServerIconCache = {
  catalogKey: string;
  savedAt: number;
  icons: ServerIconEntry[];
};

export const DEFAULT_SERVER_ICON_CATALOG_URLS = [
  "https://emby-icon.vercel.app/TFEL-Emby.json",
] as const;

const SERVER_ICON_CACHE_KEY = "zplayer:server-icon-catalog:v1";
const SERVER_ICON_CACHE_MAX_AGE_MS = 1000 * 60 * 60 * 24 * 7;
const GENERIC_ICON_NAMES = new Set(["emby", "jellyfin", "media", "server"]);

let memoryCatalogKey = "";
let memoryIcons: ServerIconEntry[] | null = null;
let loadingCatalogKey = "";
let loadingIcons: Promise<ServerIconEntry[]> | null = null;

export function useServerIconEntries(catalogUrls: readonly string[] = DEFAULT_SERVER_ICON_CATALOG_URLS) {
  const catalogKey = serverIconCatalogKey(catalogUrls);
  const [icons, setIcons] = useState<ServerIconEntry[]>(() => {
    if (memoryCatalogKey === catalogKey && memoryIcons) return memoryIcons;
    return readServerIconCache(catalogKey, false) ?? [];
  });

  useEffect(() => {
    let cancelled = false;
    void loadServerIconEntries(catalogKey ? catalogKey.split("\n") : []).then((nextIcons) => {
      if (!cancelled) setIcons(nextIcons);
    });
    return () => {
      cancelled = true;
    };
  }, [catalogKey]);

  return icons;
}

export function resolveServerIconUrl(serverName: string, icons: ServerIconEntry[]) {
  if (!serverName.trim() || !icons.length) return null;
  const candidates = serverIconNameCandidates(serverName);
  const normalizedCandidates = new Set(candidates.map(normalizeServerIconName));
  const compactCandidates = new Set(candidates.map(compactServerIconName));

  for (const icon of icons) {
    if (normalizedCandidates.has(normalizeServerIconName(icon.name))) return icon.url;
  }

  for (const icon of icons) {
    if (compactCandidates.has(compactServerIconName(icon.name))) return icon.url;
  }

  const normalizedServerName = normalizeServerIconName(serverName);
  return icons
    .map((icon) => ({ ...icon, normalizedName: normalizeServerIconName(icon.name) }))
    .filter((icon) => icon.normalizedName.length >= 3 && !GENERIC_ICON_NAMES.has(icon.normalizedName))
    .sort((left, right) => right.normalizedName.length - left.normalizedName.length)
    .find((icon) => normalizedServerName.includes(icon.normalizedName))?.url ?? null;
}

export function serverIconCatalogUrls(value: string) {
  return value
    .split(/[\n,，]+/)
    .map((url) => url.trim())
    .filter(Boolean);
}

export function parseServerIconCatalog(value: unknown, catalogUrl: string) {
  if (!isRecord(value) || !Array.isArray(value.icons)) return [];
  return value.icons.flatMap((entry): ServerIconEntry[] => {
    if (!isRecord(entry) || typeof entry.name !== "string" || typeof entry.url !== "string") {
      return [];
    }
    const name = entry.name.trim();
    const url = resolveIconUrl(entry.url, catalogUrl);
    return name && url ? [{ name, url }] : [];
  });
}

export function normalizeServerIconName(value: string) {
  return value
    .normalize("NFKC")
    .trim()
    .toLowerCase()
    .replace(/[_-]+/g, " ")
    .replace(/\s+/g, " ");
}

function compactServerIconName(value: string) {
  return normalizeServerIconName(value).replace(/\s+/g, "");
}

function serverIconNameCandidates(serverName: string) {
  const candidates = new Set<string>();
  const normalized = normalizeServerIconName(serverName);
  const add = (value: string) => {
    const candidate = normalizeServerIconName(value);
    if (candidate && !GENERIC_ICON_NAMES.has(candidate)) candidates.add(candidate);
  };

  add(normalized);
  add(stripServerSuffix(normalized));

  normalized
    .split(/[|/\\:：,，·•]+/)
    .map(stripServerSuffix)
    .forEach(add);

  return [...candidates];
}

function stripServerSuffix(value: string) {
  return value
    .replace(/\s*(?:media\s+server|emby|jellyfin|server)$/i, "")
    .replace(/(?:媒体服务器|影音库|影视库|影院|影视)$/i, "")
    .trim();
}

function serverIconCatalogKey(catalogUrls: readonly string[]) {
  return [...catalogUrls].join("\n");
}

export async function loadServerIconEntries(catalogUrls: readonly string[]) {
  const catalogKey = serverIconCatalogKey(catalogUrls);
  if (memoryCatalogKey === catalogKey && memoryIcons) return memoryIcons;

  const cached = readServerIconCache(catalogKey, false);
  if (cached) {
    memoryCatalogKey = catalogKey;
    memoryIcons = cached;
    return cached;
  }

  if (loadingIcons && loadingCatalogKey === catalogKey) return loadingIcons;
  loadingCatalogKey = catalogKey;
  loadingIcons = fetchServerIconEntries(catalogUrls, catalogKey)
    .finally(() => {
      if (loadingCatalogKey === catalogKey) {
        loadingCatalogKey = "";
        loadingIcons = null;
      }
    });
  return loadingIcons;
}

async function fetchServerIconEntries(catalogUrls: readonly string[], catalogKey: string) {
  if (typeof fetch !== "function") {
    return readServerIconCache(catalogKey, true) ?? [];
  }

  try {
    const iconGroups = await Promise.all(catalogUrls.map(async (catalogUrl) => {
      const response = await fetch(catalogUrl, { cache: "force-cache" });
      if (!response.ok) throw new Error(`Failed to load server icon catalog: ${response.status}`);
      return parseServerIconCatalog(await response.json(), catalogUrl);
    }));
    const icons = dedupeServerIcons(iconGroups.flat());
    memoryCatalogKey = catalogKey;
    memoryIcons = icons;
    writeServerIconCache(catalogKey, icons);
    return icons;
  } catch {
    const stale = readServerIconCache(catalogKey, true);
    if (stale) {
      memoryCatalogKey = catalogKey;
      memoryIcons = stale;
      return stale;
    }
    return [];
  }
}

function dedupeServerIcons(icons: ServerIconEntry[]) {
  const seen = new Set<string>();
  return icons.filter((icon) => {
    const key = `${normalizeServerIconName(icon.name)}\n${icon.url}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

function readServerIconCache(catalogKey: string, allowStale: boolean) {
  try {
    if (typeof localStorage === "undefined") return null;
    const raw = localStorage.getItem(SERVER_ICON_CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as ServerIconCache;
    if (
      parsed.catalogKey !== catalogKey
      || typeof parsed.savedAt !== "number"
      || !Array.isArray(parsed.icons)
      || (!allowStale && Date.now() - parsed.savedAt > SERVER_ICON_CACHE_MAX_AGE_MS)
    ) {
      return null;
    }
    return dedupeServerIcons(parsed.icons.filter(isServerIconEntry));
  } catch {
    return null;
  }
}

function writeServerIconCache(catalogKey: string, icons: ServerIconEntry[]) {
  try {
    if (typeof localStorage === "undefined") return;
    localStorage.setItem(SERVER_ICON_CACHE_KEY, JSON.stringify({
      catalogKey,
      savedAt: Date.now(),
      icons,
    }));
  } catch {
    // Missing storage should not block normal server rendering.
  }
}

function resolveIconUrl(rawUrl: string, catalogUrl: string) {
  try {
    const url = new URL(rawUrl.trim(), catalogUrl);
    return url.protocol === "http:" || url.protocol === "https:" ? url.toString() : "";
  } catch {
    return "";
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isServerIconEntry(value: unknown): value is ServerIconEntry {
  return isRecord(value) && typeof value.name === "string" && typeof value.url === "string";
}
