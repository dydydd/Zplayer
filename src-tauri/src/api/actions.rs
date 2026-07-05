use super::http::{
    delete_from_server, delete_from_server_with_params, post_empty_to_server_with_params,
    post_to_server, post_to_server_with_params,
};
use crate::models::{
    ReportPlaybackProgressInput, ReportPlaybackStartInput, ReportPlaybackStoppedInput, SavedServer,
};
use reqwest::blocking::Client;
use std::thread;
use std::time::Duration;

const PLAYBACK_REPORT_RETRIES: usize = 3;
const PLAYBACK_REPORT_RETRY_DELAY: Duration = Duration::from_millis(400);

pub(crate) fn set_favorite(
    client: &Client,
    server: &SavedServer,
    item_id: &str,
    value: bool,
) -> Result<(), String> {
    if value {
        post_to_server(
            client,
            server,
            &format!("Users/{}/FavoriteItems/{item_id}", server.user_id),
            &serde_json::json!({}),
        )
        .or_else(|_| {
            post_to_server(
                client,
                server,
                &format!("UserFavoriteItems/{item_id}"),
                &serde_json::json!({}),
            )
        })
    } else {
        delete_from_server(
            client,
            server,
            &format!("Users/{}/FavoriteItems/{item_id}", server.user_id),
        )
        .or_else(|_| delete_from_server(client, server, &format!("UserFavoriteItems/{item_id}")))
    }
}

pub(crate) fn set_played(
    client: &Client,
    server: &SavedServer,
    item_id: &str,
    value: bool,
) -> Result<(), String> {
    if value {
        post_to_server(
            client,
            server,
            &format!("Users/{}/PlayedItems/{item_id}", server.user_id),
            &serde_json::json!({}),
        )
        .or_else(|_| {
            post_to_server(
                client,
                server,
                &format!("UserPlayedItems/{item_id}"),
                &serde_json::json!({}),
            )
        })
    } else {
        delete_from_server(
            client,
            server,
            &format!("Users/{}/PlayedItems/{item_id}", server.user_id),
        )
        .or_else(|_| delete_from_server(client, server, &format!("UserPlayedItems/{item_id}")))
    }
}

pub(crate) fn report_playback_start(
    client: &Client,
    server: &SavedServer,
    input: &ReportPlaybackStartInput,
) -> Result<(), String> {
    let body = serde_json::json!({
        "ItemId": input.item_id,
        "MediaSourceId": input.media_source_id,
        "PlaySessionId": input.play_session_id,
        "AudioStreamIndex": input.audio_stream_index,
        "SubtitleStreamIndex": input.subtitle_stream_index,
        "PositionTicks": input.position_ticks,
        "IsPaused": false,
        "IsMuted": false,
        "CanSeek": true,
        "PlayMethod": "DirectStream"
    });
    retry_playback_report(PLAYBACK_REPORT_RETRIES, PLAYBACK_REPORT_RETRY_DELAY, || {
        playback_report_result(
            "playback start",
            vec![
                (
                    "Sessions/Playing",
                    post_to_server(client, server, "Sessions/Playing", &body),
                ),
                (
                    "Users/{UserId}/PlayingItems/{Id}",
                    report_emby_playback_start(client, server, input),
                ),
            ],
        )
    })
}

pub(crate) fn report_playback_progress(
    client: &Client,
    server: &SavedServer,
    input: &ReportPlaybackProgressInput,
) -> Result<(), String> {
    let body = serde_json::json!({
        "ItemId": input.item_id,
        "MediaSourceId": input.media_source_id,
        "PlaySessionId": input.play_session_id,
        "PositionTicks": input.position_ticks,
        "IsPaused": input.is_paused,
        "IsMuted": input.is_muted,
        "AudioStreamIndex": input.audio_stream_index,
        "SubtitleStreamIndex": input.subtitle_stream_index,
        "VolumeLevel": input.volume_level,
        "PlayMethod": input.play_method.as_deref().unwrap_or("DirectStream"),
        "CanSeek": true
    });
    retry_playback_report(PLAYBACK_REPORT_RETRIES, PLAYBACK_REPORT_RETRY_DELAY, || {
        let mut reports = vec![
            (
                "Sessions/Playing/Progress",
                post_to_server(client, server, "Sessions/Playing/Progress", &body),
            ),
            (
                "Users/{UserId}/PlayingItems/{Id}/Progress",
                report_emby_playback_progress(client, server, input),
            ),
        ];
        if let Some(ticks) = positive_ticks(input.position_ticks) {
            reports.push((
                "UserItems/{itemId}/UserData",
                update_playback_position(client, server, &input.item_id, ticks),
            ));
        }
        playback_report_result("playback progress", reports)
    })
}

