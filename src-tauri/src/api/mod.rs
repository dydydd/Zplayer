mod actions;
mod http;
mod images;
mod mapping;

pub(crate) use self::actions::{
    report_playback_progress, report_playback_start, report_playback_stopped, set_favorite,
    set_played,
};
use self::http::{auth_header, build_url, get_json, read_json};
pub(crate) use self::http::{http_client, http_client_with_timeout};
use self::images::{media_image_url_indexed, media_playback_url, primary_image_url};
use self::mapping::{first_playable_child, is_playable_item_type, map_search_hint, stream_info};
pub(crate) use self::mapping::{map_item, people};
use crate::models::{
    ApiItem, AuthResponse, ItemCounts, ItemsResponse, MediaArt, MediaItem, MediaLibrary,
    MediaSource, MediaVersion, PlaybackInfo, PublicSystemInfo, RecommendationsResponse,
    SavedServer, SearchHintResult,
};
use reqwest::blocking::Client;
use reqwest::Url;

pub(crate) fn authenticate_by_name(
    client: &Client,
    base_url: &str,
    server_type: &str,
    username: &str,
    password: &str,
) -> Result<AuthResponse, String> {
    let body = serde_json::json!({
        "Username": username,
        "Pw": password,
        "Password": password,
    });
    let preferred = if server_type.eq_ignore_ascii_case("emby") {
        "Emby"
    } else {
        "MediaBrowser"
    };
    authenticate_by_name_with_prefix(client, base_url, preferred, &body).or_else(|_| {
        let fallback = if preferred == "Emby" {
            "MediaBrowser"
        } else {
            "Emby"
        };
        authenticate_by_name_with_prefix(client, base_url, fallback, &body)
    })
}

fn authenticate_by_name_with_prefix(
    client: &Client,
    base_url: &str,
    prefix: &str,
    body: &serde_json::Value,
) -> Result<AuthResponse, String> {
    let url = build_url(base_url, "Users/AuthenticateByName", &[])?;
    let auth = auth_header(prefix);
    let response = client
        .post(url)
        .header("X-Emby-Authorization", &auth)
        .header("Authorization", &auth)
        .json(body)
        .send()
        .map_err(|err| format!("Server request failed: {err}"))?;
    read_json(response, "Users/AuthenticateByName")
}

pub(crate) fn fetch_libraries(
    client: &Client,
    server: &SavedServer,
) -> Result<Vec<MediaLibrary>, String> {
    let emby_params = vec![
        ("Fields", "PrimaryImageAspectRatio".to_string()),
        ("IncludeExternalContent", "false".to_string()),
    ];
    let jellyfin_params = vec![
        ("userId", server.user_id.clone()),
        ("includeExternalContent", "false".to_string()),
    ];
    let response = get_json::<ItemsResponse>(client, server, "Users/{user_id}/Views", &emby_params)
        .or_else(|_| get_json::<ItemsResponse>(client, server, "UserViews", &jellyfin_params))?;
    Ok(response
        .into_items()
        .into_iter()
        .map(|item| {
            let image_url = primary_image_url(server, &item);
            MediaLibrary {
                id: item.id.clone(),
                name: item.name.unwrap_or_else(|| "Untitled".to_string()),
                collection_type: item.collection_type,
                image_url,
            }
        })
        .collect())
}

pub(crate) fn item_counts(client: &Client, server: &SavedServer) -> Result<ItemCounts, String> {
    get_json(client, server, "Items/Counts", &[])
}

pub(crate) fn get_latest_items(
    client: &Client,
    server: &SavedServer,
    parent_id: Option<&str>,
    limit: &str,
) -> Result<Vec<MediaItem>, String> {
    let mut emby_params = vec![
        ("Limit", limit.to_string()),
        ("Fields", item_fields()),
        ("EnableImages", "true".to_string()),
        ("EnableImageTypes", image_types()),
        ("ImageTypeLimit", "1".to_string()),
    ];
    let mut jellyfin_params = vec![
        ("userId", server.user_id.clone()),
        ("limit", limit.to_string()),
        ("fields", item_fields()),
        ("enableImages", "true".to_string()),
        ("enableImageTypes", image_types()),
        ("imageTypeLimit", "1".to_string()),
    ];
    if let Some(parent_id) = parent_id {
        emby_params.push(("ParentId", parent_id.to_string()));
        jellyfin_params.push(("parentId", parent_id.to_string()));
    }
    get_items(client, server, "Users/{user_id}/Items/Latest", &emby_params)
        .or_else(|_| get_items(client, server, "Items/Latest", &jellyfin_params))
}

