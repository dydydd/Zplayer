use super::*;

fn source(id: &str, bitrate: Option<i64>, url: Option<&str>) -> MediaSourceCandidate {
    MediaSourceCandidate {
        id: Some(id.to_owned()),
        bitrate,
        direct_stream_url: url.map(str::to_owned),
        transcoding_url: None,
        direct_download_url: None,
        direct_url: None,
    }
}

fn transcoding_source(id: &str, url: &str) -> MediaSourceCandidate {
    MediaSourceCandidate {
        id: Some(id.to_owned()),
        bitrate: None,
        direct_stream_url: None,
        transcoding_url: Some(url.to_owned()),
        direct_download_url: None,
        direct_url: None,
    }
}

fn request() -> OnlinePlaybackRequest {
    OnlinePlaybackRequest {
        server_id: "server".into(),
        item_id: "item".into(),
        media_source_id: None,
        audio_stream_index: Some(2),
        subtitle_stream_index: Some(-1),
        start_position_ticks: 42,
        player_choice: PlaybackPlayerChoice::Internal,
    }
}

#[test]
fn explicit_source_wins_and_unplayable_explicit_source_fails() {
    let sources = [
        source("a", Some(1), Some("/a")),
        source("b", Some(2), Some("/b")),
    ];
    assert_eq!(
        choose_media_source(&sources, Some("a"))
            .unwrap()
            .id
            .as_deref(),
        Some("a")
    );
    let sources = [source("a", Some(1), None)];
    assert!(choose_media_source(&sources, Some("a")).is_none());
}

#[test]
fn default_source_selection_preserves_server_order() {
    let sources = [
        source("first", Some(1), None),
        source("next", Some(2), Some("/next")),
        source("last", Some(3), Some("/last")),
    ];
    assert_eq!(
        choose_media_source(&sources, None).unwrap().id.as_deref(),
        Some("next")
    );
}

#[test]
fn bitrate_order_is_descending_and_stable_for_ties() {
    let sources = [
        source("a", Some(10), Some("/a")),
        source("b", None, Some("/b")),
        source("c", Some(10), Some("/c")),
        source("d", Some(20), Some("/d")),
    ];
    let ids: Vec<_> = sources_by_bitrate_descending(&sources)
        .iter()
        .map(|source| source.id.as_deref().unwrap())
        .collect();
    assert_eq!(ids, ["d", "a", "c", "b"]);
}

#[test]
fn normalizes_absolute_relative_and_reverse_proxy_urls() {
    assert_eq!(
        normalize_playback_url("http://server/base", "https://cdn/media").unwrap(),
        "https://cdn/media"
    );
    assert_eq!(
        normalize_playback_url("http://server/base", "/Videos/item").unwrap(),
        "http://server/Videos/item"
    );
    assert_eq!(
        normalize_playback_url("http://server", "/http://proxy/path").unwrap(),
        "http://proxy/path"
    );
    assert_eq!(
        normalize_playback_url("http://server", "/https://proxy/path").unwrap(),
        "https://proxy/path"
    );
}

#[test]
fn adds_play_session_once_and_preserves_existing_query() {
    let sources = [source("a", None, Some("/stream?token=x&playsessionid=old"))];
    let resolved = resolve_playback_session(
        request(),
        "http://server",
        &sources,
        Some("new"),
        None,
        NetworkOptions::default(),
    )
    .unwrap();
    assert!(resolved.url.contains("token=x"));
    assert_eq!(
        resolved
            .url
            .to_ascii_lowercase()
            .matches("playsessionid=")
            .count(),
        1
    );
    assert!(resolved.url.contains("playsessionid=old"));
}

#[test]
fn resolves_session_and_track_selection() {
    let sources = [source("a", None, Some("/stream"))];
    let resolved = resolve_playback_session(
        request(),
        "http://server",
        &sources,
        Some("session"),
        Some("/subtitles/2.srt"),
        NetworkOptions {
            disable_ytdl: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(resolved.url, "http://server/stream?PlaySessionId=session");
    assert_eq!(
        resolved.subtitle_url.as_deref(),
        Some("http://server/subtitles/2.srt")
    );
    assert_eq!(resolved.initial_tracks.audio_stream_index, Some(2));
    assert_eq!(resolved.initial_tracks.subtitle_stream_index, None);
    assert!(resolved.initial_tracks.disable_subtitles);
    assert_eq!(resolved.start_position_ticks, 42);
    assert!(resolved.network_options.disable_ytdl);
}

#[test]
fn transcoding_source_resolves_to_transcoding_mode() {
    let sources = [transcoding_source("a", "/transcode.m3u8")];
    let resolved = resolve_playback_session(
        request(),
        "http://server",
        &sources,
        Some("session"),
        None,
        NetworkOptions::default(),
    )
    .unwrap();
    assert_eq!(resolved.mode, PlaybackMode::Transcoding);
    assert_eq!(resolved.play_method(), "Transcoding");
    assert_eq!(
        resolved.url,
        "http://server/transcode.m3u8?PlaySessionId=session"
    );
    assert_eq!(resolved.mode, PlaybackMode::Transcoding);
}

#[test]
fn network_options_render_mpv_http_headers() {
    let options = NetworkOptions {
        user_agent: Some("Zplayer".to_string()),
        headers: vec![("X-Custom".to_string(), "yes".to_string())],
        disable_ytdl: false,
        address_family: AddressFamilyPreference::System,
    };

    assert_eq!(
        options.mpv_http_header_fields().as_deref(),
        Some("X-Custom:yes,User-Agent:Zplayer")
    );
    assert_eq!(NetworkOptions::default().mpv_http_header_fields(), None);
}

#[test]
fn source_index_matches_selected_source() {
    let sources = [
        source("a", None, None),
        source("b", None, Some("/b")),
        source("c", None, Some("/c")),
    ];

    assert_eq!(choose_media_source_index(&sources, None), Some(1));
    assert_eq!(choose_media_source_index(&sources, Some("c")), Some(2));
    assert_eq!(choose_media_source_index(&sources, Some("a")), None);
}

#[test]
fn negative_start_position_is_clamped_and_missing_source_errors() {
    let mut req = request();
    req.start_position_ticks = -1;
    let sources = [source("a", None, Some("/stream"))];
    let resolved = resolve_playback_session(
        req,
        "http://server",
        &sources,
        None,
        None,
        NetworkOptions::default(),
    )
    .unwrap();
    assert_eq!(resolved.start_position_ticks, 0);
    assert_eq!(
        resolve_playback_session(
            request(),
            "http://server",
            &[],
            None,
            None,
            NetworkOptions::default()
        ),
        Err(SessionResolutionError::NoPlayableSource)
    );
}
