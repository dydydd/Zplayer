use super::images::{backdrop_url, logo_url, media_image_url, primary_image_url};
use crate::models::{
    ApiItem, MediaItem, MediaPerson, MediaStream, SavedServer, SearchHint, StreamInfo,
};

pub(crate) fn stream_info(stream: &MediaStream) -> StreamInfo {
    StreamInfo {
        index: stream.index,
        display_title: stream.display_title.clone(),
        title: stream.title.clone(),
        language: stream.language.clone(),
        codec: stream.codec.clone(),
        channel_layout: stream.channel_layout.clone(),
        channels: stream.channels,
        bitrate: stream.bitrate,
        sample_rate: stream.sample_rate,
        is_external: stream.is_external,
        is_default: stream.is_default,
    }
}

pub(crate) fn map_item(server: &SavedServer, item: ApiItem) -> MediaItem {
    let primary_image_url = primary_image_url(server, &item);
    let backdrop_url = backdrop_url(server, &item);
    let logo_url = logo_url(server, &item);
    let playback_position_ticks = item
        .user_data
        .as_ref()
        .and_then(|data| data.playback_position_ticks);
    let played_percentage = item
        .user_data
        .as_ref()
        .and_then(|data| data.played_percentage);
    MediaItem {
        id: item.id.clone(),
        name: item.name.unwrap_or_else(|| "Untitled".to_string()),
        item_type: item.item_type.unwrap_or_else(|| "Item".to_string()),
        year: item.production_year,
        overview: item.overview,
        community_rating: item.community_rating,
        run_time_ticks: item.run_time_ticks,
        playback_position_ticks,
        played_percentage,
        child_count: item.child_count,
        season_number: item.parent_index_number,
        episode_number: item.index_number,
        series_name: item.series_name,
        series_id: item.series_id,
        season_name: item.season_name,
        season_id: item.season_id,
        genres: item.genres,
        official_rating: item.official_rating,
        studios: item
            .studios
            .into_iter()
            .filter_map(|studio| studio.name)
            .collect(),
        tags: item.tags,
        played: item
            .user_data
            .as_ref()
            .map(|data| data.played)
            .unwrap_or(false),
        favorite: item
            .user_data
            .as_ref()
            .map(|data| data.is_favorite)
            .unwrap_or(false),
        primary_image_url,
        backdrop_url,
        logo_url,
    }
}

pub(crate) fn map_search_hint(server: &SavedServer, hint: SearchHint) -> Option<MediaItem> {
    let id = hint.item_id.or(hint.id)?;
    let primary_image_url = if hint.primary_image_tag.is_some() {
        media_image_url(server, &id, "Primary", &[("fillWidth", "360")]).ok()
    } else {
        None
    };
    let backdrop_item_id = hint.backdrop_image_item_id.as_deref().unwrap_or(&id);
    let backdrop_url = if hint.backdrop_image_tag.is_some() {
        media_image_url(
            server,
            backdrop_item_id,
            "Backdrop",
            &[("fillWidth", "1600")],
        )
        .ok()
    } else if hint.thumb_image_tag.is_some() {
        let thumb_item_id = hint.thumb_image_item_id.as_deref().unwrap_or(&id);
        media_image_url(server, thumb_item_id, "Thumb", &[("fillWidth", "1600")]).ok()
    } else {
        None
    };

    Some(MediaItem {
        id,
        name: hint
            .name
            .or(hint.matched_term)
            .unwrap_or_else(|| "Untitled".to_string()),
        item_type: hint.item_type.unwrap_or_else(|| "Item".to_string()),
        year: hint.production_year,
        overview: None,
        community_rating: None,
        run_time_ticks: hint.run_time_ticks,
        playback_position_ticks: None,
        played_percentage: None,
        child_count: None,
        season_number: hint.parent_index_number,
        episode_number: hint.index_number,
        series_name: hint.series,
        series_id: hint.series_id,
        season_name: None,
        season_id: None,
        genres: Vec::new(),
        official_rating: None,
        studios: Vec::new(),
        tags: Vec::new(),
        played: false,
        favorite: false,
        primary_image_url,
        backdrop_url,
        logo_url: None,
    })
}

pub(crate) fn people(server: &SavedServer, item: &ApiItem) -> Vec<MediaPerson> {
    item.people
        .iter()
        .filter_map(|person| {
            let name = person.name.clone()?;
            let image_url = person
                .id
                .as_deref()
                .filter(|_| {
                    person.primary_image_tag.is_some() || person.image_tags.contains_key("Primary")
                })
                .and_then(|id| {
                    media_image_url(server, id, "Primary", &[("fillWidth", "360")]).ok()
                });
            Some(MediaPerson {
                id: person.id.clone(),
                name,
                role: person.role.clone(),
                person_type: person.person_type.clone(),
                image_url,
            })
        })
        .collect()
}

pub(crate) fn first_playable_child(children: Vec<ApiItem>) -> Option<ApiItem> {
    children.into_iter().find(|child| {
        child
            .item_type
            .as_deref()
            .map(is_playable_item_type)
            .unwrap_or(false)
    })
}

pub(crate) fn is_playable_item_type(item_type: &str) -> bool {
    matches!(item_type, "Movie" | "Episode" | "Video")
}