pub(crate) fn get_library_items(
    client: &Client,
    server: &SavedServer,
    library_id: &str,
    start_index: usize,
    limit: usize,
    item_type: Option<&str>,
    sort_by: &str,
    sort_order: &str,
) -> Result<(Vec<MediaItem>, usize), String> {
    let mut emby_params = vec![
        ("ParentId", library_id.to_string()),
        ("Recursive", "true".to_string()),
        ("StartIndex", start_index.to_string()),
        ("Limit", limit.to_string()),
        ("EnableTotalRecordCount", "true".to_string()),
        ("SortBy", sort_by.to_string()),
        ("SortOrder", sort_order.to_string()),
        ("Fields", item_fields()),
        ("EnableImages", "true".to_string()),
        ("EnableImageTypes", image_types()),
        ("ImageTypeLimit", "1".to_string()),
    ];
    let mut jellyfin_params = vec![
        ("userId", server.user_id.clone()),
        ("parentId", library_id.to_string()),
        ("recursive", "true".to_string()),
        ("startIndex", start_index.to_string()),
        ("limit", limit.to_string()),
        ("enableTotalRecordCount", "true".to_string()),
        ("sortBy", sort_by.to_string()),
        ("sortOrder", sort_order.to_string()),
        ("fields", item_fields()),
        ("enableImages", "true".to_string()),
        ("enableImageTypes", image_types()),
        ("imageTypeLimit", "1".to_string()),
    ];
    if let Some(item_type) = item_type.filter(|value| !value.is_empty()) {
        emby_params.push(("IncludeItemTypes", item_type.to_string()));
        jellyfin_params.push(("includeItemTypes", item_type.to_string()));
    }
    get_items_page(client, server, "Users/{user_id}/Items", &emby_params)
        .or_else(|_| get_items_page(client, server, "Items", &jellyfin_params))
}

pub(crate) fn get_item_children(
    client: &Client,
    server: &SavedServer,
    parent_id: &str,
) -> Result<Vec<MediaItem>, String> {
    Ok(
        get_recursive_children_raw(client, server, parent_id, "60", "SortName", None)?
            .into_iter()
            .map(|item| map_item(server, item))
            .collect(),
    )
}

pub(crate) fn get_show_seasons(
    client: &Client,
    server: &SavedServer,
    series_id: &str,
) -> Result<Vec<MediaItem>, String> {
    let params = [
        ("UserId", server.user_id.clone()),
        ("userId", server.user_id.clone()),
        ("Fields", item_fields()),
        ("fields", item_fields()),
        ("EnableImages", "true".to_string()),
        ("enableImages", "true".to_string()),
        ("EnableUserData", "true".to_string()),
        ("enableUserData", "true".to_string()),
    ];
    get_items(
        client,
        server,
        &format!("Shows/{series_id}/Seasons"),
        &params,
    )
}

pub(crate) fn get_show_episodes(
    client: &Client,
    server: &SavedServer,
    series_id: &str,
    season_id: Option<&str>,
) -> Result<Vec<MediaItem>, String> {
    let mut params = vec![
        ("UserId", server.user_id.clone()),
        ("userId", server.user_id.clone()),
        ("Fields", item_fields()),
        ("fields", item_fields()),
        (
            "SortBy",
            "ParentIndexNumber,IndexNumber,SortName".to_string(),
        ),
        (
            "sortBy",
            "ParentIndexNumber,IndexNumber,SortName".to_string(),
        ),
        ("SortOrder", "Ascending".to_string()),
        ("sortOrder", "Ascending".to_string()),
        ("EnableImages", "true".to_string()),
        ("enableImages", "true".to_string()),
        ("EnableUserData", "true".to_string()),
        ("enableUserData", "true".to_string()),
    ];
    if let Some(season_id) = season_id {
        params.push(("SeasonId", season_id.to_string()));
        params.push(("seasonId", season_id.to_string()));
    }
    get_items(
        client,
        server,
        &format!("Shows/{series_id}/Episodes"),
        &params,
    )
}

pub(crate) fn get_similar_items(
    client: &Client,
    server: &SavedServer,
    item_id: &str,
) -> Result<Vec<MediaItem>, String> {
    let params = [
        ("UserId", server.user_id.clone()),
        ("userId", server.user_id.clone()),
        ("Limit", "14".to_string()),
        ("limit", "14".to_string()),
        ("Fields", item_fields()),
        ("fields", item_fields()),
        ("EnableImages", "true".to_string()),
        ("enableImages", "true".to_string()),
        ("EnableUserData", "true".to_string()),
        ("enableUserData", "true".to_string()),
        ("EnableImageTypes", image_types()),
        ("enableImageTypes", image_types()),
    ];
    get_items(client, server, &format!("Items/{item_id}/Similar"), &params)
}

