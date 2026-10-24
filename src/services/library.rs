use super::AppPaths;
use crate::api::{
    self, LibraryBrowseFilters, LibraryBrowseOptions, LibraryItemsQuery, LibraryQueryFilters,
};
use crate::models::{
    EpisodesInput, EpisodesPayload, FavoritesPayload, HomePayload, ItemDetailPayload, ItemInput,
    LibraryBrowsePayload, LibraryInput, LibraryPayload, MediaItem, MediaVersion, PagedItems,
    PersonPayload, PlayedHistoryPayload, SearchInput, SearchPayload,
};
use crate::storage::{AppStore, server_summary};
use gpui_kit::{Image, ImageFormat};
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
pub(crate) struct LibraryService {
    store: AppStore,
}

/// 人物页「出演作品」请求的条目类型（电影 + 剧集）。
const FILMOGRAPHY_ITEM_TYPES: &[&str] = &["Movie", "Series"];

impl LibraryService {
    pub(crate) fn new(paths: &AppPaths) -> Result<Self, String> {
        Ok(Self {
            store: paths.store()?,
        })
    }

    pub(crate) fn home(&self) -> Result<HomePayload, String> {
        let server = self.store.active_server()?;
        let client = api::http_client_for_server(&server, Duration::from_secs(8))?;
        let recent_ids = self.store.recent_play_ids(&server.id).unwrap_or_default();
        let (
            libraries,
            latest,
            recommended_movies,
            recommended_shows,
            resume_items,
            favorite_items,
            recent_items,
        ) = thread::scope(|scope| {
            let libraries = scope.spawn(|| api::fetch_libraries(&client, &server));
            let latest = scope.spawn(|| api::get_latest_items(&client, &server, None, "24"));
            let recommended_movies =
                scope.spawn(|| api::get_suggested_items(&client, &server, "Movie"));
            let recommended_shows =
                scope.spawn(|| api::get_suggested_items(&client, &server, "Series"));
            let resume_items = scope.spawn(|| api::get_resume_items(&client, &server));
            let favorite_items =
                scope.spawn(|| api::get_favorite_items(&client, &server, "Movie,Series"));
            let recent_items = scope.spawn(|| load_recent_items(&client, &server, &recent_ids));

            let libraries = libraries
                .join()
                .map_err(|_| "Failed to load libraries.".to_string())??;
            let latest = latest.join().ok().and_then(Result::ok).unwrap_or_default();
            let recommended_movies = recommended_movies
                .join()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            let recommended_shows = recommended_shows
                .join()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            let resume_items = resume_items
                .join()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            let favorite_items = favorite_items
                .join()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default();
            let recent_items = recent_items.join().ok().unwrap_or_default();

            Ok::<_, String>((
                libraries,
                latest,
                recommended_movies,
                recommended_shows,
                resume_items,
                favorite_items,
                recent_items,
            ))
        })?;

        Ok(HomePayload {
            server: server_summary(&server),
            libraries,
            library_latest: Vec::new(),
            latest,
            recommended_movies,
            recommended_shows,
            resume_items,
            favorite_items,
            recent_items,
        })
    }

    pub(crate) fn library(&self, input: LibraryInput) -> Result<LibraryPayload, String> {
        let server = self.server_for(input.server_id.as_deref())?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        let start_index = input.start_index.unwrap_or_default();
        let limit = input.limit.unwrap_or(60).clamp(1, 200);
        let sort_by = normalize_sort_by(input.sort_by.as_deref());
        let sort_order = normalize_sort_order(input.sort_order.as_deref());
        let item_type = normalize_item_type(input.item_type.as_deref());
        let filters = input
            .filters
            .map(|filters| LibraryQueryFilters {
                played: filters.played.as_deref().and_then(normalize_played),
                favorite: filters.favorite,
                genre: filters.genre,
                person_id: filters.person_id,
                collection_id: filters.collection_id,
            })
            .unwrap_or_default();

        let (items, total_count) = api::get_library_items(
            &client,
            &server,
            LibraryItemsQuery {
                library_id: &input.library_id,
                start_index,
                limit,
                item_type,
                sort_by,
                sort_order,
                filters: &filters,
            },
        )?;
        let library = api::fetch_libraries(&client, &server)?
            .into_iter()
            .find(|library| library.id == input.library_id)
            .ok_or_else(|| "Library not found.".to_string())?;
        let has_more = start_index + items.len() < total_count;
        Ok(LibraryPayload {
            library,
            items,
            total_count,
            start_index,
            limit,
            has_more,
        })
    }

