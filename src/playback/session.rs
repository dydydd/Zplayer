use serde::{Deserialize, Serialize};
use url::Url;

use super::{
    request::OnlinePlaybackRequest,
    source::{MediaSourceCandidate, choose_media_source},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PlaybackMode {
    DirectStream,
    Transcoding,
    DirectDownload,
    DirectUrl,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AddressFamilyPreference {
    #[default]
    System,
    Ipv4,
    Ipv6,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NetworkOptions {
    pub(crate) user_agent: Option<String>,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) disable_ytdl: bool,
    pub(crate) address_family: AddressFamilyPreference,
}

impl NetworkOptions {
    pub(crate) fn mpv_http_header_fields(&self) -> Option<String> {
        let mut fields = String::new();
        let mut has_user_agent = false;
        for (name, value) in &self.headers {
            if name.eq_ignore_ascii_case("User-Agent") {
                has_user_agent = true;
            }
            if !fields.is_empty() {
                fields.push(',');
            }
            fields.push_str(name);
            fields.push(':');
            fields.push_str(value);
        }
        if let Some(user_agent) = self.user_agent.as_deref().filter(|value| !value.is_empty()) {
            if !has_user_agent {
                if !fields.is_empty() {
                    fields.push(',');
                }
                fields.push_str("User-Agent:");
                fields.push_str(user_agent);
            }
        }
        (!fields.is_empty()).then_some(fields)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitialTrackSelection {
    pub(crate) audio_stream_index: Option<i32>,
    pub(crate) subtitle_stream_index: Option<i32>,
    pub(crate) disable_subtitles: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedPlaybackSession {
    #[serde(default)]
    pub(crate) server_id: String,
    pub(crate) url: String,
    pub(crate) mode: PlaybackMode,
    pub(crate) media_source_id: Option<String>,
    pub(crate) play_session_id: Option<String>,
    pub(crate) subtitle_url: Option<String>,
    pub(crate) network_options: NetworkOptions,
    pub(crate) initial_tracks: InitialTrackSelection,
    pub(crate) start_position_ticks: i64,
}

impl ResolvedPlaybackSession {
    pub(crate) fn play_method(&self) -> &'static str {
        match self.mode {
            PlaybackMode::DirectStream | PlaybackMode::DirectDownload | PlaybackMode::DirectUrl => {
                "DirectStream"
            }
            PlaybackMode::Transcoding => "Transcoding",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionResolutionError {
    NoPlayableSource,
    InvalidServerBase(String),
    InvalidPlaybackUrl(String),
}

impl std::fmt::Display for SessionResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoPlayableSource => write!(f, "No playable media source matched the request."),
            Self::InvalidServerBase(value) => write!(f, "Invalid server base URL: {value}"),
            Self::InvalidPlaybackUrl(value) => write!(f, "Invalid playback URL: {value}"),
        }
    }
}

impl std::error::Error for SessionResolutionError {}

pub(crate) fn resolve_playback_session(
    request: OnlinePlaybackRequest,
    server_base: &str,
    sources: &[MediaSourceCandidate],
    play_session_id: Option<&str>,
    subtitle_url: Option<&str>,
    network_options: NetworkOptions,
) -> Result<ResolvedPlaybackSession, SessionResolutionError> {
    let OnlinePlaybackRequest {
        server_id,
        media_source_id,
        audio_stream_index,
        subtitle_stream_index,
        start_position_ticks,
        ..
    } = request;
    let source = choose_media_source(sources, media_source_id.as_deref())
        .ok_or(SessionResolutionError::NoPlayableSource)?;
    let raw_url = source
        .playback_url()
        .expect("selected playable source has URL");
    let mode = source
        .playback_mode()
        .expect("selected playable source has a mode");
    let mut url = normalize_playback_url(server_base, raw_url)?;
    if let Some(session_id) = play_session_id.filter(|value| !value.is_empty()) {
        append_query_once(&mut url, "PlaySessionId", session_id)?;
    }

    Ok(ResolvedPlaybackSession {
        server_id,
        url,
        mode,
        media_source_id: source.id.clone(),
        play_session_id: play_session_id.map(str::to_owned),
        subtitle_url: subtitle_url
            .map(|value| normalize_playback_url(server_base, value))
            .transpose()?,
        network_options,
        initial_tracks: InitialTrackSelection {
            audio_stream_index,
            subtitle_stream_index: subtitle_stream_index.filter(|index| *index >= 0),
            disable_subtitles: subtitle_stream_index == Some(-1),
        },
        start_position_ticks: start_position_ticks.max(0),
    })
}

pub(crate) fn normalize_playback_url(
    server_base: &str,
    input: &str,
) -> Result<String, SessionResolutionError> {
    let input = input.trim();
    if let Some(rest) = input
        .strip_prefix("/http://")
        .or_else(|| input.strip_prefix("/https://"))
    {
        return Ok(format!(
            "{}://{}",
            if input.starts_with("/https://") {
                "https"
            } else {
                "http"
            },
            rest
        ));
    }
    if let Ok(url) = Url::parse(input) {
        if matches!(url.scheme(), "http" | "https") {
            return Ok(input.to_owned());
        }
    }
    let base = Url::parse(server_base.trim_end_matches('/'))
        .map_err(|_| SessionResolutionError::InvalidServerBase(server_base.to_owned()))?;
    base.join(input)
        .map(|url| url.to_string())
        .map_err(|_| SessionResolutionError::InvalidPlaybackUrl(input.to_owned()))
}

fn append_query_once(
    url: &mut String,
    key: &str,
    value: &str,
) -> Result<(), SessionResolutionError> {
    let mut parsed =
        Url::parse(url).map_err(|_| SessionResolutionError::InvalidPlaybackUrl(url.clone()))?;
    if !parsed
        .query_pairs()
        .any(|(name, _)| name.eq_ignore_ascii_case(key))
    {
        parsed.query_pairs_mut().append_pair(key, value);
        *url = parsed.to_string();
    }
    Ok(())
}