pub(crate) fn get_media_versions(
    client: &Client,
    server: &SavedServer,
    item_id: &str,
) -> Result<Vec<MediaVersion>, String> {
    Ok(playback_info(client, server, item_id, None, None, None)?
        .media_sources
        .into_iter()
        .enumerate()
        .map(|(index, source)| {
            let id = source.id.clone().unwrap_or_else(|| index.to_string());
            let video = source
                .media_streams
                .iter()
                .find(|stream| stream.stream_type.as_deref() == Some("Video"));
            let audio = source
                .media_streams
                .iter()
                .find(|stream| stream.stream_type.as_deref() == Some("Audio"));
            MediaVersion {
                id,
                name: source
                    .name
                    .or(source.container.clone())
                    .unwrap_or_else(|| format!("鐗堟湰 {}", index + 1)),
                container: source.container,
                path: source.path,
                protocol: source.protocol,
                bitrate: source.bitrate,
                size: source.size,
                video_codec: video.and_then(|stream| stream.codec.clone()),
                video_display_title: video.and_then(|stream| stream.display_title.clone()),
                video_range: video.and_then(|stream| stream.video_range.clone()),
                video_profile: video.and_then(|stream| stream.profile.clone()),
                video_level: video.and_then(|stream| stream.level),
                aspect_ratio: video.and_then(|stream| stream.aspect_ratio.clone()),
                interlaced: video.and_then(|stream| stream.is_interlaced),
                bit_depth: video.and_then(|stream| stream.bit_depth),
                pixel_format: video.and_then(|stream| stream.pixel_format.clone()),
                resolution: video.and_then(|stream| {
                    stream
                        .width
                        .zip(stream.height)
                        .map(|(width, height)| format!("{width}x{height}"))
                }),
                frame_rate: video.and_then(|stream| stream.real_frame_rate),
                audio_codec: audio.and_then(|stream| stream.codec.clone()),
                audio_display_title: audio.and_then(|stream| stream.display_title.clone()),
                audio_title: audio.and_then(|stream| stream.title.clone()),
                audio_language: audio.and_then(|stream| stream.language.clone()),
                channel_layout: audio.and_then(|stream| stream.channel_layout.clone()),
                audio_channels: audio.and_then(|stream| stream.channels),
                audio_bitrate: audio.and_then(|stream| stream.bitrate),
                sample_rate: audio.and_then(|stream| stream.sample_rate),
                audio_external: audio.and_then(|stream| stream.is_external),
                audio_default: audio.and_then(|stream| stream.is_default),
                audio_streams: source
                    .media_streams
                    .iter()
                    .filter(|stream| stream.stream_type.as_deref() == Some("Audio"))
                    .map(stream_info)
                    .collect(),
                subtitle_count: source
                    .media_streams
                    .iter()
                    .filter(|stream| stream.stream_type.as_deref() == Some("Subtitle"))
                    .count(),
                subtitle_languages: source
                    .media_streams
                    .iter()
                    .filter(|stream| stream.stream_type.as_deref() == Some("Subtitle"))
                    .filter_map(|stream| stream.language.clone().or(stream.title.clone()))
                    .collect(),
                subtitle_streams: source
                    .media_streams
                    .iter()
                    .filter(|stream| stream.stream_type.as_deref() == Some("Subtitle"))
                    .map(stream_info)
                    .collect(),
            }
        })
        .collect())
}

pub(crate) fn art_urls(server: &SavedServer, item: &ApiItem) -> Vec<MediaArt> {
    let mut art = Vec::new();
    for index in 0..item.backdrop_image_tags.len().min(12) {
        if let Ok(url) = media_image_url_indexed(
            server,
            &item.id,
            "Backdrop",
            index,
            &[("fillWidth", "1600")],
        ) {
            art.push(MediaArt {
                image_type: "Backdrop".to_string(),
                url,
            });
        }
    }
    for index in 0..item.screenshot_image_tags.len().min(8) {
        if let Ok(url) = media_image_url_indexed(
            server,
            &item.id,
            "Screenshot",
            index,
            &[("fillWidth", "1200")],
        ) {
            art.push(MediaArt {
                image_type: "Screenshot".to_string(),
                url,
            });
        }
    }
    art
}

