use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct AuthResponse {
    pub(crate) access_token: String,
    pub(crate) user: AuthUser,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct AuthUser {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ItemsResponse {
    Object {
        #[serde(default, rename = "Items")]
        items: Vec<ApiItem>,
        #[serde(default, rename = "TotalRecordCount")]
        total_record_count: Option<usize>,
    },
    Array(Vec<ApiItem>),
}

impl ItemsResponse {
    pub(crate) fn total_record_count(&self) -> Option<usize> {
        match self {
            Self::Object {
                total_record_count, ..
            } => *total_record_count,
            Self::Array(items) => Some(items.len()),
        }
    }

    pub(crate) fn into_items(self) -> Vec<ApiItem> {
        match self {
            Self::Object { items, .. } => items,
            Self::Array(items) => items,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum RecommendationsResponse {
    Groups(Vec<RecommendationGroup>),
    Items(ItemsResponse),
}

impl RecommendationsResponse {
    pub(crate) fn into_items(self) -> Vec<ApiItem> {
        match self {
            Self::Groups(groups) => groups.into_iter().flat_map(|group| group.items).collect(),
            Self::Items(items) => items.into_items(),
        }
    }
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct RecommendationGroup {
    pub(crate) items: Vec<ApiItem>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct SearchHintResult {
    pub(crate) search_hints: Vec<SearchHint>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct SearchHint {
    pub(crate) item_id: Option<String>,
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) matched_term: Option<String>,
    #[serde(rename = "Type")]
    pub(crate) item_type: Option<String>,
    pub(crate) production_year: Option<i64>,
    pub(crate) run_time_ticks: Option<i64>,
    pub(crate) index_number: Option<i64>,
    pub(crate) parent_index_number: Option<i64>,
    pub(crate) series: Option<String>,
    pub(crate) series_id: Option<String>,
    pub(crate) primary_image_tag: Option<String>,
    pub(crate) backdrop_image_tag: Option<String>,
    pub(crate) backdrop_image_item_id: Option<String>,
    pub(crate) thumb_image_tag: Option<String>,
    pub(crate) thumb_image_item_id: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct ApiItem {
    pub(crate) id: String,
    pub(crate) name: Option<String>,
    #[serde(rename = "Type")]
    pub(crate) item_type: Option<String>,
    pub(crate) collection_type: Option<String>,
    pub(crate) overview: Option<String>,
    pub(crate) production_year: Option<i64>,
    pub(crate) community_rating: Option<f64>,
    pub(crate) run_time_ticks: Option<i64>,
    pub(crate) child_count: Option<i64>,
    pub(crate) index_number: Option<i64>,
    pub(crate) parent_index_number: Option<i64>,
    pub(crate) series_name: Option<String>,
    pub(crate) series_id: Option<String>,
    pub(crate) season_name: Option<String>,
    pub(crate) season_id: Option<String>,
    pub(crate) image_tags: HashMap<String, String>,
    pub(crate) backdrop_image_tags: Vec<String>,
    pub(crate) parent_backdrop_item_id: Option<String>,
    pub(crate) parent_backdrop_image_tags: Vec<String>,
    pub(crate) parent_logo_item_id: Option<String>,
    pub(crate) parent_logo_image_tag: Option<String>,
    pub(crate) genres: Vec<String>,
    pub(crate) official_rating: Option<String>,
    pub(crate) studios: Vec<NameItem>,
    pub(crate) tags: Vec<String>,
    pub(crate) user_data: Option<UserData>,
    pub(crate) media_sources: Vec<MediaSource>,
    pub(crate) people: Vec<ApiPerson>,
    pub(crate) screenshot_image_tags: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct PlaybackInfo {
    pub(crate) media_sources: Vec<MediaSource>,
    pub(crate) play_session_id: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct MediaSource {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) container: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) protocol: Option<String>,
    pub(crate) bitrate: Option<i64>,
    pub(crate) size: Option<i64>,
    pub(crate) media_streams: Vec<MediaStream>,
    pub(crate) direct_stream_url: Option<String>,
    pub(crate) transcoding_url: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct MediaStream {
    pub(crate) index: Option<i32>,
    #[serde(rename = "Type")]
    pub(crate) stream_type: Option<String>,
    pub(crate) display_title: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) codec: Option<String>,
    pub(crate) width: Option<i64>,
    pub(crate) height: Option<i64>,
    pub(crate) real_frame_rate: Option<f64>,
    pub(crate) bitrate: Option<i64>,
    pub(crate) video_range: Option<String>,
    pub(crate) profile: Option<String>,
    pub(crate) level: Option<f64>,
    pub(crate) aspect_ratio: Option<String>,
    pub(crate) is_interlaced: Option<bool>,
    pub(crate) bit_depth: Option<i64>,
    pub(crate) pixel_format: Option<String>,
    pub(crate) channel_layout: Option<String>,
    pub(crate) channels: Option<i64>,
    pub(crate) sample_rate: Option<i64>,
    pub(crate) is_external: Option<bool>,
    pub(crate) is_default: Option<bool>,
    pub(crate) delivery_method: Option<String>,
    pub(crate) delivery_url: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct UserData {
    pub(crate) played: bool,
    pub(crate) is_favorite: bool,
    pub(crate) playback_position_ticks: Option<i64>,
    pub(crate) played_percentage: Option<f64>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct NameItem {
    pub(crate) name: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct ApiPerson {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) role: Option<String>,
    #[serde(rename = "Type")]
    pub(crate) person_type: Option<String>,
    pub(crate) primary_image_tag: Option<String>,
    pub(crate) image_tags: HashMap<String, String>,
}
