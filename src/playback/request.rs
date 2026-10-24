use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PlaybackPlayerChoice {
    #[default]
    ConfiguredDefault,
    Internal,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OnlinePlaybackRequest {
    pub(crate) server_id: String,
    pub(crate) item_id: String,
    #[serde(default)]
    pub(crate) media_source_id: Option<String>,
    #[serde(default)]
    pub(crate) audio_stream_index: Option<i32>,
    #[serde(default)]
    pub(crate) subtitle_stream_index: Option<i32>,
    #[serde(default)]
    pub(crate) start_position_ticks: i64,
    #[serde(default)]
    pub(crate) player_choice: PlaybackPlayerChoice,
}