pub(crate) fn get_recommendations(
    client: &Client,
    server: &SavedServer,
    params: &[(&str, String)],
) -> Result<Vec<MediaItem>, String> {
    let response: RecommendationsResponse =
        get_json(client, server, "Movies/Recommendations", params)?;
    Ok(response
        .into_items()
        .into_iter()
        .map(|item| map_item(server, item))
        .collect())
}

pub(crate) fn get_suggested_items(
    client: &Client,
    server: &SavedServer,
    item_type: &str,
) -> Result<Vec<MediaItem>, String> {
    let params = [
        ("UserId", server.user_id.clone()),
        ("userId", server.user_id.clone()),
        ("Limit", "12".to_string()),
        ("limit", "12".to_string()),
        ("Type", item_type.to_string()),
        ("type", item_type.to_string()),
        ("IncludeItemTypes", item_type.to_string()),
        ("includeItemTypes", item_type.to_string()),
        ("Fields", item_fields()),
        ("fields", item_fields()),
        ("EnableImageTypes", image_types()),
        ("enableImageTypes", image_types()),
    ];
    let items = get_items(client, server, "Items/Suggestions", &params)
        .or_else(|_| get_items(client, server, "Users/{user_id}/Suggestions", &params))?;
    Ok(items
        .into_iter()
        .filter(|item| item.item_type == item_type)
        .collect())
}

pub(crate) fn get_resume_items(
    client: &Client,
    server: &SavedServer,
) -> Result<Vec<MediaItem>, String> {
    let params = [
        ("UserId", server.user_id.clone()),
        ("userId", server.user_id.clone()),
        ("Limit", "16".to_string()),
        ("limit", "16".to_string()),
        ("Recursive", "true".to_string()),
        ("recursive", "true".to_string()),
        ("IncludeItemTypes", "Movie,Episode,Video".to_string()),
        ("includeItemTypes", "Movie,Episode,Video".to_string()),
        ("Fields", item_fields()),
        ("fields", item_fields()),
        ("EnableUserData", "true".to_string()),
        ("enableUserData", "true".to_string()),
        ("EnableImageTypes", image_types()),
        ("enableImageTypes", image_types()),
    ];
    get_items(client, server, "UserItems/Resume", &params)
        .or_else(|_| get_items(client, server, "Users/{user_id}/Items/Resume", &params))
}

