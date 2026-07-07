import { invoke } from "@tauri-apps/api/core";
import type {
  AppSettings,
  HomeMorePayload,
  HomePayload,
  ItemDetailPayload,
  ItemMorePayload,
  LibraryItemType,
  LibraryPayload,
  LibrarySortBy,
  LibrarySortOrder,
  LoginResult,
  PlaybackCommand,
  PlaybackState,
  PlayResult,
  SavedServer,
  SearchPayload,
  ServerForm,
} from "./types";

export const ipc = {
  searchItems: (query: string) => invoke<SearchPayload>("search_items", { input: { query } }),
  playbackState: (playSessionId: string) => invoke<PlaybackState>("playback_state", { input: { playSessionId } }),
  listServers: () => invoke<SavedServer[]>("list_servers"),
  loadSettings: () => invoke<AppSettings>("load_settings"),
  saveSettings: (input: AppSettings) => invoke<AppSettings>("save_settings", { input }),
  loadHome: () => invoke<HomePayload>("load_home"),
  loadHomeMore: () => invoke<HomeMorePayload>("load_home_more"),
  loadLibrary: (
    libraryId: string,
    startIndex: number,
    limit: number,
    itemType: LibraryItemType,
    sortBy: LibrarySortBy,
    sortOrder: LibrarySortOrder,
  ) => invoke<LibraryPayload>("load_library", { input: { libraryId, startIndex, limit, itemType, sortBy, sortOrder } }),
  loadItem: (itemId: string) => invoke<ItemDetailPayload>("load_item", { input: { itemId } }),
  loadItemMore: (itemId: string) => invoke<ItemMorePayload>("load_item_more", { input: { itemId } }),
  loadMediaSources: (itemId: string) => invoke<ItemDetailPayload["mediaSources"]>("load_media_sources", { input: { itemId } }),
  testServerLogin: (input: ServerForm) => invoke<LoginResult>("test_server_login", { input }),
  saveServer: (input: LoginResult) => invoke<SavedServer>("save_server", { input }),
  setActiveServer: (serverId: string) => invoke<SavedServer>("set_active_server", { input: { serverId } }),
  deleteServer: (serverId: string) => invoke<void>("delete_server", { input: { serverId } }),
  playItem: (itemId: string, mediaSourceId?: string, audioStreamIndex?: number, subtitleStreamIndex?: number, subtitleStreamPosition?: number) =>
    invoke<PlayResult>("play_item", { input: { itemId, mediaSourceId, audioStreamIndex, subtitleStreamIndex, subtitleStreamPosition } }),
  controlPlayback: (playSessionId: string, command: PlaybackCommand) =>
    invoke("control_playback", { input: { playSessionId, command } }),
  markFavorite: (itemId: string, value: boolean) => invoke<void>("mark_favorite", { input: { itemId, value } }),
  markPlayed: (itemId: string, value: boolean) => invoke<void>("mark_played", { input: { itemId, value } }),
  fetchServerName: (input: Pick<ServerForm, "url" | "serverType" | "useSystemProxy">) =>
    invoke<{ name: string }>("fetch_server_name", { input }),
};
