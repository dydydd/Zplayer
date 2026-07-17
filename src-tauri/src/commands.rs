use crate::api;
use crate::models::{
    normalize_settings, AppSettings, FetchServerNameInput, FetchServerNameResult, FilePathInput,
    HomeMorePayload, HomePayload, ItemDetailPayload, ItemInput, ItemMorePayload,
    LibraryFiltersInput, LibraryInput, LibraryLatestPayload, LibraryPayload, LoginInput,
    LoginResult, MarkInput, MediaItem, MediaLibrary, MediaVersion, PlayResult,
    PlaybackControlInput, PlaybackPreference, PlaybackStateInput, PlaybackStateResult,
    ReportPlaybackProgressInput, ReportPlaybackStartInput, ReportPlaybackStoppedInput,
    SavePlaybackPreferenceInput, SaveServerInput, SaveSettingsInput, SavedServer,
    SavedServerSummary, SearchInput, SearchPayload, ServerIdInput, ServerImportResult,
};
use crate::mpv;
use crate::platform_window::{self, LinuxWindowDiagnostics};
use crate::store;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

const HOME_FIRST_LOAD_TIMEOUT: Duration = Duration::from_secs(8);
const HOME_MORE_TIMEOUT: Duration = Duration::from_secs(12);

fn server_for_input(app: &AppHandle, server_id: Option<&str>) -> Result<SavedServer, String> {
    match server_id.filter(|value| !value.trim().is_empty()) {
        Some(server_id) => store::server_by_id(app, server_id),
        None => store::active_server(app),
    }
}

#[tauri::command]
pub(crate) async fn test_server_login(input: LoginInput) -> Result<LoginResult, String> {
    tauri::async_runtime::spawn_blocking(move || test_server_login_sync(input))
        .await
        .map_err(|err| err.to_string())?
}

fn test_server_login_sync(input: LoginInput) -> Result<LoginResult, String> {
    let url = api::normalize_server_url(&input.url)?;
    let client = api::http_client(input.use_system_proxy)?;
    let auth = api::authenticate_by_name(
        &client,
        &url,
        &input.server_type,
        input.username.trim(),
        &input.password,
    )?;
    let public_name = if input.name.trim().is_empty() {
        let public_info = api::fetch_server_public_info(&client, &url).unwrap_or_default();
        public_info
            .server_name
            .or(public_info.product_name)
            .unwrap_or_default()
    } else {
        String::new()
    };
    let username = auth
        .user
        .name
        .unwrap_or_else(|| input.username.trim().to_string());
    Ok(LoginResult {
        id: api::server_id(&url, &auth.user.id),
        name: api::clean_name(&input.name, &public_name, &url),
        url,
        username,
        user_id: auth.user.id,
        access_token: auth.access_token,
        use_system_proxy: input.use_system_proxy,
    })
}

#[tauri::command]
pub(crate) async fn save_server(
    app: AppHandle,
    input: SaveServerInput,
) -> Result<SavedServerSummary, String> {
    tauri::async_runtime::spawn_blocking(move || save_server_sync(app, input))
        .await
        .map_err(|err| err.to_string())?
}

fn save_server_sync(app: AppHandle, input: SaveServerInput) -> Result<SavedServerSummary, String> {
    let saved = SavedServer {
        id: input.id,
        name: input.name,
        url: api::normalize_server_url(&input.url)?,
        username: input.username,
        user_id: input.user_id,
        access_token: input.access_token,
        active: true,
        saved_at: store::unix_now(),
        use_system_proxy: input.use_system_proxy,
    };
    store::save_server(&app, saved)
}

#[tauri::command]
pub(crate) async fn list_servers(app: AppHandle) -> Result<Vec<SavedServerSummary>, String> {
    tauri::async_runtime::spawn_blocking(move || list_servers_sync(app))
        .await
        .map_err(|err| err.to_string())?
}

