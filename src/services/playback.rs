use super::AppPaths;
use crate::api;
use crate::models::{
    ApiItem, ItemInput, PlayResult, PlaybackControlInput, PlaybackPreference, PlaybackStateInput,
    PlaybackStateResult, ReportPlaybackProgressInput, ReportPlaybackStartInput,
    ReportPlaybackStoppedInput, SavePlaybackPreferenceInput, SavedServer,
};
use crate::playback::{
    MediaSourceCandidate, NetworkOptions, OnlinePlaybackRequest, PlaybackPlayerChoice,
    choose_media_source_index, resolve_playback_session,
};
use crate::player::{PlayerSession, PlayerState};
use crate::storage::{AppStore, playback_preference_key};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub(crate) struct PlaybackService {
    store: AppStore,
}

impl PlaybackService {
    pub(crate) fn new(paths: &AppPaths) -> Result<Self, String> {
        Ok(Self {
            store: paths.store()?,
        })
    }

    pub(crate) fn resolve(
        &self,
        input: ItemInput,
    ) -> Result<crate::playback::ResolvedPlaybackSession, String> {
        let server = self.server_for(input.server_id.as_deref())?;
        let client = api::http_client_for_server(&server, Duration::from_secs(20))?;
        let item = api::resolve_playable_item(&client, &server, &input.item_id)?;
        self.store.remember_recent_play(&server.id, &item.id)?;
        let preference = self.preference_for(&server.id, &item)?;
        let selection = playback_selection(input, preference);
        let start_position_ticks = item
            .user_data
            .as_ref()
            .and_then(|data| data.playback_position_ticks)
            .filter(|ticks| *ticks > 0)
            .unwrap_or_default();
        let fallback_session_id = play_session_id(&item.id);
        let negotiation = api::negotiate_playback(
            &client,
            &server,
            &item.id,
            selection.media_source_id.as_deref(),
            selection.audio_stream_index,
            selection.subtitle_stream_index,
        )?;
        let sources = negotiation.sources;
        let media_sources = negotiation.media_sources;
        let server_session_id = negotiation.play_session_id.unwrap_or(fallback_session_id);
        let selected_source =
            choose_media_source_index(&sources, selection.media_source_id.as_deref())
                .and_then(|index| media_sources.get(index));
        let subtitle_url = selected_source
            .map(|source| {
                api::selected_subtitle_url(
                    &server,
                    &item.id,
                    source,
                    selection.subtitle_stream_index,
                )
            })
            .transpose()?
            .flatten();
        let request = OnlinePlaybackRequest {
            server_id: server.id,
            item_id: item.id,
            media_source_id: selection.media_source_id,
            audio_stream_index: selection.audio_stream_index,
            subtitle_stream_index: selection.subtitle_stream_index,
            start_position_ticks,
            player_choice: PlaybackPlayerChoice::ConfiguredDefault,
        };
        resolve_playback_session(
            request,
            &server.url,
            &sources,
            Some(&server_session_id),
            subtitle_url.as_deref(),
            NetworkOptions::default(),
        )
        .map_err(|err| err.to_string())
    }