pub(crate) fn search_items(
    client: &Client,
    server: &SavedServer,
    query: &str,
) -> Result<Vec<MediaItem>, String> {
    let result: SearchHintResult = get_json(
        client,
        server,
        "Search/Hints",
        &[
            ("UserId", server.user_id.clone()),
            ("userId", server.user_id.clone()),
            ("SearchTerm", query.to_string()),
            ("searchTerm", query.to_string()),
            ("Limit", "30".to_string()),
            ("limit", "30".to_string()),
            ("IncludeMedia", "true".to_string()),
            ("includeMedia", "true".to_string()),
            ("IncludePeople", "false".to_string()),
            ("includePeople", "false".to_string()),
            ("IncludeGenres", "false".to_string()),
            ("includeGenres", "false".to_string()),
            ("IncludeStudios", "false".to_string()),
            ("includeStudios", "false".to_string()),
            ("IncludeArtists", "false".to_string()),
            ("includeArtists", "false".to_string()),
            ("IncludeItemTypes", "Movie,Series,Episode,Video".to_string()),
            ("includeItemTypes", "Movie,Series,Episode,Video".to_string()),
        ],
    )?;

    Ok(result
        .search_hints
        .into_iter()
        .filter_map(|hint| map_search_hint(server, hint))
        .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlaybackLaunchInfo {
    pub(crate) stream_url: String,
    pub(crate) media_source_id: Option<String>,
    pub(crate) play_session_id: String,
}

pub(crate) fn playback_launch_info(
    client: &Client,
    server: &SavedServer,
    item_id: &str,
    media_source_id: Option<&str>,
    audio_stream_index: Option<i32>,
    subtitle_stream_index: Option<i32>,
    fallback_play_session_id: &str,
) -> Result<PlaybackLaunchInfo, String> {
    if let Ok(info) = playback_info(
        client,
        server,
        item_id,
        media_source_id,
        audio_stream_index,
        subtitle_stream_index,
    ) {
        return playback_launch_info_from_response(
            server,
            item_id,
            info,
            media_source_id,
            audio_stream_index,
            subtitle_stream_index,
            fallback_play_session_id,
        );
    }

    let stream_url = fallback_stream_url(
        server,
        item_id,
        media_source_id,
        audio_stream_index,
        subtitle_stream_index,
        fallback_play_session_id,
    )?;
    Ok(PlaybackLaunchInfo {
        stream_url,
        media_source_id: media_source_id.map(str::to_string),
        play_session_id: fallback_play_session_id.to_string(),
    })
}

fn playback_launch_info_from_response(
    server: &SavedServer,
    item_id: &str,
    info: PlaybackInfo,
    requested_media_source_id: Option<&str>,
    audio_stream_index: Option<i32>,
    subtitle_stream_index: Option<i32>,
    fallback_play_session_id: &str,
) -> Result<PlaybackLaunchInfo, String> {
    let play_session_id = info
        .play_session_id
        .filter(|id| !id.trim().is_empty())
        .unwrap_or_else(|| fallback_play_session_id.to_string());
    let sources = ordered_media_sources(&info.media_sources, requested_media_source_id);

    for source in &sources {
        if let Some(raw_url) = source
            .direct_stream_url
            .as_deref()
            .or(source.transcoding_url.as_deref())
        {
            let stream_url = append_play_session_id(
                &media_playback_url(server, raw_url)?,
                Some(&play_session_id),
            )?;
            return Ok(PlaybackLaunchInfo {
                stream_url,
                media_source_id: source
                    .id
                    .clone()
                    .or_else(|| requested_media_source_id.map(str::to_string)),
                play_session_id,
            });
        }

        if let Some(source_id) = source.id.as_deref() {
            let stream_url = fallback_stream_url(
                server,
                item_id,
                Some(source_id),
                audio_stream_index,
                subtitle_stream_index,
                &play_session_id,
            )?;
            return Ok(PlaybackLaunchInfo {
                stream_url,
                media_source_id: Some(source_id.to_string()),
                play_session_id,
            });
        }
    }

    let stream_url = fallback_stream_url(
        server,
        item_id,
        requested_media_source_id,
        audio_stream_index,
        subtitle_stream_index,
        &play_session_id,
    )?;
    Ok(PlaybackLaunchInfo {
        stream_url,
        media_source_id: requested_media_source_id.map(str::to_string),
        play_session_id,
    })
}

fn ordered_media_sources<'a>(
    sources: &'a [MediaSource],
    requested_media_source_id: Option<&str>,
) -> Vec<&'a MediaSource> {
    let mut ordered = Vec::new();
    if let Some(requested) = requested_media_source_id {
        if let Some(source) = sources
            .iter()
            .find(|source| source.id.as_deref() == Some(requested))
        {
            ordered.push(source);
        }
    }
    for source in sources {
        if !ordered
            .iter()
            .any(|selected| std::ptr::eq(*selected, source))
        {
            ordered.push(source);
        }
    }
    ordered
}

fn fallback_stream_url(
    server: &SavedServer,
    item_id: &str,
    media_source_id: Option<&str>,
    audio_stream_index: Option<i32>,
    subtitle_stream_index: Option<i32>,
    play_session_id: &str,
) -> Result<String, String> {
    let mut params = vec![
        ("static", "true".to_string()),
        ("api_key", server.access_token.clone()),
        ("PlaySessionId", play_session_id.to_string()),
        ("playSessionId", play_session_id.to_string()),
    ];
    if let Some(source_id) = media_source_id {
        params.push(("MediaSourceId", source_id.to_string()));
        params.push(("mediaSourceId", source_id.to_string()));
    }
    if let Some(index) = audio_stream_index {
        params.push(("AudioStreamIndex", index.to_string()));
        params.push(("audioStreamIndex", index.to_string()));
    }
    if let Some(index) = subtitle_stream_index {
        if index < 0 {
            params.push(("SubtitleMethod", "None".to_string()));
            params.push(("subtitleMethod", "None".to_string()));
        } else {
            params.push(("SubtitleStreamIndex", index.to_string()));
            params.push(("subtitleStreamIndex", index.to_string()));
        }
    }
    Ok(build_url(&server.url, &format!("Videos/{item_id}/stream"), &params)?.to_string())
}

fn append_play_session_id(raw_url: &str, play_session_id: Option<&str>) -> Result<String, String> {
    let Some(play_session_id) = play_session_id.filter(|id| !id.trim().is_empty()) else {
        return Ok(raw_url.to_string());
    };
    let mut url = Url::parse(raw_url).map_err(|err| format!("Invalid playback URL: {err}"))?;
    if !url
        .query_pairs()
        .any(|(key, _)| key.eq_ignore_ascii_case("PlaySessionId"))
    {
        url.query_pairs_mut()
            .append_pair("PlaySessionId", play_session_id);
        url.query_pairs_mut()
            .append_pair("playSessionId", play_session_id);
    }
    Ok(url.to_string())
}

