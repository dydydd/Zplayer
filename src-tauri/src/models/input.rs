use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerIdInput {
    pub(crate) server_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibraryInput {
    pub(crate) library_id: String,
    pub(crate) start_index: Option<usize>,
    pub(crate) limit: Option<usize>,
    pub(crate) item_type: Option<String>,
    pub(crate) sort_by: Option<String>,
    pub(crate) sort_order: Option<String>,
    pub(crate) filters: Option<LibraryFiltersInput>,
}

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct LibraryFiltersInput {
    pub(crate) played: Option<String>,
    pub(crate) favorite: Option<bool>,
    pub(crate) genre: Option<String>,
    pub(crate) person_id: Option<String>,
    pub(crate) collection_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ItemInput {
    pub(crate) item_id: String,
    pub(crate) media_source_id: Option<String>,
    pub(crate) audio_stream_index: Option<i32>,
    pub(crate) subtitle_stream_index: Option<i32>,
    pub(crate) subtitle_stream_position: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlaybackControlInput {
    pub(crate) play_session_id: String,
    pub(crate) command: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlaybackStateInput {
    pub(crate) play_session_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlaybackStateResult {
    pub(crate) time_pos: Option<f64>,
    pub(crate) duration: Option<f64>,
    pub(crate) paused: bool,
    pub(crate) muted: bool,
    pub(crate) volume: Option<i32>,
    #[serde(default)]
    pub(crate) video_ready: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MarkInput {
    pub(crate) item_id: String,
    pub(crate) value: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchInput {
    pub(crate) query: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FetchServerNameInput {
    pub(crate) url: String,
    pub(crate) server_type: String,
    pub(crate) use_system_proxy: bool,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct PublicSystemInfo {
    pub(crate) server_name: Option<String>,
    pub(crate) product_name: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FetchServerNameResult {
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportPlaybackStartInput {
    pub(crate) item_id: String,
    pub(crate) media_source_id: Option<String>,
    pub(crate) play_session_id: Option<String>,
    pub(crate) audio_stream_index: Option<i32>,
    pub(crate) subtitle_stream_index: Option<i32>,
    pub(crate) position_ticks: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportPlaybackProgressInput {
    pub(crate) item_id: String,
    pub(crate) media_source_id: Option<String>,
    pub(crate) play_session_id: Option<String>,
    pub(crate) position_ticks: Option<i64>,
    pub(crate) is_paused: bool,
    pub(crate) is_muted: bool,
    pub(crate) audio_stream_index: Option<i32>,
    pub(crate) subtitle_stream_index: Option<i32>,
    pub(crate) volume_level: Option<i32>,
    pub(crate) play_method: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReportPlaybackStoppedInput {
    pub(crate) item_id: String,
    pub(crate) media_source_id: Option<String>,
    pub(crate) play_session_id: Option<String>,
    pub(crate) position_ticks: Option<i64>,
    pub(crate) failed: bool,
}