pub(crate) fn report_playback_stopped(
    client: &Client,
    server: &SavedServer,
    input: &ReportPlaybackStoppedInput,
) -> Result<(), String> {
    let body = serde_json::json!({
        "ItemId": input.item_id,
        "MediaSourceId": input.media_source_id,
        "PlaySessionId": input.play_session_id,
        "PositionTicks": input.position_ticks,
        "Failed": input.failed
    });
    retry_playback_report(PLAYBACK_REPORT_RETRIES, PLAYBACK_REPORT_RETRY_DELAY, || {
        let mut reports = vec![
            (
                "Sessions/Playing/Stopped",
                post_to_server(client, server, "Sessions/Playing/Stopped", &body),
            ),
            (
                "Users/{UserId}/PlayingItems/{Id}",
                report_emby_playback_stopped(client, server, input),
            ),
        ];
        if let Some(ticks) = positive_ticks(input.position_ticks) {
            reports.push((
                "UserItems/{itemId}/UserData",
                update_playback_position(client, server, &input.item_id, ticks),
            ));
        }
        playback_report_result("playback stopped", reports)
    })
}

fn report_emby_playback_start(
    client: &Client,
    server: &SavedServer,
    input: &ReportPlaybackStartInput,
) -> Result<(), String> {
    post_empty_to_server_with_params(
        client,
        server,
        &format!("Users/{}/PlayingItems/{}", server.user_id, input.item_id),
        &emby_start_params(input),
    )
}

fn report_emby_playback_progress(
    client: &Client,
    server: &SavedServer,
    input: &ReportPlaybackProgressInput,
) -> Result<(), String> {
    post_empty_to_server_with_params(
        client,
        server,
        &format!(
            "Users/{}/PlayingItems/{}/Progress",
            server.user_id, input.item_id
        ),
        &emby_progress_params(input),
    )
}

fn report_emby_playback_stopped(
    client: &Client,
    server: &SavedServer,
    input: &ReportPlaybackStoppedInput,
) -> Result<(), String> {
    delete_from_server_with_params(
        client,
        server,
        &format!("Users/{}/PlayingItems/{}", server.user_id, input.item_id),
        &emby_stopped_params(input),
    )
}

fn update_playback_position(
    client: &Client,
    server: &SavedServer,
    item_id: &str,
    position_ticks: i64,
) -> Result<(), String> {
    post_to_server_with_params(
        client,
        server,
        &format!("UserItems/{item_id}/UserData"),
        &[("userId", server.user_id.clone())],
        &serde_json::json!({
            "PlaybackPositionTicks": position_ticks
        }),
    )
}

fn emby_start_params(input: &ReportPlaybackStartInput) -> Vec<(&'static str, String)> {
    let mut params = vec![
        ("CanSeek", "true".to_string()),
        ("PlayMethod", "DirectStream".to_string()),
    ];
    push_optional_string(
        &mut params,
        "MediaSourceId",
        input.media_source_id.as_deref(),
    );
    push_optional_string(
        &mut params,
        "PlaySessionId",
        input.play_session_id.as_deref(),
    );
    push_optional_i32(&mut params, "AudioStreamIndex", input.audio_stream_index);
    push_optional_i32(
        &mut params,
        "SubtitleStreamIndex",
        input.subtitle_stream_index,
    );
    params
}

fn emby_progress_params(input: &ReportPlaybackProgressInput) -> Vec<(&'static str, String)> {
    let mut params = vec![
        ("IsPaused", input.is_paused.to_string()),
        ("IsMuted", input.is_muted.to_string()),
        (
            "PlayMethod",
            input
                .play_method
                .as_deref()
                .unwrap_or("DirectStream")
                .to_string(),
        ),
    ];
    push_optional_string(
        &mut params,
        "MediaSourceId",
        input.media_source_id.as_deref(),
    );
    push_optional_string(
        &mut params,
        "PlaySessionId",
        input.play_session_id.as_deref(),
    );
    push_optional_i64(&mut params, "PositionTicks", input.position_ticks);
    push_optional_i32(&mut params, "AudioStreamIndex", input.audio_stream_index);
    push_optional_i32(
        &mut params,
        "SubtitleStreamIndex",
        input.subtitle_stream_index,
    );
    push_optional_i32(&mut params, "VolumeLevel", input.volume_level);
    params
}

fn emby_stopped_params(input: &ReportPlaybackStoppedInput) -> Vec<(&'static str, String)> {
    let mut params = vec![("NextMediaType", "Video".to_string())];
    push_optional_string(
        &mut params,
        "MediaSourceId",
        input.media_source_id.as_deref(),
    );
    push_optional_string(
        &mut params,
        "PlaySessionId",
        input.play_session_id.as_deref(),
    );
    push_optional_i64(&mut params, "PositionTicks", input.position_ticks);
    params
}

