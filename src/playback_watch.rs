use crate::models::{
    ReportPlaybackProgressInput, ReportPlaybackStartInput, ReportPlaybackStoppedInput,
};
use crate::player::{PlayerSession, PlayerState};
use crate::services::PlaybackService;
use std::thread;
use std::time::Duration;

const LOCAL_POLL_INTERVAL: Duration = Duration::from_millis(250);
const SERVER_REPORT_INTERVAL: Duration = Duration::from_secs(10);

pub(crate) struct WatchPlaybackInput {
    pub service: PlaybackService,
    pub player: PlayerSession,
    pub item_id: String,
    pub media_source_id: Option<String>,
    pub play_session_id: String,
    pub play_method: &'static str,
    pub start_position_ticks: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlaybackStopped {
    pub item_id: String,
    pub play_session_id: String,
    pub failed: bool,
    pub completed: bool,
    pub errors: Vec<String>,
}

pub(crate) fn watch(input: WatchPlaybackInput) -> PlaybackStopped {
    let WatchPlaybackInput {
        service,
        player,
        item_id,
        media_source_id,
        play_session_id,
        play_method,
        start_position_ticks,
    } = input;
    let mut last_ticks = positive_ticks(start_position_ticks);
    let mut last_state = PlayerState::default();
    let mut report_elapsed = Duration::ZERO;
    let mut start_reported = false;
    let mut errors = Vec::new();
    let server_id = player.server_id().to_string();
    let audio_stream_index = player.audio_stream_index();
    let subtitle_stream_index = player.subtitle_stream_index();

    loop {
        thread::sleep(LOCAL_POLL_INTERVAL);
        report_elapsed += LOCAL_POLL_INTERVAL;
        let state = match player.state() {
            Ok(state) => state,
            Err(error) => {
                errors.push(format!("Failed to read player state: {error}"));
                last_state.clone()
            }
        };
        if let Some(ticks) = seconds_to_ticks(state.time_pos) {
            last_ticks = Some(ticks);
        }
        last_state = state.clone();

        if !start_reported {
            if let Err(error) = service.report_start(ReportPlaybackStartInput {
                server_id: Some(server_id.clone()),
                item_id: item_id.clone(),
                media_source_id: media_source_id.clone(),
                play_session_id: Some(play_session_id.clone()),
                audio_stream_index,
                subtitle_stream_index,
                position_ticks: last_ticks,
            }) {
                errors.push(format!("Playback start report failed: {error}"));
            }
            start_reported = true;
        } else if report_elapsed >= SERVER_REPORT_INTERVAL {
            report_elapsed = Duration::ZERO;
            if let Err(error) = service.report_progress(progress_input(
                &item_id,
                &media_source_id,
                &play_session_id,
                last_ticks,
                &state,
                play_method,
                &server_id,
                audio_stream_index,
                subtitle_stream_index,
            )) {
                errors.push(format!("Playback progress report failed: {error}"));
            }
        }

        if state.stopped || playback_finished(&state) {
            let final_ticks = last_ticks;
            if let Err(error) = service.report_progress(progress_input(
                &item_id,
                &media_source_id,
                &play_session_id,
                final_ticks,
                &state,
                play_method,
                &server_id,
                audio_stream_index,
                subtitle_stream_index,
            )) {
                errors.push(format!("Playback progress report failed: {error}"));
            }
            if let Err(error) = service.report_stop(ReportPlaybackStoppedInput {
                server_id: Some(server_id.clone()),
                item_id: item_id.clone(),
                media_source_id,
                play_session_id: Some(play_session_id.clone()),
                position_ticks: final_ticks,
                failed: false,
            }) {
                errors.push(format!("Playback stopped report failed: {error}"));
            }
            return PlaybackStopped {
                item_id,
                play_session_id,
                failed: !errors.is_empty(),
                completed: !state.stopped && playback_completed(&state),
                errors,
            };
        }
    }
}

fn progress_input(
    item_id: &str,
    media_source_id: &Option<String>,
    play_session_id: &str,
    position_ticks: Option<i64>,
    state: &PlayerState,
    play_method: &str,
    server_id: &str,
    audio_stream_index: Option<i32>,
    subtitle_stream_index: Option<i32>,
) -> ReportPlaybackProgressInput {
    ReportPlaybackProgressInput {
        server_id: Some(server_id.to_string()),
        item_id: item_id.to_string(),
        media_source_id: media_source_id.clone(),
        play_session_id: Some(play_session_id.to_string()),
        position_ticks,
        is_paused: state.paused,
        is_muted: false,
        audio_stream_index,
        subtitle_stream_index,
        volume_level: state.volume.map(|volume| volume.round() as i32),
        play_method: Some(play_method.to_string()),
    }
}

fn playback_finished(state: &PlayerState) -> bool {
    playback_completed(state)
}

fn playback_completed(state: &PlayerState) -> bool {
    state
        .time_pos
        .zip(state.duration)
        .is_some_and(|(position, duration)| duration > 0.0 && position / duration >= 0.9)
}

fn seconds_to_ticks(seconds: Option<f64>) -> Option<i64> {
    seconds
        .filter(|seconds| *seconds > 0.0)
        .map(|seconds| (seconds * 10_000_000.0).round() as i64)
}

fn positive_ticks(ticks: i64) -> Option<i64> {
    (ticks > 0).then_some(ticks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_positive_seconds_to_server_ticks() {
        assert_eq!(seconds_to_ticks(Some(12.345_678_9)), Some(123_456_789));
        assert_eq!(seconds_to_ticks(Some(-1.0)), None);
        assert_eq!(seconds_to_ticks(None), None);
    }

    #[test]
    fn completion_requires_duration_and_90_percent_progress() {
        let state = PlayerState {
            time_pos: Some(91.0),
            duration: Some(100.0),
            ..Default::default()
        };
        assert!(playback_completed(&state));
        assert!(playback_finished(&state));

        let short = PlayerState {
            time_pos: Some(50.0),
            duration: Some(100.0),
            ..Default::default()
        };
        assert!(!playback_completed(&short));
        assert!(!playback_finished(&short));
    }

    #[test]
    fn missing_duration_is_not_completion() {
        let state = PlayerState {
            time_pos: Some(100.0),
            duration: None,
            ..Default::default()
        };
        assert!(!playback_completed(&state));
    }

    #[test]
    fn stopped_player_is_not_completed() {
        let state = PlayerState {
            stopped: true,
            ..Default::default()
        };

        assert!(!playback_completed(&state));
    }

    #[test]
    fn progress_input_carries_playback_identity_and_tracks() {
        let state = PlayerState {
            paused: true,
            volume: Some(72.6),
            ..Default::default()
        };

        let input = progress_input(
            "item",
            &Some("source".to_string()),
            "session",
            Some(123_456_789),
            &state,
            "Transcoding",
            "server",
            Some(1),
            Some(2),
        );

        assert_eq!(input.server_id.as_deref(), Some("server"));
        assert_eq!(input.item_id, "item");
        assert_eq!(input.media_source_id.as_deref(), Some("source"));
        assert_eq!(input.play_session_id.as_deref(), Some("session"));
        assert_eq!(input.position_ticks, Some(123_456_789));
        assert_eq!(input.audio_stream_index, Some(1));
        assert_eq!(input.subtitle_stream_index, Some(2));
        assert_eq!(input.play_method.as_deref(), Some("Transcoding"));
        assert_eq!(input.volume_level, Some(73));
        assert!(input.is_paused);
    }
}
