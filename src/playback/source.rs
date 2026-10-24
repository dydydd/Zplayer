use super::session::PlaybackMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MediaSourceCandidate {
    pub(crate) id: Option<String>,
    pub(crate) bitrate: Option<i64>,
    pub(crate) direct_stream_url: Option<String>,
    pub(crate) transcoding_url: Option<String>,
    pub(crate) direct_download_url: Option<String>,
    pub(crate) direct_url: Option<String>,
}

impl MediaSourceCandidate {
    pub(crate) fn playback_url(&self) -> Option<&str> {
        self.direct_stream_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or(self.transcoding_url.as_deref())
            .or(self.direct_download_url.as_deref())
            .or(self.direct_url.as_deref())
    }

    pub(crate) fn playback_mode(&self) -> Option<PlaybackMode> {
        if self
            .direct_stream_url
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return Some(PlaybackMode::DirectStream);
        }
        if self
            .transcoding_url
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return Some(PlaybackMode::Transcoding);
        }
        if self
            .direct_download_url
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return Some(PlaybackMode::DirectDownload);
        }
        self.direct_url
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            .then_some(PlaybackMode::DirectUrl)
    }
}

pub(crate) fn choose_media_source<'a>(
    sources: &'a [MediaSourceCandidate],
    requested_id: Option<&str>,
) -> Option<&'a MediaSourceCandidate> {
    if let Some(id) = requested_id {
        return sources
            .iter()
            .find(|source| source.id.as_deref() == Some(id) && is_playable(source));
    }
    sources.iter().find(|source| is_playable(source))
}

pub(crate) fn choose_media_source_index(
    sources: &[MediaSourceCandidate],
    requested_id: Option<&str>,
) -> Option<usize> {
    if let Some(id) = requested_id {
        return sources
            .iter()
            .position(|source| source.id.as_deref() == Some(id) && is_playable(source));
    }
    sources.iter().position(is_playable)
}

pub(crate) fn sources_by_bitrate_descending(
    sources: &[MediaSourceCandidate],
) -> Vec<&MediaSourceCandidate> {
    let mut ordered: Vec<_> = sources.iter().collect();
    // Stable sorting preserves server order for equal or unknown bitrates.
    ordered.sort_by(|a, b| b.bitrate.unwrap_or(-1).cmp(&a.bitrate.unwrap_or(-1)));
    ordered
}

fn is_playable(source: &MediaSourceCandidate) -> bool {
    source.playback_url().is_some()
}