fn push_optional_string(
    params: &mut Vec<(&'static str, String)>,
    key: &'static str,
    value: Option<&str>,
) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        params.push((key, value.to_string()));
    }
}

fn push_optional_i32(
    params: &mut Vec<(&'static str, String)>,
    key: &'static str,
    value: Option<i32>,
) {
    if let Some(value) = value {
        params.push((key, value.to_string()));
    }
}

fn push_optional_i64(
    params: &mut Vec<(&'static str, String)>,
    key: &'static str,
    value: Option<i64>,
) {
    if let Some(value) = value {
        params.push((key, value.to_string()));
    }
}

fn positive_ticks(value: Option<i64>) -> Option<i64> {
    value.filter(|ticks| *ticks > 0)
}

fn playback_report_result(
    label: &str,
    reports: Vec<(&'static str, Result<(), String>)>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    let mut succeeded = false;
    for (name, result) in reports {
        match result {
            Ok(()) => succeeded = true,
            Err(err) => errors.push(format!("{name}: {err}")),
        }
    }
    if succeeded {
        Ok(())
    } else {
        Err(format!("{label} failed: {}", errors.join(" | ")))
    }
}

fn retry_playback_report(
    attempts: usize,
    delay: Duration,
    mut report: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    let attempts = attempts.max(1);
    let mut last_error = None;
    for attempt in 1..=attempts {
        match report() {
            Ok(()) => return Ok(()),
            Err(err) => last_error = Some(err),
        }
        if attempt < attempts {
            thread::sleep(delay);
        }
    }
    Err(last_error.unwrap_or_else(|| "Playback report failed.".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_transient_playback_report_failure() {
        let mut calls = 0;

        retry_playback_report(3, Duration::ZERO, || {
            calls += 1;
            if calls < 2 {
                Err("not yet".to_string())
            } else {
                Ok(())
            }
        })
        .unwrap();

        assert_eq!(calls, 2);
    }

    #[test]
    fn returns_last_playback_report_error() {
        let err =
            retry_playback_report(2, Duration::ZERO, || Err("still down".to_string())).unwrap_err();

        assert_eq!(err, "still down");
    }

    #[test]
    fn emby_progress_params_include_resume_identity() {
        let input = ReportPlaybackProgressInput {
            item_id: "item".to_string(),
            media_source_id: Some("source".to_string()),
            play_session_id: Some("session".to_string()),
            position_ticks: Some(123_456_789),
            is_paused: false,
            is_muted: true,
            audio_stream_index: Some(1),
            subtitle_stream_index: Some(2),
            volume_level: Some(80),
            play_method: Some("DirectStream".to_string()),
        };

        let params = emby_progress_params(&input);

        assert!(params.contains(&("MediaSourceId", "source".to_string())));
        assert!(params.contains(&("PlaySessionId", "session".to_string())));
        assert!(params.contains(&("PositionTicks", "123456789".to_string())));
        assert!(params.contains(&("IsPaused", "false".to_string())));
        assert!(params.contains(&("IsMuted", "true".to_string())));
        assert!(params.contains(&("VolumeLevel", "80".to_string())));
    }

    #[test]
    fn emby_stopped_params_include_final_position() {
        let input = ReportPlaybackStoppedInput {
            item_id: "item".to_string(),
            media_source_id: Some("source".to_string()),
            play_session_id: Some("session".to_string()),
            position_ticks: Some(987_654_321),
            failed: false,
        };

        let params = emby_stopped_params(&input);

        assert!(params.contains(&("MediaSourceId", "source".to_string())));
        assert!(params.contains(&("PlaySessionId", "session".to_string())));
        assert!(params.contains(&("PositionTicks", "987654321".to_string())));
        assert!(params.contains(&("NextMediaType", "Video".to_string())));
    }

    #[test]
    fn playback_report_succeeds_when_any_protocol_succeeds() {
        playback_report_result(
            "progress",
            vec![
                (
                    "Sessions/Playing/Progress",
                    Err("not supported".to_string()),
                ),
                ("Users/{UserId}/PlayingItems/{Id}/Progress", Ok(())),
            ],
        )
        .unwrap();
    }

    #[test]
    fn playback_report_collects_all_protocol_errors() {
        let err = playback_report_result(
            "progress",
            vec![
                ("Sessions/Playing/Progress", Err("first".to_string())),
                (
                    "Users/{UserId}/PlayingItems/{Id}/Progress",
                    Err("second".to_string()),
                ),
            ],
        )
        .unwrap_err();

        assert!(err.contains("Sessions/Playing/Progress: first"));
        assert!(err.contains("Users/{UserId}/PlayingItems/{Id}/Progress: second"));
    }
}
