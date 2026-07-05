use crate::api;
use crate::models::{
    PlaybackStateResult, ReportPlaybackProgressInput, ReportPlaybackStartInput,
    ReportPlaybackStoppedInput, SavedServer,
};
use crate::mpv;
use serde::Serialize;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const LOCAL_PROGRESS_POLL_INTERVAL: Duration = Duration::from_millis(250);
const SERVER_PROGRESS_REPORT_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackStoppedEvent {
    item_id: String,
    play_session_id: String,
    failed: bool,
}

pub(crate) fn watch_mpv_playback(
    app: AppHandle,
    server: SavedServer,
    mut child: std::process::Child,
    progress_path: std::path::PathBuf,
    item_id: String,
    media_source_id: Option<String>,
    play_session_id: String,
    audio_stream_index: Option<i32>,
    subtitle_stream_index: Option<i32>,
    start_position_ticks: Option<i64>,
) {
    let client = api::http_client(server.use_system_proxy).ok();
    let mut last_ticks = None;
    let mut report_elapsed = Duration::ZERO;
    let mut start_reported = false;
    loop {
        thread::sleep(LOCAL_PROGRESS_POLL_INTERVAL);
        report_elapsed += LOCAL_PROGRESS_POLL_INTERVAL;
        let progress = read_mpv_progress(&progress_path);
        if let Some(progress) = progress.as_ref() {
            mpv::cache_state(&play_session_id, progress.clone());
            last_ticks = progress
                .time_pos
                .map(seconds_to_ticks)
                .filter(|ticks| *ticks > 0)
                .or(last_ticks);
            if !start_reported {
                if let Some(client) = client.clone() {
                    let input = ReportPlaybackStartInput {
                        item_id: item_id.clone(),
                        media_source_id: media_source_id.clone(),
                        play_session_id: Some(play_session_id.clone()),
                        audio_stream_index,
                        subtitle_stream_index,
                        position_ticks: last_ticks.or(start_position_ticks),
                    };
                    let _ = api::report_playback_start(&client, &server, &input);
                    start_reported = true;
                }
            } else if report_elapsed >= SERVER_PROGRESS_REPORT_INTERVAL {
                report_elapsed = Duration::ZERO;
                if let Some(client) = client.clone() {
                    let server = server.clone();
                    let input = ReportPlaybackProgressInput {
                        item_id: item_id.clone(),
                        media_source_id: media_source_id.clone(),
                        play_session_id: Some(play_session_id.clone()),
                        position_ticks: last_ticks,
                        is_paused: progress.paused,
                        is_muted: progress.muted,
                        audio_stream_index: None,
                        subtitle_stream_index: None,
                        volume_level: progress.volume,
                        play_method: Some("DirectStream".to_string()),
                    };
                    thread::spawn(move || {
                        let _ = api::report_playback_progress(&client, &server, &input);
                    });
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let failed = !status.success();
                if let Some(client) = client.clone() {
                    let final_ticks =
                        final_position_ticks(progress.as_ref(), last_ticks, start_position_ticks);
                    if let Some(progress) = progress.as_ref() {
                        let input = ReportPlaybackProgressInput {
                            item_id: item_id.clone(),
                            media_source_id: media_source_id.clone(),
                            play_session_id: Some(play_session_id.clone()),
                            position_ticks: final_ticks,
                            is_paused: progress.paused,
                            is_muted: progress.muted,
                            audio_stream_index: None,
                            subtitle_stream_index: None,
                            volume_level: progress.volume,
                            play_method: Some("DirectStream".to_string()),
                        };
                        let _ = api::report_playback_progress(&client, &server, &input);
                    }
                    let input = ReportPlaybackStoppedInput {
                        item_id: item_id.clone(),
                        media_source_id,
                        play_session_id: Some(play_session_id.clone()),
                        position_ticks: final_ticks,
                        failed,
                    };
                    let _ = api::report_playback_stopped(&client, &server, &input);
                }
                mpv::forget_control(&play_session_id);
                let _ = app.emit(
                    "playback-stopped",
                    PlaybackStoppedEvent {
                        item_id,
                        play_session_id,
                        failed,
                    },
                );
                break;
            }
            Ok(None) => {}
            Err(_) => break,
        }
    }
}

fn read_mpv_progress(path: &std::path::Path) -> Option<PlaybackStateResult> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds.max(0.0) * 10_000_000.0).round() as i64
}

fn final_position_ticks(
    progress: Option<&PlaybackStateResult>,
    last_ticks: Option<i64>,
    start_position_ticks: Option<i64>,
) -> Option<i64> {
    progress
        .and_then(|progress| progress.time_pos.map(seconds_to_ticks))
        .filter(|ticks| *ticks > 0)
        .or(last_ticks)
        .or(start_position_ticks)
}

pub(crate) fn play_session_id(item_id: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("zplayer-{item_id}-{stamp}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_seconds_to_server_ticks() {
        assert_eq!(seconds_to_ticks(12.345_678_9), 123_456_789);
        assert_eq!(seconds_to_ticks(-1.0), 0);
    }

    #[test]
    fn final_ticks_prefers_current_progress_then_last_then_start() {
        let progress = PlaybackStateResult {
            time_pos: Some(12.0),
            duration: Some(120.0),
            paused: false,
            muted: false,
            volume: Some(100),
            video_ready: true,
        };

        assert_eq!(
            final_position_ticks(Some(&progress), Some(50), Some(10)),
            Some(120_000_000)
        );
        assert_eq!(final_position_ticks(None, Some(50), Some(10)), Some(50));
        assert_eq!(final_position_ticks(None, None, Some(10)), Some(10));
    }

    #[test]
    fn final_ticks_does_not_replace_resume_with_zero_progress() {
        let progress = PlaybackStateResult {
            time_pos: Some(0.0),
            duration: Some(120.0),
            paused: false,
            muted: false,
            volume: Some(100),
            video_ready: false,
        };

        assert_eq!(
            final_position_ticks(Some(&progress), None, Some(90_000_000)),
            Some(90_000_000)
        );
    }
}