pub(crate) fn resolve_playable_item(
    client: &Client,
    server: &SavedServer,
    item_id: &str,
) -> Result<ApiItem, String> {
    let item = get_item_raw(client, server, item_id)?;
    if item
        .item_type
        .as_deref()
        .map(is_playable_item_type)
        .unwrap_or(false)
    {
        return Ok(item);
    }

    let item_name = item.name.clone().unwrap_or_else(|| item_id.to_string());
    let item_type = item.item_type.clone().unwrap_or_else(|| "Item".to_string());
    if item_type == "Series" {
        if let Some(episode) = first_playable_child(get_items_raw(
            client,
            server,
            &format!("Shows/{}/Episodes", item.id),
            &[
                ("UserId", server.user_id.clone()),
                ("userId", server.user_id.clone()),
                ("Fields", item_fields()),
                ("fields", item_fields()),
                (
                    "SortBy",
                    "ParentIndexNumber,IndexNumber,SortName".to_string(),
                ),
                (
                    "sortBy",
                    "ParentIndexNumber,IndexNumber,SortName".to_string(),
                ),
                ("SortOrder", "Ascending".to_string()),
                ("sortOrder", "Ascending".to_string()),
            ],
        )?) {
            return Ok(episode);
        }
    }

    let children = get_recursive_children_raw(
        client,
        server,
        &item.id,
        "50",
        "SortName",
        Some("Movie,Episode,Video"),
    )?;
    first_playable_child(children).ok_or_else(|| {
        format!(
            "{item_name} is {item_type}, and no playable Movie, Episode, or Video was found under it."
        )
    })
}

pub(crate) fn normalize_server_url(input: &str) -> Result<String, String> {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Server URL is required.".to_string());
    }
    let url = Url::parse(trimmed)
        .map_err(|_| "Server URL must include http:// or https://.".to_string())?;
    match url.scheme() {
        "http" | "https" => Ok(trimmed.to_string()),
        _ => Err("Server URL must start with http:// or https://.".to_string()),
    }
}

pub(crate) fn clean_name(name: &str, server_name: &str, url: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        let server_name = server_name.trim();
        if !server_name.is_empty() {
            return server_name.to_string();
        }
        return Url::parse(url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .unwrap_or_else(|| "Media Server".to_string());
    }
    trimmed.to_string()
}

pub(crate) fn server_id(url: &str, user_id: &str) -> String {
    let mut id = format!("{url}-{user_id}")
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    while id.contains("--") {
        id = id.replace("--", "-");
    }
    id.trim_matches('-').to_string()
}

fn get_items(
    client: &Client,
    server: &SavedServer,
    path: &str,
    params: &[(&str, String)],
) -> Result<Vec<MediaItem>, String> {
    Ok(get_items_raw(client, server, path, params)?
        .into_iter()
        .map(|item| map_item(server, item))
        .collect())
}

fn get_items_page(
    client: &Client,
    server: &SavedServer,
    path: &str,
    params: &[(&str, String)],
) -> Result<(Vec<MediaItem>, usize), String> {
    let response: ItemsResponse = get_json(client, server, path, params)?;
    let total = response.total_record_count();
    let items: Vec<MediaItem> = response
        .into_items()
        .into_iter()
        .map(|item| map_item(server, item))
        .collect();
    Ok((items, total.unwrap_or(0)))
}

fn get_items_raw(
    client: &Client,
    server: &SavedServer,
    path: &str,
    params: &[(&str, String)],
) -> Result<Vec<ApiItem>, String> {
    let response: ItemsResponse = get_json(client, server, path, params)?;
    Ok(response.into_items())
}

pub(crate) fn get_item_raw(
    client: &Client,
    server: &SavedServer,
    item_id: &str,
) -> Result<ApiItem, String> {
    let emby_path = format!("Users/{}/Items/{item_id}", server.user_id);
    let jellyfin_path = format!("Items/{item_id}");
    let emby_params = vec![
        ("Fields", item_fields()),
        ("EnableImageTypes", image_types()),
        ("EnableUserData", "true".to_string()),
    ];
    let jellyfin_params = vec![
        ("userId", server.user_id.clone()),
        ("fields", item_fields()),
        ("enableImageTypes", image_types()),
        ("enableUserData", "true".to_string()),
    ];
    get_json(client, server, &emby_path, &emby_params)
        .or_else(|_| get_json(client, server, &jellyfin_path, &jellyfin_params))
}