fn list_servers_sync(app: AppHandle) -> Result<Vec<SavedServerSummary>, String> {
    let mut summaries = store::list_servers(&app)?;
    let store_servers = store::servers(&app)?;
    let counts = thread::scope(|scope| {
        store_servers
            .iter()
            .map(|server| {
                scope.spawn(|| {
                    let client = api::http_client_with_timeout(
                        server.use_system_proxy,
                        Duration::from_secs(3),
                    )
                    .ok()?;
                    let counts = api::item_counts(&client, server).ok()?;
                    Some((server.id.as_str(), counts))
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|handle| handle.join().ok().flatten())
            .collect::<Vec<_>>()
    });
    for summary in &mut summaries {
        if let Some((_, counts)) = counts.iter().find(|(id, _)| *id == summary.id) {
            summary.movie_count = counts.movie_count;
            summary.series_count = counts.series_count;
        }
    }
    Ok(summaries)
}

#[tauri::command]
pub(crate) async fn export_servers(app: AppHandle, input: FilePathInput) -> Result<usize, String> {
    tauri::async_runtime::spawn_blocking(move || {
        store::export_servers(&app, PathBuf::from(input.path))
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) async fn import_servers(
    app: AppHandle,
    input: FilePathInput,
) -> Result<ServerImportResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        store::import_servers(&app, PathBuf::from(input.path))
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) fn load_settings(app: AppHandle) -> Result<AppSettings, String> {
    store::settings(&app)
}

#[tauri::command]
pub(crate) fn linux_window_diagnostics() -> LinuxWindowDiagnostics {
    platform_window::diagnostics()
}

#[tauri::command]
pub(crate) fn save_settings(
    app: AppHandle,
    input: SaveSettingsInput,
) -> Result<AppSettings, String> {
    store::save_settings(&app, normalize_settings(input))
}

#[tauri::command]
pub(crate) async fn set_active_server(
    app: AppHandle,
    input: ServerIdInput,
) -> Result<SavedServerSummary, String> {
    tauri::async_runtime::spawn_blocking(move || store::set_active_server(&app, input.server_id))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) async fn delete_server(app: AppHandle, input: ServerIdInput) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || store::delete_server(&app, input.server_id))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) async fn load_home(app: AppHandle) -> Result<HomePayload, String> {
    tauri::async_runtime::spawn_blocking(move || load_home_sync(app))
        .await
        .map_err(|err| err.to_string())?
}

fn load_home_sync(app: AppHandle) -> Result<HomePayload, String> {
    let server = store::active_server(&app)?;
    let client = api::http_client_with_timeout(server.use_system_proxy, HOME_FIRST_LOAD_TIMEOUT)?;
    let (libraries, latest) = thread::scope(|scope| {
        let libraries = scope.spawn(|| api::fetch_libraries(&client, &server));
        let latest = scope.spawn(|| api::get_latest_items(&client, &server, None, "24"));

        let libraries = libraries
            .join()
            .map_err(|_| "Failed to load libraries.".to_string())??;
        let latest = latest.join().ok().and_then(Result::ok).unwrap_or_default();
        Ok::<_, String>((libraries, latest))
    })?;

    Ok(HomePayload {
        server: store::server_summary(&server),
        libraries,
        library_latest: Vec::new(),
        latest,
        recommended_movies: Vec::new(),
        recommended_shows: Vec::new(),
        resume_items: Vec::new(),
        favorite_items: Vec::new(),
        recent_items: Vec::new(),
    })
}

#[tauri::command]
pub(crate) async fn load_home_more(app: AppHandle) -> Result<HomeMorePayload, String> {
    tauri::async_runtime::spawn_blocking(move || load_home_more_sync(app))
        .await
        .map_err(|err| err.to_string())?
}

fn load_home_more_sync(app: AppHandle) -> Result<HomeMorePayload, String> {
    let server = store::active_server(&app)?;
    let client = api::http_client_with_timeout(server.use_system_proxy, HOME_MORE_TIMEOUT)?;
    let libraries = api::fetch_libraries(&client, &server)?;
    let app_for_recent = app.clone();
    let (
        library_latest,
        recommended_movies,
        recommended_shows,
        resume_items,
        favorite_items,
        recent_items,
    ) = thread::scope(|scope| {
        let library_latest = scope.spawn(|| load_library_latest(&client, &server, &libraries));
        let recommended_movies = scope.spawn(|| load_recommended_movies(&client, &server));
        let recommended_shows =
            scope.spawn(|| api::get_suggested_items(&client, &server, "Series"));
        let resume_items = scope.spawn(|| api::get_resume_items(&client, &server));
        let favorite_items = scope.spawn(|| load_favorite_items(&client, &server));
        let recent_items = scope.spawn(|| load_recent_items(&app_for_recent, &client, &server));

        (
            library_latest.join().unwrap_or_default(),
            recommended_movies
                .join()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default(),
            recommended_shows
                .join()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default(),
            resume_items
                .join()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default(),
            favorite_items
                .join()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default(),
            recent_items.join().unwrap_or_default(),
        )
    });

    Ok(HomeMorePayload {
        server_id: server.id,
        library_latest,
        recommended_movies,
        recommended_shows,
        resume_items,
        favorite_items,
        recent_items,
    })
}

fn load_library_latest(
    client: &reqwest::blocking::Client,
    server: &SavedServer,
    libraries: &[MediaLibrary],
) -> Vec<LibraryLatestPayload> {
    thread::scope(|scope| {
        libraries
            .iter()
            .map(|library| {
                scope.spawn(|| LibraryLatestPayload {
                    library: library.clone(),
                    items: api::get_latest_items(client, server, Some(&library.id), "18")
                        .unwrap_or_default()
                        .into_iter()
                        .take(12)
                        .collect(),
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|handle| handle.join().ok())
            .filter(|row| !row.items.is_empty())
            .collect()
    })
}

fn load_recommended_movies(
    client: &reqwest::blocking::Client,
    server: &SavedServer,
) -> Result<Vec<crate::models::MediaItem>, String> {
    api::get_recommendations(
        client,
        server,
        &[
            ("UserId", server.user_id.clone()),
            ("userId", server.user_id.clone()),
            ("Limit", "12".to_string()),
            ("limit", "12".to_string()),
            ("CategoryLimit", "12".to_string()),
            ("categoryLimit", "12".to_string()),
            ("ItemLimit", "12".to_string()),
            ("itemLimit", "12".to_string()),
            ("Fields", api::item_fields()),
            ("fields", api::item_fields()),
            ("EnableImageTypes", api::image_types()),
            ("enableImageTypes", api::image_types()),
        ],
    )
}

fn load_favorite_items(
    client: &reqwest::blocking::Client,
    server: &SavedServer,
) -> Result<Vec<crate::models::MediaItem>, String> {
    api::get_library_items(
        client,
        server,
        api::LibraryItemsQuery {
            library_id: "",
            start_index: 0,
            limit: 16,
            item_type: None,
            sort_by: "DateCreated",
            sort_order: "Descending",
            filters: &api::LibraryQueryFilters {
                favorite: Some(true),
                ..Default::default()
            },
        },
    )
    .map(|(items, _)| items)
}

fn load_recent_items(
    app: &AppHandle,
    client: &reqwest::blocking::Client,
    server: &SavedServer,
) -> Vec<crate::models::MediaItem> {
    store::recent_play_ids(app, &server.id)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|id| api::get_item_raw(client, server, &id).ok())
        .map(|item| api::map_item(server, item))
        .take(16)
        .collect()
}

fn load_aggregated_media_versions(
    app: &AppHandle,
    base_server: &SavedServer,
    item: &MediaItem,
) -> Vec<MediaVersion> {
    let mut versions = api::http_client(base_server.use_system_proxy)
        .and_then(|client| api::get_media_versions(&client, base_server, &item.id))
        .unwrap_or_default();
    let queries = version_search_queries(item);
    if queries.is_empty() {
        return versions;
    }

    let servers = store::servers(app).unwrap_or_default();
    let mut remote_versions = thread::scope(|scope| {
        servers
            .iter()
            .filter(|server| server.id != base_server.id)
            .map(|server| {
                scope.spawn(|| matching_item_versions(server, item, &queries).unwrap_or_default())
            })
            .collect::<Vec<_>>()
            .into_iter()
            .flat_map(|handle| handle.join().unwrap_or_default())
            .collect::<Vec<_>>()
    });
    versions.append(&mut remote_versions);
    versions
}

fn matching_item_versions(
    server: &SavedServer,
    target: &MediaItem,
    queries: &[String],
) -> Result<Vec<MediaVersion>, String> {
    let client = api::http_client_with_timeout(server.use_system_proxy, Duration::from_secs(8))?;
    for query in queries {
        if let Some(candidate) = api::search_items(&client, server, query)?
            .into_iter()
            .find(|candidate| same_media_identity(target, candidate))
        {
            return api::get_media_versions(&client, server, &candidate.id);
        }
    }
    Ok(Vec::new())
}

fn version_search_queries(item: &MediaItem) -> Vec<String> {
    let mut queries = Vec::new();
    if item.item_type.eq_ignore_ascii_case("Episode") {
        if let Some(series_name) = item.series_name.as_deref() {
            push_version_query(&mut queries, series_name);
        }
    }
    push_version_query(&mut queries, &item.name);
    queries
}

fn push_version_query(queries: &mut Vec<String>, value: &str) {
    let query = value.trim();
    if !query.is_empty() && !queries.iter().any(|existing| existing == query) {
        queries.push(query.to_string());
    }
}

fn same_media_identity(target: &MediaItem, candidate: &MediaItem) -> bool {
    if target.item_type.eq_ignore_ascii_case("Episode") {
        return candidate.item_type.eq_ignore_ascii_case("Episode")
            && same_episode_identity(target, candidate);
    }
    compatible_item_type(&target.item_type, &candidate.item_type)
        && same_text(&target.name, &candidate.name)
        && same_year_when_known(target.year, candidate.year)
}

fn same_episode_identity(target: &MediaItem, candidate: &MediaItem) -> bool {
    let same_season = match (target.season_number, candidate.season_number) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    };
    let same_episode = match (target.episode_number, candidate.episode_number) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    };
    let same_number = same_season && same_episode;
    let same_series = target
        .series_name
        .as_deref()
        .zip(candidate.series_name.as_deref())
        .map(|(left, right)| same_text(left, right))
        .unwrap_or(false);
    if same_series && same_number {
        return true;
    }
    same_text(&target.name, &candidate.name)
        && same_series
        && same_year_when_known(target.year, candidate.year)
}

fn compatible_item_type(left: &str, right: &str) -> bool {
    let left = left.to_ascii_lowercase();
    let right = right.to_ascii_lowercase();
    left == right
        || matches!(
            (left.as_str(), right.as_str()),
            ("movie", "video") | ("video", "movie")
        )
}

fn same_year_when_known(left: Option<i64>, right: Option<i64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left == right,
        _ => true,
    }
}

fn same_text(left: &str, right: &str) -> bool {
    let left = normalized_match_text(left);
    !left.is_empty() && left == normalized_match_text(right)
}

fn normalized_match_text(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .collect()
}

fn normalize_library_filters(input: Option<LibraryFiltersInput>) -> api::LibraryQueryFilters {
    let input = input.unwrap_or_default();
    api::LibraryQueryFilters {
        played: match input.played.as_deref() {
            Some("played") => Some(true),
            Some("unplayed") => Some(false),
            _ => None,
        },
        favorite: input.favorite,
        genre: input
            .genre
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        person_id: input
            .person_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        collection_id: input
            .collection_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    }
}

#[tauri::command]
pub(crate) async fn load_library(
    app: AppHandle,
    input: LibraryInput,
) -> Result<LibraryPayload, String> {
    tauri::async_runtime::spawn_blocking(move || load_library_sync(app, input))
        .await
        .map_err(|err| err.to_string())?
}

fn load_library_sync(app: AppHandle, input: LibraryInput) -> Result<LibraryPayload, String> {
    let server = store::active_server(&app)?;
    let client = api::http_client(server.use_system_proxy)?;
    let start_index = input.start_index.unwrap_or(0);
    let limit = input.limit.unwrap_or(60).clamp(1, 120);
    let item_type = normalize_item_type(input.item_type.as_deref());
    let (sort_by, sort_order) =
        normalize_library_sort(input.sort_by.as_deref(), input.sort_order.as_deref());
    let filters = normalize_library_filters(input.filters.clone());
    let (library, (items, total_count)) = if start_index == 0 {
        thread::scope(|scope| {
            let items = scope.spawn(|| {
                api::get_library_items(
                    &client,
                    &server,
                    api::LibraryItemsQuery {
                        library_id: &input.library_id,
                        start_index,
                        limit,
                        item_type,
                        sort_by,
                        sort_order,
                        filters: &filters,
                    },
                )
            });
            let library = if input.library_id.is_empty() {
                MediaLibrary {
                    id: input.library_id.clone(),
                    name: "Favorites".to_string(),
                    collection_type: None,
                    image_url: None,
                }
            } else {
                api::fetch_libraries(&client, &server)?
                    .into_iter()
                    .find(|library| library.id == input.library_id)
                    .ok_or_else(|| "Library not found.".to_string())?
            };
            let items = items
                .join()
                .map_err(|_| "Failed to load library items.".to_string())??;
            Ok::<_, String>((library, items))
        })?
    } else {
        (
            MediaLibrary {
                id: input.library_id.clone(),
                name: String::new(),
                collection_type: None,
                image_url: None,
            },
            api::get_library_items(
                &client,
                &server,
                api::LibraryItemsQuery {
                    library_id: &input.library_id,
                    start_index,
                    limit,
                    item_type,
                    sort_by,
                    sort_order,
                    filters: &filters,
                },
            )?,
        )
    };
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

#[tauri::command]
pub(crate) async fn load_item(
    app: AppHandle,
    input: ItemInput,
) -> Result<ItemDetailPayload, String> {
    tauri::async_runtime::spawn_blocking(move || load_item_sync(app, input))
        .await
        .map_err(|err| err.to_string())?
}

fn load_item_sync(app: AppHandle, input: ItemInput) -> Result<ItemDetailPayload, String> {
    let server = server_for_input(&app, input.server_id.as_deref())?;
    let client = api::http_client(server.use_system_proxy)?;
    let raw = api::get_item_raw(&client, &server, &input.item_id)?;
    let entry_item = api::map_item(&server, raw.clone());
    let detail_raw = entry_item
        .series_id
        .as_deref()
        .filter(|_| entry_item.item_type == "Episode")
        .and_then(|series_id| api::get_item_raw(&client, &server, series_id).ok())
        .unwrap_or_else(|| raw.clone());
    let item = api::map_item(&server, detail_raw.clone());
    let series_id = if item.item_type == "Series" {
        Some(item.id.as_str())
    } else {
        item.series_id.as_deref()
    };
    let (children, seasons, episodes_result) = thread::scope(|scope| {
        let children = scope.spawn(|| api::get_item_children(&client, &server, &item.id));
        let seasons = scope.spawn(|| {
            series_id
                .map(|id| api::get_show_seasons(&client, &server, id))
                .unwrap_or_else(|| Ok(Vec::new()))
        });
        let episodes = scope.spawn(|| {
            series_id
                .map(|id| api::get_show_episodes(&client, &server, id, None))
                .unwrap_or_else(|| Ok((Vec::new(), 0)))
        });
        Ok::<_, String>((
            children
                .join()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default(),
            seasons.join().ok().and_then(Result::ok).unwrap_or_default(),
            episodes
                .join()
                .ok()
                .and_then(Result::ok)
                .unwrap_or_default(),
        ))
    })?;
    let (episodes, episode_total_count) = episodes_result;
    let selected_media_item = if raw.item_type.as_deref() == Some("Episode") {
        entry_item.clone()
    } else {
        episodes.first().cloned().unwrap_or_else(|| item.clone())
    };
    let media_sources = load_aggregated_media_versions(&app, &server, &selected_media_item);

    Ok(ItemDetailPayload {
        item,
        children,
        seasons,
        episodes,
        episode_total_count: Some(episode_total_count),
        media_sources,
        people: Vec::new(),
        art: Vec::new(),
        similar: Vec::new(),
    })
}

#[tauri::command]
pub(crate) async fn load_item_more(
    app: AppHandle,
    input: ItemInput,
) -> Result<ItemMorePayload, String> {
    tauri::async_runtime::spawn_blocking(move || load_item_more_sync(app, input))
        .await
        .map_err(|err| err.to_string())?
}

fn load_item_more_sync(app: AppHandle, input: ItemInput) -> Result<ItemMorePayload, String> {
    let server = server_for_input(&app, input.server_id.as_deref())?;
    let client = api::http_client(server.use_system_proxy)?;
    let raw = api::get_item_raw(&client, &server, &input.item_id)?;
    let item = api::map_item(&server, raw.clone());
    let detail_raw = item
        .series_id
        .as_deref()
        .filter(|_| item.item_type == "Episode")
        .and_then(|series_id| api::get_item_raw(&client, &server, series_id).ok())
        .unwrap_or_else(|| raw.clone());
    let detail_item = api::map_item(&server, detail_raw.clone());
    let similar = api::get_similar_items(&client, &server, &detail_item.id).unwrap_or_default();

    Ok(ItemMorePayload {
        item_id: input.item_id,
        people: api::people(&server, &detail_raw),
        art: api::art_urls(&server, &detail_raw),
        similar,
    })
}

#[tauri::command]
pub(crate) async fn load_media_sources(
    app: AppHandle,
    input: ItemInput,
) -> Result<Vec<crate::models::MediaVersion>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let server = server_for_input(&app, input.server_id.as_deref())?;
        let client = api::http_client(server.use_system_proxy)?;
        let item = api::map_item(
            &server,
            api::get_item_raw(&client, &server, &input.item_id)?,
        );
        Ok(load_aggregated_media_versions(&app, &server, &item))
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) async fn search_items(
    app: AppHandle,
    input: SearchInput,
) -> Result<SearchPayload, String> {
    tauri::async_runtime::spawn_blocking(move || search_items_sync(app, input))
        .await
        .map_err(|err| err.to_string())?
}

fn search_items_sync(app: AppHandle, input: SearchInput) -> Result<SearchPayload, String> {
    let query = input.query.trim();
    if query.is_empty() {
        return Ok(SearchPayload { items: Vec::new() });
    }

    let servers = store::servers(&app)?;
    if servers.is_empty() {
        return Ok(SearchPayload { items: Vec::new() });
    }
    let (items, errors) = thread::scope(|scope| {
        servers
            .iter()
            .map(|server| {
                scope.spawn(|| {
                    let client = api::http_client_with_timeout(
                        server.use_system_proxy,
                        Duration::from_secs(8),
                    )?;
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

#[tauri::command]
pub(crate) async fn play_item(app: AppHandle, input: ItemInput) -> Result<PlayResult, String> {
    tauri::async_runtime::spawn_blocking(move || play_item_sync(app, input))
        .await
        .map_err(|err| err.to_string())?
}

fn play_item_sync(app: AppHandle, input: ItemInput) -> Result<PlayResult, String> {
    let server = server_for_input(&app, input.server_id.as_deref())?;
    let client = api::http_client(server.use_system_proxy)?;
    let item = api::resolve_playable_item(&client, &server, &input.item_id)?;
    let _ = store::remember_recent_play(&app, &server.id, &item.id);
    let start_position_ticks = item
        .user_data
        .as_ref()
        .and_then(|data| data.playback_position_ticks)
        .filter(|ticks| *ticks > 0);
    let fallback_play_session_id = crate::playback_watch::play_session_id(&item.id);
    let playback = api::playback_launch_info(
        &client,
        &server,
        &item.id,
        input.media_source_id.as_deref(),
        input.audio_stream_index,
        input.subtitle_stream_index,
        &fallback_play_session_id,
    )?;
    let play_session_id = playback.play_session_id.clone();
    let media_source_id = playback.media_source_id.clone();
    if mpv::terminate_all() {
        thread::sleep(Duration::from_millis(300));
    }
    let launch = mpv::launch(
        &app,
        &server,
        mpv::LaunchRequest {
            item_id: &item.id,
            media_source_id,
            play_session_id: play_session_id.clone(),
            stream_url: &playback.stream_url,
            subtitle_track_position: input.subtitle_stream_position,
            subtitle_url: playback.subtitle_url.as_deref(),
            start_position_ticks,
        },
    )?;
    let result = launch.result;
    let watched_item_id = result.item_id.clone();
    let watched_media_source_id = result.media_source_id.clone();
    let watched_audio_stream_index = input.audio_stream_index;
    let watched_subtitle_stream_index = input.subtitle_stream_index;
    thread::spawn(move || {
        crate::playback_watch::watch_mpv_playback(crate::playback_watch::WatchMpvPlaybackInput {
            app,
            server,
            item_id: watched_item_id,
            media_source_id: watched_media_source_id,
            play_session_id,
            audio_stream_index: watched_audio_stream_index,
            subtitle_stream_index: watched_subtitle_stream_index,
            start_position_ticks,
        });
    });
    Ok(result)
}

#[tauri::command]
pub(crate) async fn control_playback(input: PlaybackControlInput) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        mpv::control(&input.play_session_id, &input.command)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) async fn playback_state(
    input: PlaybackStateInput,
) -> Result<PlaybackStateResult, String> {
    tauri::async_runtime::spawn_blocking(move || mpv::refresh_state(&input.play_session_id))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) async fn save_playback_preference(
    app: AppHandle,
    input: SavePlaybackPreferenceInput,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let server = server_for_input(&app, input.server_id.as_deref())?;
        let key = store::playback_preference_key(input.series_id.as_deref(), &input.item_id);
        store::save_playback_preference(
            &app,
            &server.id,
            key,
            PlaybackPreference {
                media_source_id: input.media_source_id,
                audio_stream_index: input.audio_stream_index,
                audio_language: input.audio_language,
                subtitle_stream_index: input.subtitle_stream_index,
                subtitle_language: input.subtitle_language,
            },
        )
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) async fn load_playback_preferences(
    app: AppHandle,
) -> Result<std::collections::HashMap<String, PlaybackPreference>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let server = store::active_server(&app)?;
        store::playback_preferences(&app, &server.id)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) async fn mark_favorite(app: AppHandle, input: MarkInput) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || mark_item(app, input, api::set_favorite))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
pub(crate) async fn mark_played(app: AppHandle, input: MarkInput) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || mark_item(app, input, api::set_played))
        .await
        .map_err(|err| err.to_string())?
}

fn mark_item(
    app: AppHandle,
    input: MarkInput,
    mark: fn(&reqwest::blocking::Client, &SavedServer, &str, bool) -> Result<(), String>,
) -> Result<(), String> {
    let server = server_for_input(&app, input.server_id.as_deref())?;
    let client = api::http_client(server.use_system_proxy)?;
    mark(&client, &server, &input.item_id, input.value)
}

#[tauri::command]
pub(crate) async fn fetch_server_name(
    input: FetchServerNameInput,
) -> Result<FetchServerNameResult, String> {
    tauri::async_runtime::spawn_blocking(move || fetch_server_name_sync(input))
        .await
        .map_err(|err| err.to_string())?
}

fn fetch_server_name_sync(input: FetchServerNameInput) -> Result<FetchServerNameResult, String> {
    let _server_type = input.server_type.as_str();
    let url = api::normalize_server_url(&input.url)?;
    let client = api::http_client(input.use_system_proxy)?;
    let info = api::fetch_server_public_info(&client, &url).unwrap_or_default();
    let name = info
        .server_name
        .or(info.product_name)
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| api::clean_name("", "", &url));
    Ok(FetchServerNameResult { name })
}

#[tauri::command]
pub(crate) async fn report_playback_start(
    app: AppHandle,
    input: ReportPlaybackStartInput,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || report_playback_start_sync(app, input))
        .await
        .map_err(|err| err.to_string())?
}

fn report_playback_start_sync(
    app: AppHandle,
    input: ReportPlaybackStartInput,
) -> Result<(), String> {
    let server = store::active_server(&app)?;
    let client = api::http_client(server.use_system_proxy)?;
    api::report_playback_start(&client, &server, &input)
}

#[tauri::command]
pub(crate) async fn report_playback_progress(
    app: AppHandle,
    input: ReportPlaybackProgressInput,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || report_playback_progress_sync(app, input))
        .await
        .map_err(|err| err.to_string())?
}