    pub(crate) fn detail(&self, input: ItemInput) -> Result<ItemDetailPayload, String> {
        let server = self.server_for(input.server_id.as_deref())?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        let raw = api::get_item_raw(&client, &server, &input.item_id)?;
        let people = api::people(&server, &raw);
        let art = api::art_urls(&server, &raw);
        let item = api::map_item(&server, raw);
        let (children, seasons, episodes, episode_total_count, media_sources, similar) =
            thread::scope(|scope| {
                let children = scope.spawn(|| api::get_item_children(&client, &server, &item.id));
                let media_sources =
                    scope.spawn(|| api::get_media_versions(&client, &server, &item.id));
                let similar = scope.spawn(|| api::get_similar_items(&client, &server, &item.id));
                let children = children
                    .join()
                    .map_err(|_| "Detail worker failed.".to_string())??;
                let media_sources = media_sources
                    .join()
                    .map_err(|_| "Media source worker failed.".to_string())??;
                let similar = similar
                    .join()
                    .map_err(|_| "Similar item worker failed.".to_string())??;
                let mut seasons = Vec::new();
                let mut episodes = Vec::new();
                let mut episode_total_count = None;
                if item.item_type == "Series" {
                    seasons = api::get_show_seasons(&client, &server, &item.id)?;
                    if let Some(season) = seasons.first() {
                        let (page, total) = api::get_show_episodes_page(
                            &client,
                            &server,
                            &item.id,
                            Some(&season.id),
                            0,
                            50,
                        )?;
                        episodes = page;
                        episode_total_count = Some(total);
                    }
                }
                Ok::<_, String>((
                    children,
                    seasons,
                    episodes,
                    episode_total_count,
                    media_sources,
                    similar,
                ))
            })?;

        Ok(ItemDetailPayload {
            item,
            children,
            seasons,
            episodes,
            episode_total_count,
            media_sources,
            people,
            art,
            similar,
        })
    }

    pub(crate) fn media_sources(&self, input: ItemInput) -> Result<Vec<MediaVersion>, String> {
        let server = self.server_for(input.server_id.as_deref())?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        api::get_media_versions(&client, &server, &input.item_id)
    }

    pub(crate) fn episodes(&self, input: EpisodesInput) -> Result<EpisodesPayload, String> {
        let server = self.server_for(input.server_id.as_deref())?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        let start_index = input.start_index.unwrap_or_default();
        let limit = input.limit.unwrap_or(50).clamp(1, 200);
        let (items, total_count) = api::get_show_episodes_page(
            &client,
            &server,
            &input.series_id,
            input.season_id.as_deref(),
            start_index,
            limit,
        )?;
        let has_more = start_index + items.len() < total_count;
        Ok(EpisodesPayload {
            series_id: input.series_id,
            season_id: input.season_id,
            items,
            total_count,
            start_index,
            limit,
            has_more,
        })
    }