fn get_recursive_children_raw(
    client: &Client,
    server: &SavedServer,
    parent_id: &str,
    limit: &str,
    sort_by: &str,
    include_item_types: Option<&str>,
) -> Result<Vec<ApiItem>, String> {
    let mut emby_params = vec![
        ("ParentId", parent_id.to_string()),
        ("Recursive", "true".to_string()),
        ("Limit", limit.to_string()),
        ("SortBy", sort_by.to_string()),
        ("SortOrder", "Ascending".to_string()),
        ("Fields", item_fields()),
        ("EnableImageTypes", image_types()),
    ];
    let mut jellyfin_params = vec![
        ("userId", server.user_id.clone()),
        ("parentId", parent_id.to_string()),
        ("recursive", "true".to_string()),
        ("limit", limit.to_string()),
        ("sortBy", sort_by.to_string()),
        ("sortOrder", "Ascending".to_string()),
        ("fields", item_fields()),
        ("enableImageTypes", image_types()),
    ];
    if let Some(types) = include_item_types {
        emby_params.push(("IncludeItemTypes", types.to_string()));
        jellyfin_params.push(("includeItemTypes", types.to_string()));
    }
    get_items_raw(client, server, "Users/{user_id}/Items", &emby_params)
        .or_else(|_| get_items_raw(client, server, "Items", &jellyfin_params))
}

fn playback_info(
    client: &Client,
    server: &SavedServer,
    item_id: &str,
    media_source_id: Option<&str>,
    audio_stream_index: Option<i32>,
    subtitle_stream_index: Option<i32>,
) -> Result<PlaybackInfo, String> {
    let mut params = vec![
        ("UserId", server.user_id.clone()),
        ("userId", server.user_id.clone()),
        ("StartTimeTicks", "0".to_string()),
        ("startTimeTicks", "0".to_string()),
        ("IsPlayback", "true".to_string()),
        ("isPlayback", "true".to_string()),
        ("AutoOpenLiveStream", "true".to_string()),
        ("autoOpenLiveStream", "true".to_string()),
        ("MaxStreamingBitrate", "140000000".to_string()),
        ("maxStreamingBitrate", "140000000".to_string()),
    ];
    if let Some(source_id) = media_source_id {
        params.push(("MediaSourceId", source_id.to_string()));
        params.push(("mediaSourceId", source_id.to_string()));
    }
    if let Some(index) = audio_stream_index {
        params.push(("AudioStreamIndex", index.to_string()));
        params.push(("audioStreamIndex", index.to_string()));
    }
    if let Some(index) = subtitle_stream_index {
        if index < 0 {
            params.push(("SubtitleMethod", "None".to_string()));
            params.push(("subtitleMethod", "None".to_string()));
        } else {
            params.push(("SubtitleStreamIndex", index.to_string()));
            params.push(("subtitleStreamIndex", index.to_string()));
        }
    }
    get_json(
        client,
        server,
        &format!("Items/{item_id}/PlaybackInfo"),
        &params,
    )
}

pub(crate) fn item_fields() -> String {
    [
        "PrimaryImageAspectRatio",
        "Overview",
        "Genres",
        "CommunityRating",
        "ProductionYear",
        "RunTimeTicks",
        "ChildCount",
        "ParentBackdropItemId",
        "ParentBackdropImageTags",
        "ParentLogoItemId",
        "ParentLogoImageTag",
        "People",
        "Studios",
        "Tags",
        "OfficialRating",
        "MediaSources",
    ]
    .join(",")
}

pub(crate) fn image_types() -> String {
    "Primary,Backdrop,Thumb,Logo".to_string()
}

