use crate::models::{
    MediaArt, MediaItem, MediaLibrary, MediaPerson, MediaVersion, SavedServerSummary,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HomePayload {
    pub(crate) server: SavedServerSummary,
    pub(crate) libraries: Vec<MediaLibrary>,
    pub(crate) library_latest: Vec<LibraryLatestPayload>,
    pub(crate) latest: Vec<MediaItem>,
    pub(crate) recommended_movies: Vec<MediaItem>,
    pub(crate) recommended_shows: Vec<MediaItem>,
    pub(crate) resume_items: Vec<MediaItem>,
    pub(crate) favorite_items: Vec<MediaItem>,
    pub(crate) recent_items: Vec<MediaItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HomeMorePayload {
    pub(crate) server_id: String,
    pub(crate) library_latest: Vec<LibraryLatestPayload>,
    pub(crate) recommended_movies: Vec<MediaItem>,
    pub(crate) recommended_shows: Vec<MediaItem>,
    pub(crate) resume_items: Vec<MediaItem>,
    pub(crate) favorite_items: Vec<MediaItem>,
    pub(crate) recent_items: Vec<MediaItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchPayload {
    pub(crate) items: Vec<MediaItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibraryPayload {
    pub(crate) library: MediaLibrary,
    pub(crate) items: Vec<MediaItem>,
    pub(crate) total_count: usize,
    pub(crate) start_index: usize,
    pub(crate) limit: usize,
    pub(crate) has_more: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LibraryLatestPayload {
    pub(crate) library: MediaLibrary,
    pub(crate) items: Vec<MediaItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ItemDetailPayload {
    pub(crate) item: MediaItem,
    pub(crate) children: Vec<MediaItem>,
    pub(crate) seasons: Vec<MediaItem>,
    pub(crate) episodes: Vec<MediaItem>,
    pub(crate) media_sources: Vec<MediaVersion>,
    pub(crate) people: Vec<MediaPerson>,
    pub(crate) art: Vec<MediaArt>,
    pub(crate) similar: Vec<MediaItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ItemMorePayload {
    pub(crate) item_id: String,
    pub(crate) people: Vec<MediaPerson>,
    pub(crate) art: Vec<MediaArt>,
    pub(crate) similar: Vec<MediaItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PlayResult {
    pub(crate) item_id: String,
    pub(crate) server_id: String,
    pub(crate) server_name: String,
    pub(crate) media_source_id: Option<String>,
    pub(crate) play_session_id: String,
    pub(crate) url: String,
    pub(crate) log_path: String,
    pub(crate) log_tail: String,
}