    pub(crate) fn search(&self, input: SearchInput) -> Result<SearchPayload, String> {
        let query = input.query.trim();
        if query.is_empty() {
            return Ok(SearchPayload { items: Vec::new() });
        }
        let servers = self.store.servers()?;
        if servers.is_empty() {
            return Ok(SearchPayload { items: Vec::new() });
        }
        let (items, errors) = thread::scope(|scope| {
            servers
                .iter()
                .map(|server| {
                    scope.spawn(|| {
                        let client = api::http_client_for_server(server, Duration::from_secs(8))?;
                        api::search_items(&client, server, query)
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .fold(
                    (Vec::new(), Vec::new()),
                    |(mut items, mut errors), handle| {
                        match handle.join() {
                            Ok(Ok(mut server_items)) => items.append(&mut server_items),
                            Ok(Err(err)) => errors.push(err),
                            Err(_) => errors.push("Search worker failed.".to_string()),
                        }
                        (items, errors)
                    },
                )
        });
        if items.is_empty() && !errors.is_empty() {
            return Err(format!(
                "Search failed on all servers: {}",
                errors.join(" | ")
            ));
        }
        Ok(SearchPayload { items })
    }

    pub(crate) fn person_detail(&self, person_id: &str) -> Result<MediaItem, String> {
        let server = self.store.active_server()?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        api::get_person_item(&client, &server, &server.user_id, person_id)
    }

    pub(crate) fn person_filmography(
        &self,
        person_id: &str,
        include_item_types: &[&str],
    ) -> Result<Vec<MediaItem>, String> {
        let server = self.store.active_server()?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        api::get_person_filmography(
            &client,
            &server,
            &server.user_id,
            person_id,
            include_item_types,
        )
    }

    pub(crate) fn library_browse(
        &self,
        library_id: &str,
        opts: LibraryBrowseOptions<'_>,
    ) -> Result<PagedItems, String> {
        let server = self.store.active_server()?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        api::get_library_items_paged(&client, &server, &server.user_id, library_id, opts)
    }

    pub(crate) fn played_history(
        &self,
        limit: u64,
        start_index: u64,
    ) -> Result<PagedItems, String> {
        let server = self.store.active_server()?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        api::get_played_history(&client, &server, &server.user_id, limit, start_index)
    }

    /// 人物页数据：并行拉取人物条目与出演作品。
    pub(crate) fn person_page(&self, person_id: &str) -> Result<PersonPayload, String> {
        let (person, filmography) = thread::scope(|scope| {
            let person = scope.spawn(|| self.person_detail(person_id));
            let filmography =
                scope.spawn(|| self.person_filmography(person_id, FILMOGRAPHY_ITEM_TYPES));
            let person = person
                .join()
                .map_err(|_| "Person worker failed.".to_string())??;
            // 出演作品是可选的：单条失败（例如服务器不支持 Persons 端点）时
            // 仍展示人物资料，而不是整页报错。
            let filmography = filmography.join().ok().and_then(Result::ok).unwrap_or_default();
            Ok::<_, String>((person, filmography))
        })?;
        Ok(PersonPayload {
            person,
            filmography,
        })
    }

    /// 媒体库浏览页的一页数据（分页 + 排序）。
    pub(crate) fn library_page(
        &self,
        library_id: &str,
        library_name: &str,
        start_index: u64,
        limit: u64,
        sort_by: &str,
    ) -> Result<LibraryBrowsePayload, String> {
        // 库浏览页不做题材/年份筛选：使用空切片即可。
        let genres: [&str; 0] = [];
        let include_item_types: [&str; 0] = [];
        let paged = self.library_browse(
            library_id,
            LibraryBrowseOptions {
                start_index,
                limit,
                sort_by,
                sort_order: "Descending",
                recursive: true,
                filters: LibraryBrowseFilters {
                    genres: &genres,
                    year: None,
                },
                include_item_types: &include_item_types,
            },
        )?;
        let loaded = start_index + paged.items.len() as u64;
        let has_more = loaded < paged.total_record_count;
        Ok(LibraryBrowsePayload {
            library_id: library_id.to_string(),
            library_name: library_name.to_string(),
            items: paged.items,
            total_count: paged.total_record_count,
            start_index,
            has_more,
        })
    }

    /// 播放历史页的一页数据（`SortBy=DatePlayed` 倒序）。
    pub(crate) fn played_history_page(
        &self,
        start_index: u64,
        limit: u64,
    ) -> Result<PlayedHistoryPayload, String> {
        let paged = self.played_history(limit, start_index)?;
        let loaded = start_index + paged.items.len() as u64;
        let has_more = loaded < paged.total_record_count;
        Ok(PlayedHistoryPayload {
            items: paged.items,
            total_count: paged.total_record_count,
            start_index,
            has_more,
        })
    }

    /// 收藏列表（按 `IncludeItemTypes` 分类过滤）。
    pub(crate) fn favorites(&self, include_item_types: &str) -> Result<FavoritesPayload, String> {
        let server = self.store.active_server()?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        let items = api::get_favorite_items(&client, &server, include_item_types)?;
        Ok(FavoritesPayload { items })
    }

    /// 设置/取消收藏（详情页「收藏」按钮）。
    pub(crate) fn set_favorite(
        &self,
        server_id: &str,
        item_id: &str,
        value: bool,
    ) -> Result<(), String> {
        let server = self.server_for(Some(server_id))?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        api::set_favorite(&client, &server, item_id, value)
    }

    /// 设置/取消已看（详情页「已看」按钮）。
    pub(crate) fn set_played(
        &self,
        server_id: &str,
        item_id: &str,
        value: bool,
    ) -> Result<(), String> {
        let server = self.server_for(Some(server_id))?;
        let client = api::http_client_for_server(&server, Duration::from_secs(12))?;
        api::set_played(&client, &server, item_id, value)
    }

    pub(crate) fn image(&self, server_id: &str, url: &str) -> Result<Image, String> {
        let server = self.store.server_by_id(server_id)?;
        let client = api::http_client_for_server(&server, Duration::from_secs(10))?;
        let response = client
            .get(url)
            .header("X-Emby-Token", &server.access_token)
            .send()
            .map_err(|err| format!("Image request failed: {err}"))?;
        let bytes = api::read_image(response, "media image")?;
        Ok(Image::from_bytes(image_format(&bytes), bytes))
    }

    fn server_for(&self, server_id: Option<&str>) -> Result<crate::models::SavedServer, String> {
        match server_id.filter(|value| !value.trim().is_empty()) {
            Some(server_id) => self.store.server_by_id(server_id),
            None => self.store.active_server(),
        }
    }
}

fn load_recent_items(
    client: &reqwest::blocking::Client,
    server: &crate::models::SavedServer,
    ids: &[String],
) -> Vec<MediaItem> {
    ids.iter()
        .filter_map(|id| {
            api::get_item_raw(client, server, id)
                .ok()
                .map(|raw| api::map_item(server, raw))
        })
        .collect()
}

fn normalize_sort_by(value: Option<&str>) -> &'static str {
    match value {
        Some("PremiereDate") => "PremiereDate",
        Some("CommunityRating") => "CommunityRating",
        Some("DatePlayed") => "DatePlayed",
        _ => "SortName",
    }
}

fn normalize_sort_order(value: Option<&str>) -> &'static str {
    match value {
        Some("Ascending") => "Ascending",
        _ => "Descending",
    }
}

fn normalize_item_type(value: Option<&str>) -> Option<&'static str> {
    match value {
        Some("Movie") => Some("Movie"),
        Some("Series") => Some("Series"),
        Some("Episode") => Some("Episode"),
        Some("Video") => Some("Video"),
        _ => None,
    }
}

fn normalize_played(value: &str) -> Option<bool> {
    match value {
        "played" | "true" => Some(true),
        "unplayed" | "false" => Some(false),
        _ => None,
    }
}

#[allow(dead_code)]
fn media_item_id(item: &MediaItem) -> &str {
    &item.id
}

fn image_format(bytes: &[u8]) -> ImageFormat {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        ImageFormat::Png
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        ImageFormat::Jpeg
    } else if bytes.starts_with(b"GIF8") {
        ImageFormat::Gif
    } else if bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
        ImageFormat::Webp
    } else if bytes.starts_with(b"BM") {
        ImageFormat::Bmp
    } else {
        ImageFormat::Png
    }
}