pub(crate) fn fetch_server_public_info(
    client: &Client,
    base_url: &str,
) -> Result<PublicSystemInfo, String> {
    let url = build_url(base_url, "System/Info/Public", &[])?;
    let response = client
        .get(url)
        .send()
        .map_err(|err| format!("Server request failed: {err}"))?;
    read_json(response, "System/Info/Public")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_http_server_url() {
        assert_eq!(
            normalize_server_url(" http://127.0.0.1:8096/ ").unwrap(),
            "http://127.0.0.1:8096"
        );
    }

    #[test]
    fn rejects_url_without_scheme() {
        assert!(normalize_server_url("127.0.0.1:8096").is_err());
    }

    #[test]
    fn playback_url_adds_missing_api_key() {
        let server = SavedServer {
            id: "id".to_string(),
            name: "name".to_string(),
            url: "http://127.0.0.1:8096".to_string(),
            username: "user".to_string(),
            user_id: "user-id".to_string(),
            access_token: "token".to_string(),
            active: true,
            saved_at: 0,
            use_system_proxy: true,
        };

        let url = media_playback_url(&server, "/Videos/item/stream?static=true").unwrap();
        assert_eq!(
            url,
            "http://127.0.0.1:8096/Videos/item/stream?static=true&api_key=token"
        );
    }

    #[test]
    fn playback_info_deserializes_server_play_session_id() {
        let raw = r#"{
          "PlaySessionId": "server-session",
          "MediaSources": [
            { "Id": "source-a", "DirectStreamUrl": "/Videos/item/stream?MediaSourceId=source-a" }
          ]
        }"#;

        let info: PlaybackInfo = serde_json::from_str(raw).unwrap();

        assert_eq!(info.play_session_id.as_deref(), Some("server-session"));
        assert_eq!(info.media_sources.len(), 1);
        assert_eq!(info.media_sources[0].id.as_deref(), Some("source-a"));
    }

    #[test]
    fn launch_info_uses_server_session_and_selected_media_source() {
        let server = SavedServer {
            id: "id".to_string(),
            name: "name".to_string(),
            url: "http://127.0.0.1:8096".to_string(),
            username: "user".to_string(),
            user_id: "user-id".to_string(),
            access_token: "token".to_string(),
            active: true,
            saved_at: 0,
            use_system_proxy: true,
        };
        let info = PlaybackInfo {
            play_session_id: Some("server-session".to_string()),
            media_sources: vec![
                MediaSource {
                    id: Some("source-a".to_string()),
                    direct_stream_url: Some(
                        "/Videos/item/stream?MediaSourceId=source-a".to_string(),
                    ),
                    ..Default::default()
                },
                MediaSource {
                    id: Some("source-b".to_string()),
                    direct_stream_url: Some(
                        "/Videos/item/stream?MediaSourceId=source-b".to_string(),
                    ),
                    ..Default::default()
                },
            ],
        };

        let launch = playback_launch_info_from_response(
            &server,
            "item",
            info,
            Some("source-b"),
            None,
            None,
            "fallback-session",
        )
        .unwrap();

        assert_eq!(launch.play_session_id, "server-session");
        assert_eq!(launch.media_source_id.as_deref(), Some("source-b"));
        assert!(launch.stream_url.contains("MediaSourceId=source-b"));
        assert!(launch.stream_url.contains("PlaySessionId=server-session"));
        assert!(launch.stream_url.contains("api_key=token"));
    }

    #[test]
    fn fallback_stream_url_keeps_play_session_and_media_source() {
        let server = SavedServer {
            id: "id".to_string(),
            name: "name".to_string(),
            url: "http://127.0.0.1:8096".to_string(),
            username: "user".to_string(),
            user_id: "user-id".to_string(),
            access_token: "token".to_string(),
            active: true,
            saved_at: 0,
            use_system_proxy: true,
        };

        let url = fallback_stream_url(
            &server,
            "item",
            Some("source-a"),
            Some(1),
            Some(2),
            "server-session",
        )
        .unwrap();

        assert!(url.contains("MediaSourceId=source-a"));
        assert!(url.contains("AudioStreamIndex=1"));
        assert!(url.contains("SubtitleStreamIndex=2"));
        assert!(url.contains("PlaySessionId=server-session"));
        assert!(url.contains("playSessionId=server-session"));
    }

    #[test]
    fn fallback_stream_url_can_disable_subtitles() {
        let server = SavedServer {
            id: "id".to_string(),
            name: "name".to_string(),
            url: "http://127.0.0.1:8096".to_string(),
            username: "user".to_string(),
            user_id: "user-id".to_string(),
            access_token: "token".to_string(),
            active: true,
            saved_at: 0,
            use_system_proxy: true,
        };

        let url = fallback_stream_url(&server, "item", None, None, Some(-1), "session").unwrap();

        assert!(url.contains("SubtitleMethod=None"));
        assert!(url.contains("subtitleMethod=None"));
        assert!(!url.contains("SubtitleStreamIndex=-1"));
    }

    #[test]
    fn primary_images_are_sized_for_poster_lists() {
        let server = SavedServer {
            id: "id".to_string(),
            name: "name".to_string(),
            url: "http://127.0.0.1:8096".to_string(),
            username: "user".to_string(),
            user_id: "user-id".to_string(),
            access_token: "token".to_string(),
            active: true,
            saved_at: 0,
            use_system_proxy: true,
        };
        let mut item = ApiItem::default();
        item.id = "item".to_string();
        item.image_tags
            .insert("Primary".to_string(), "tag".to_string());

        let url = primary_image_url(&server, &item).unwrap();

        assert!(url.contains("fillWidth=360"));
    }
}
