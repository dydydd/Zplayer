use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoginInput {
    pub(crate) server_type: String,
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) use_system_proxy: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveServerInput {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) username: String,
    pub(crate) user_id: String,
    pub(crate) access_token: String,
    #[serde(default = "default_true")]
    pub(crate) use_system_proxy: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedServer {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) username: String,
    pub(crate) user_id: String,
    pub(crate) access_token: String,
    pub(crate) active: bool,
    pub(crate) saved_at: u64,
    #[serde(default = "default_true")]
    pub(crate) use_system_proxy: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedServerSummary {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) username: String,
    pub(crate) active: bool,
    pub(crate) use_system_proxy: bool,
    pub(crate) movie_count: Option<i64>,
    pub(crate) series_count: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "PascalCase")]
pub(crate) struct ItemCounts {
    pub(crate) movie_count: Option<i64>,
    pub(crate) series_count: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerStore {
    pub(crate) active_server_id: Option<String>,
    pub(crate) servers: Vec<SavedServer>,
    #[serde(default)]
    pub(crate) settings: AppSettings,
    #[serde(default)]
    pub(crate) recent_plays: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub(crate) playback_preferences: HashMap<String, HashMap<String, PlaybackPreference>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerExport {
    pub(crate) version: u32,
    pub(crate) exported_at: u64,
    pub(crate) servers: Vec<SavedServer>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServerImportResult {
    pub(crate) imported: usize,
    pub(crate) added: usize,
    pub(crate) updated: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct PlaybackPreference {
    pub(crate) media_source_id: Option<String>,
    pub(crate) audio_stream_index: Option<i32>,
    pub(crate) audio_language: Option<String>,
    pub(crate) subtitle_stream_index: Option<i32>,
    pub(crate) subtitle_language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppSettings {
    pub(crate) mpv_path: Option<String>,
    #[serde(default = "default_volume")]
    pub(crate) default_volume: i32,
    #[serde(default = "default_seek_back_seconds")]
    pub(crate) seek_back_seconds: i32,
    #[serde(default = "default_seek_forward_seconds")]
    pub(crate) seek_forward_seconds: i32,
    #[serde(default = "default_subtitle_mode")]
    pub(crate) subtitle_mode: String,
    #[serde(default = "default_poster_density")]
    pub(crate) poster_density: String,
    #[serde(default = "default_true")]
    pub(crate) metadata_cache_enabled: bool,
    #[serde(default = "default_theme")]
    pub(crate) theme: String,
    #[serde(default)]
    pub(crate) diagnostics_enabled: bool,
    #[serde(default = "default_true")]
    pub(crate) autoplay_next_episode: bool,
    #[serde(default = "default_language")]
    pub(crate) language: String,
    #[serde(default, alias = "tmdbAccessToken")]
    pub(crate) tmdb_api_key: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            mpv_path: None,
            default_volume: default_volume(),
            seek_back_seconds: default_seek_back_seconds(),
            seek_forward_seconds: default_seek_forward_seconds(),
            subtitle_mode: default_subtitle_mode(),
            poster_density: default_poster_density(),
            metadata_cache_enabled: default_true(),
            theme: default_theme(),
            diagnostics_enabled: false,
            autoplay_next_episode: default_true(),
            language: default_language(),
            tmdb_api_key: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveSettingsInput {
    pub(crate) mpv_path: Option<String>,
    pub(crate) default_volume: Option<i32>,
    pub(crate) seek_back_seconds: Option<i32>,
    pub(crate) seek_forward_seconds: Option<i32>,
    pub(crate) subtitle_mode: Option<String>,
    pub(crate) poster_density: Option<String>,
    pub(crate) metadata_cache_enabled: Option<bool>,
    pub(crate) theme: Option<String>,
    pub(crate) diagnostics_enabled: Option<bool>,
    pub(crate) autoplay_next_episode: Option<bool>,
    pub(crate) language: Option<String>,
    #[serde(alias = "tmdbAccessToken")]
    pub(crate) tmdb_api_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoginResult {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) username: String,
    pub(crate) user_id: String,
    pub(crate) access_token: String,
    pub(crate) use_system_proxy: bool,
}

fn default_true() -> bool {
    true
}

fn default_volume() -> i32 {
    100
}

fn default_seek_back_seconds() -> i32 {
    10
}

fn default_seek_forward_seconds() -> i32 {
    30
}

fn default_subtitle_mode() -> String {
    "auto".to_string()
}

fn default_poster_density() -> String {
    "comfortable".to_string()
}

fn default_theme() -> String {
    "dark".to_string()
}

fn default_language() -> String {
    "auto".to_string()
}

pub(crate) fn normalize_settings(input: SaveSettingsInput) -> AppSettings {
    AppSettings {
        mpv_path: input
            .mpv_path
            .map(|path| path.trim().to_string())
            .filter(|path| !path.is_empty()),
        default_volume: input
            .default_volume
            .unwrap_or_else(default_volume)
            .clamp(0, 100),
        seek_back_seconds: input
            .seek_back_seconds
            .unwrap_or_else(default_seek_back_seconds)
            .clamp(5, 60),
        seek_forward_seconds: input
            .seek_forward_seconds
            .unwrap_or_else(default_seek_forward_seconds)
            .clamp(5, 180),
        subtitle_mode: match input.subtitle_mode.as_deref() {
            Some("off") => "off".to_string(),
            _ => default_subtitle_mode(),
        },
        poster_density: match input.poster_density.as_deref() {
            Some("compact") => "compact".to_string(),
            _ => default_poster_density(),
        },
        metadata_cache_enabled: input.metadata_cache_enabled.unwrap_or_else(default_true),
        theme: match input.theme.as_deref() {
            Some("midnight") => "midnight".to_string(),
            _ => default_theme(),
        },
        diagnostics_enabled: input.diagnostics_enabled.unwrap_or(false),
        autoplay_next_episode: input.autoplay_next_episode.unwrap_or_else(default_true),
        language: match input.language.as_deref() {
            Some("zh-CN") => "zh-CN".to_string(),
            Some("en-US") => "en-US".to_string(),
            _ => default_language(),
        },
        tmdb_api_key: input
            .tmdb_api_key
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty()),
    }
}
