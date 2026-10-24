use super::*;
use crate::playback::{
    InitialTrackSelection, NetworkOptions, PlaybackMode, ResolvedPlaybackSession,
};
use std::path::PathBuf;

fn playback(url: &str) -> ResolvedPlaybackSession {
    ResolvedPlaybackSession {
        server_id: "server".into(),
        url: url.into(),
        mode: PlaybackMode::DirectStream,
        media_source_id: None,
        play_session_id: None,
        subtitle_url: None,
        network_options: NetworkOptions::default(),
        initial_tracks: InitialTrackSelection::default(),
        start_position_ticks: 0,
    }
}

#[test]
fn builds_bundled_library_candidates_before_loader_names() {
    let paths = PlayerPaths {
        libmpv_path: None,
        config_dir: "/app/config".into(),
        log_dir: "/app/log".into(),
    };
    let candidates = library_candidates(&paths);
    assert!(candidates.iter().any(|p| {
        p.components()
            .any(|component| component.as_os_str() == bundled_platform_dir())
    }));
    assert!(
        candidates
            .iter()
            .any(|p| p == &PathBuf::from(default_test_library_name()))
    );
}

#[cfg(target_os = "windows")]
fn default_test_library_name() -> &'static str {
    "libmpv-2.dll"
}
#[cfg(target_os = "macos")]
fn default_test_library_name() -> &'static str {
    "Mpv.xcframework/macos-arm64_x86_64/Mpv.framework/Versions/A/Mpv"
}
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn default_test_library_name() -> &'static str {
    "libmpv.so.2"
}

#[test]
fn converts_resume_ticks_to_seconds() {
    assert_eq!(mpv_start_value(90_500_000).as_deref(), Some("9.050"));
    assert_eq!(mpv_start_value(0), None);
    assert_eq!(mpv_start_value(-1), None);
}

#[test]
fn renders_headers_tracks_and_ytdl_options() {
    let mut session = playback("https://example.test/video");
    session.network_options.user_agent = Some("Zplayer test".into());
    session
        .network_options
        .headers
        .push(("X-Session".into(), "abc".into()));
    session.network_options.disable_ytdl = true;
    session.initial_tracks.audio_stream_index = Some(2);
    session.initial_tracks.subtitle_stream_index = Some(3);
    let options = mpv_options(&session, 120);
    assert!(options.contains(&(
        "http-header-fields".into(),
        "X-Session:abc,User-Agent:Zplayer test".into()
    )));
    assert!(options.contains(&("ytdl".into(), "no".into())));
    assert!(options.contains(&("aid".into(), "2".into())));
    assert!(options.contains(&("sid".into(), "3".into())));
    assert!(options.contains(&("volume".into(), "100".into())));
}

#[test]
fn transcoding_forces_ytdl_off() {
    let mut session = playback("https://example.test/video");
    session.mode = PlaybackMode::Transcoding;
    let options = mpv_options(&session, 100);
    assert!(options.contains(&("ytdl".into(), "no".into())));
}

#[test]
fn redacts_known_query_tokens_and_explicit_secret() {
    assert_eq!(
        redact_secrets("https://srv/v?api_key=secret&x=1", None),
        "https://srv/v?api_key=***&x=1"
    );
    assert_eq!(
        redact_secrets("header secret", Some("secret")),
        "header ***"
    );
}