    pub(crate) fn save_preferences(
        &self,
        input: SavePlaybackPreferenceInput,
    ) -> Result<(), String> {
        let server = match input.server_id.as_deref() {
            Some(server_id) if !server_id.trim().is_empty() => {
                self.store.server_by_id(server_id)?
            }
            _ => self.store.active_server()?,
        };
        let key = playback_preference_key(input.series_id.as_deref(), &input.item_id);
        self.store.save_playback_preference(
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
    }

    pub(crate) fn preferences(
        &self,
    ) -> Result<std::collections::HashMap<String, PlaybackPreference>, String> {
        let server = self.store.active_server()?;
        self.store.playback_preferences(&server.id)
    }

    pub(crate) fn control(
        &self,
        player: &PlayerSession,
        input: PlaybackControlInput,
    ) -> Result<PlaybackStateResult, String> {
        match parse_playback_command(&input.command)? {
            PlaybackCommand::Play => player.play(),
            PlaybackCommand::Pause => player.pause(),
            PlaybackCommand::TogglePause => player.toggle_pause(),
            PlaybackCommand::Stop => player.stop(),
            PlaybackCommand::Seek(seconds) => player.seek(seconds),
            PlaybackCommand::Volume(volume) => player.set_volume(volume),
        }?;
        Ok(playback_state_result(&player.state()?))
    }

    pub(crate) fn state(
        &self,
        player: &PlayerSession,
        _input: PlaybackStateInput,
    ) -> Result<PlaybackStateResult, String> {
        Ok(playback_state_result(&player.state()?))
    }

    pub(crate) fn report_start(&self, input: ReportPlaybackStartInput) -> Result<(), String> {
        let server = self.server_for(input.server_id.as_deref())?;
        let client = api::http_client_for_server(&server, Duration::from_secs(20))?;
        api::report_playback_start(&client, &server, &input)
    }

    pub(crate) fn report_progress(&self, input: ReportPlaybackProgressInput) -> Result<(), String> {
        let server = self.server_for(input.server_id.as_deref())?;
        let client = api::http_client_for_server(&server, Duration::from_secs(20))?;
        api::report_playback_progress(&client, &server, &input)
    }

    pub(crate) fn report_stop(&self, input: ReportPlaybackStoppedInput) -> Result<(), String> {
        let server = self.server_for(input.server_id.as_deref())?;
        let client = api::http_client_for_server(&server, Duration::from_secs(20))?;
        api::report_playback_stopped(&client, &server, &input)
    }

    fn server_for(&self, server_id: Option<&str>) -> Result<SavedServer, String> {
        match server_id.filter(|value| !value.trim().is_empty()) {
            Some(server_id) => self.store.server_by_id(server_id),
            None => self.store.active_server(),
        }
    }

    fn preference_for(
        &self,
        server_id: &str,
        item: &ApiItem,
    ) -> Result<Option<PlaybackPreference>, String> {
        let key = playback_preference_key(
            item.series_id.as_deref().filter(|value| !value.is_empty()),
            &item.id,
        );
        self.store.playback_preference(server_id, &key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlaybackSelection {
    media_source_id: Option<String>,
    audio_stream_index: Option<i32>,
    subtitle_stream_index: Option<i32>,
}

fn playback_selection(
    input: ItemInput,
    preference: Option<PlaybackPreference>,
) -> PlaybackSelection {
    let (preferred_source, preferred_audio, preferred_subtitle) = match preference {
        Some(preference) => (
            preference.media_source_id,
            preference.audio_stream_index,
            preference.subtitle_stream_index,
        ),
        None => (None, None, None),
    };
    PlaybackSelection {
        media_source_id: input.media_source_id.or(preferred_source),
        audio_stream_index: input.audio_stream_index.or(preferred_audio),
        subtitle_stream_index: input.subtitle_stream_index.or(preferred_subtitle),
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PlaybackCommand {
    Play,
    Pause,
    TogglePause,
    Stop,
    Seek(f64),
    Volume(i32),
}

fn parse_playback_command(command: &str) -> Result<PlaybackCommand, String> {
    let command = command.trim();
    match command {
        "play" => Ok(PlaybackCommand::Play),
        "pause" => Ok(PlaybackCommand::Pause),
        "toggle" | "toggle-pause" | "toggle_pause" => Ok(PlaybackCommand::TogglePause),
        "stop" => Ok(PlaybackCommand::Stop),
        _ => {
            if let Some(value) = command.strip_prefix("seek:") {
                return value
                    .trim()
                    .parse::<f64>()
                    .map(PlaybackCommand::Seek)
                    .map_err(|_| format!("Invalid seek value: {value}"));
            }
            if let Some(value) = command.strip_prefix("volume:") {
                return value
                    .trim()
                    .parse::<i32>()
                    .map(PlaybackCommand::Volume)
                    .map_err(|_| format!("Invalid volume value: {value}"));
            }
            Err(format!("Unknown playback command: {command}"))
        }
    }
}

fn playback_state_result(state: &PlayerState) -> PlaybackStateResult {
    PlaybackStateResult {
        time_pos: state.time_pos,
        duration: state.duration,
        paused: state.paused,
        muted: false,
        volume: state.volume.map(|volume| volume.round() as i32),
        speed: None,
        cache_speed: None,
        video_ready: state.time_pos.is_some(),
    }
}

fn play_session_id(item_id: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("zplayer-{item_id}-{stamp}")
}

#[allow(dead_code)]
fn play_result_from_session(
    session: &crate::playback::ResolvedPlaybackSession,
    server_name: &str,
) -> PlayResult {
    PlayResult {
        item_id: String::new(),
        server_id: String::new(),
        server_name: server_name.to_string(),
        media_source_id: session.media_source_id.clone(),
        play_session_id: session.play_session_id.clone().unwrap_or_default(),
        url: session.url.clone(),
        log_path: String::new(),
        log_tail: String::new(),
    }
}

#[allow(dead_code)]
fn source_count(sources: &[MediaSourceCandidate]) -> usize {
    sources.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{PlaybackPreference, SavedServer};

    fn item(series_id: Option<&str>) -> ApiItem {
        ApiItem {
            id: "episode-1".into(),
            series_id: series_id.map(str::to_string),
            ..Default::default()
        }
    }

    fn server() -> SavedServer {
        SavedServer {
            id: "server-1".into(),
            name: "Server".into(),
            url: "http://127.0.0.1:8096".into(),
            username: "user".into(),
            user_id: "user-id".into(),
            access_token: "token".into(),
            active: true,
            saved_at: 0,
            use_system_proxy: true,
            proxy_url: None,
            icon_url: None,
            icon_name: None,
        }
    }

    fn preference() -> PlaybackPreference {
        PlaybackPreference {
            media_source_id: Some("source-a".into()),
            audio_stream_index: Some(1),
            audio_language: Some("zh".into()),
            subtitle_stream_index: Some(2),
            subtitle_language: Some("zh-CN".into()),
        }
    }

    #[test]
    fn playback_selection_prefers_explicit_input_over_preference() {
        let input = ItemInput {
            item_id: "episode-1".into(),
            server_id: None,
            media_source_id: Some("explicit".into()),
            audio_stream_index: Some(3),
            subtitle_stream_index: Some(-1),
            subtitle_stream_position: None,
        };

        let selection = playback_selection(input, Some(preference()));

        assert_eq!(selection.media_source_id.as_deref(), Some("explicit"));
        assert_eq!(selection.audio_stream_index, Some(3));
        assert_eq!(selection.subtitle_stream_index, Some(-1));
    }

    #[test]
    fn playback_selection_falls_back_to_preference() {
        let input = ItemInput {
            item_id: "episode-1".into(),
            server_id: None,
            media_source_id: None,
            audio_stream_index: None,
            subtitle_stream_index: None,
            subtitle_stream_position: None,
        };

        let selection = playback_selection(input, Some(preference()));

        assert_eq!(selection.media_source_id.as_deref(), Some("source-a"));
        assert_eq!(selection.audio_stream_index, Some(1));
        assert_eq!(selection.subtitle_stream_index, Some(2));
    }

    #[test]
    fn parses_playback_control_commands() {
        assert_eq!(parse_playback_command(" play "), Ok(PlaybackCommand::Play));
        assert_eq!(parse_playback_command("pause"), Ok(PlaybackCommand::Pause));
        assert_eq!(
            parse_playback_command("toggle-pause"),
            Ok(PlaybackCommand::TogglePause)
        );
        assert_eq!(parse_playback_command("stop"), Ok(PlaybackCommand::Stop));
        assert_eq!(
            parse_playback_command("seek:12.5"),
            Ok(PlaybackCommand::Seek(12.5))
        );
        assert_eq!(
            parse_playback_command("volume:80"),
            Ok(PlaybackCommand::Volume(80))
        );
        assert!(parse_playback_command("seek:bad").is_err());
        assert!(parse_playback_command("unknown").is_err());
    }

    #[test]
    fn preference_lookup_uses_series_key_for_episodes() {
        let dir = std::env::temp_dir().join(format!(
            "zplayer-playback-service-test-{}",
            crate::storage::unix_now()
        ));
        let store = AppStore::new(&dir).unwrap();
        store.save_server(server()).unwrap();
        store
            .save_playback_preference("server-1", "series:series-1".into(), preference())
            .unwrap();
        let service = PlaybackService {
            store: store.clone(),
        };

        let found = service
            .preference_for("server-1", &item(Some("series-1")))
            .unwrap();

        assert_eq!(
            found.as_ref().unwrap().media_source_id.as_deref(),
            Some("source-a")
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn preference_lookup_uses_item_key_when_no_series_id() {
        let dir = std::env::temp_dir().join(format!(
            "zplayer-playback-service-item-test-{}",
            crate::storage::unix_now()
        ));
        let store = AppStore::new(&dir).unwrap();
        store.save_server(server()).unwrap();
        store
            .save_playback_preference("server-1", "item:episode-1".into(), preference())
            .unwrap();
        let service = PlaybackService {
            store: store.clone(),
        };

        let found = service.preference_for("server-1", &item(None)).unwrap();

        assert!(found.is_some());
        let _ = std::fs::remove_dir_all(dir);
    }
}