fn report_playback_progress_sync(
    app: AppHandle,
    input: ReportPlaybackProgressInput,
) -> Result<(), String> {
    let server = store::active_server(&app)?;
    let client = api::http_client(server.use_system_proxy)?;
    api::report_playback_progress(&client, &server, &input)
}

#[tauri::command]
pub(crate) async fn report_playback_stopped(
    app: AppHandle,
    input: ReportPlaybackStoppedInput,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || report_playback_stopped_sync(app, input))
        .await
        .map_err(|err| err.to_string())?
}

fn report_playback_stopped_sync(
    app: AppHandle,
    input: ReportPlaybackStoppedInput,
) -> Result<(), String> {
    let server = store::active_server(&app)?;
    let client = api::http_client(server.use_system_proxy)?;
    api::report_playback_stopped(&client, &server, &input)
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

fn normalize_library_sort(
    sort_by: Option<&str>,
    sort_order: Option<&str>,
) -> (&'static str, &'static str) {
    let sort_by = match sort_by {
        Some("SortName") => "SortName",
        Some("PremiereDate") => "PremiereDate",
        Some("CommunityRating") => "CommunityRating",
        _ => "DateCreated",
    };
    let sort_order = match sort_order {
        Some("Ascending") => "Ascending",
        _ => "Descending",
    };
    (sort_by, sort_order)
}
