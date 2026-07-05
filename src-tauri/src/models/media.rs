use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaLibrary {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) collection_type: Option<String>,
    pub(crate) image_url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) item_type: String,
    pub(crate) year: Option<i64>,
    pub(crate) overview: Option<String>,
    pub(crate) community_rating: Option<f64>,
    pub(crate) run_time_ticks: Option<i64>,
    pub(crate) playback_position_ticks: Option<i64>,
    pub(crate) played_percentage: Option<f64>,
    pub(crate) child_count: Option<i64>,
    pub(crate) season_number: Option<i64>,
    pub(crate) episode_number: Option<i64>,
    pub(crate) series_name: Option<String>,
    pub(crate) series_id: Option<String>,
    pub(crate) season_name: Option<String>,
    pub(crate) season_id: Option<String>,
    pub(crate) genres: Vec<String>,
    pub(crate) official_rating: Option<String>,
    pub(crate) studios: Vec<String>,
    pub(crate) tags: Vec<String>,
    pub(crate) played: bool,
    pub(crate) favorite: bool,
    pub(crate) primary_image_url: Option<String>,
    pub(crate) backdrop_url: Option<String>,
    pub(crate) logo_url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaVersion {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) container: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) protocol: Option<String>,
    pub(crate) bitrate: Option<i64>,
    pub(crate) size: Option<i64>,
    pub(crate) video_codec: Option<String>,
    pub(crate) video_display_title: Option<String>,
    pub(crate) video_range: Option<String>,
    pub(crate) video_profile: Option<String>,
    pub(crate) video_level: Option<f64>,
    pub(crate) aspect_ratio: Option<String>,
    pub(crate) interlaced: Option<bool>,
    pub(crate) bit_depth: Option<i64>,
    pub(crate) pixel_format: Option<String>,
    pub(crate) resolution: Option<String>,
    pub(crate) frame_rate: Option<f64>,
    pub(crate) audio_codec: Option<String>,
    pub(crate) audio_display_title: Option<String>,
    pub(crate) audio_title: Option<String>,
    pub(crate) audio_language: Option<String>,
    pub(crate) channel_layout: Option<String>,
    pub(crate) audio_channels: Option<i64>,
    pub(crate) audio_bitrate: Option<i64>,
    pub(crate) sample_rate: Option<i64>,
    pub(crate) audio_external: Option<bool>,
    pub(crate) audio_default: Option<bool>,
    pub(crate) audio_streams: Vec<StreamInfo>,
    pub(crate) subtitle_count: usize,
    pub(crate) subtitle_languages: Vec<String>,
    pub(crate) subtitle_streams: Vec<StreamInfo>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StreamInfo {
    pub(crate) index: Option<i32>,
    pub(crate) display_title: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) codec: Option<String>,
    pub(crate) channel_layout: Option<String>,
    pub(crate) channels: Option<i64>,
    pub(crate) bitrate: Option<i64>,
    pub(crate) sample_rate: Option<i64>,
    pub(crate) is_external: Option<bool>,
    pub(crate) is_default: Option<bool>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaPerson {
    pub(crate) id: Option<String>,
    pub(crate) name: String,
    pub(crate) role: Option<String>,
    pub(crate) person_type: Option<String>,
    pub(crate) image_url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MediaArt {
    pub(crate) image_type: String,
    pub(crate) url: String,
}
