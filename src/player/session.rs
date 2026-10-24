use super::loader::{LibmpvLoader, PlayerPaths, library_candidates};
use crate::playback::{PlaybackMode, ResolvedPlaybackSession};
use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

const MPV_FORMAT_DOUBLE: libc::c_int = 5;
const MPV_FORMAT_FLAG: libc::c_int = 3;

pub(crate) struct PlayerSessionConfig {
    pub paths: PlayerPaths,
    pub playback: ResolvedPlaybackSession,
    pub default_volume: i32,
    pub audio_device: Option<String>,
}

/// A point-in-time snapshot of the player, all fields `Copy` so polling the UI
/// copies it without any heap allocation.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct PlayerState {
    pub time_pos: Option<f64>,
    pub duration: Option<f64>,
    pub paused: bool,
    pub volume: Option<f64>,
    pub stopped: bool,
}

#[derive(Clone)]
pub(crate) struct PlayerSession {
    loader: LibmpvLoader,
    handle: *mut libc::c_void,
    destroyed: Arc<AtomicBool>,
    token: Option<String>,
    server_id: String,
    audio_stream_index: Option<i32>,
    subtitle_stream_index: Option<i32>,
}

// libmpv permits calls from multiple threads. We serialize public operations
// through the owning application; raw handle access remains encapsulated here.
unsafe impl Send for PlayerSession {}

impl PlayerSession {
    pub(crate) fn start(config: PlayerSessionConfig) -> Result<Self, String> {
        let loader = load_from_paths(&config.paths)?;
        let handle = unsafe { (loader.create)() };
        if handle.is_null() {
            return Err("libmpv could not create a playback handle.".into());
        }

        let mut player = Self {
            loader,
            handle,
            destroyed: Arc::new(AtomicBool::new(false)),
            token: secret_from_url(&config.playback.url),
            server_id: config.playback.server_id.clone(),
            audio_stream_index: config
                .playback
                .initial_tracks
                .audio_stream_index
                .filter(|index| *index >= 0),
            subtitle_stream_index: config.playback.initial_tracks.subtitle_stream_index,
        };
        let result = player.initialize(config);
        if let Err(error) = result {
            player.terminate();
            return Err(error);
        }
        Ok(player)
    }

    fn initialize(&mut self, config: PlayerSessionConfig) -> Result<(), String> {
        let mut options = mpv_options(&config.playback, config.default_volume);
        if let Some(device) = config.audio_device.filter(|s| !s.is_empty()) {
            options.push(("audio-device".into(), device));
        }
        if let Some(dir) = config.paths.config_dir.to_str() {
            options.push(("config-dir".into(), dir.to_owned()));
        }
        std::fs::create_dir_all(&config.paths.log_dir)
            .map_err(|e| format!("Could not create player log directory: {e}"))?;
        for (name, value) in options {
            self.set_option(&name, &value)?;
        }
        let status = unsafe { (self.loader.initialize)(self.handle) };
        self.check(status, "initialize libmpv")?;

        self.command(&["loadfile", &config.playback.url, "replace"])?;
        if let Some(seconds) = mpv_start_value(config.playback.start_position_ticks) {
            self.command(&["seek", &seconds, "absolute", "exact"])?;
        }
        if let Some(subtitle) = config.playback.subtitle_url.as_deref() {
            self.command(&["sub-add", subtitle, "select"])?;
        }
        Ok(())
    }

    pub(crate) fn play(&self) -> Result<(), String> {
        self.set_property_flag("pause", false)
    }
    pub(crate) fn pause(&self) -> Result<(), String> {
        self.set_property_flag("pause", true)
    }
    pub(crate) fn toggle_pause(&self) -> Result<(), String> {
        self.command(&["cycle", "pause"])
    }
    pub(crate) fn seek(&self, seconds: f64) -> Result<(), String> {
        self.command(&[
            "seek",
            &format!("{:.3}", seconds.max(0.0)),
            "absolute",
            "exact",
        ])
    }
    pub(crate) fn set_volume(&self, volume: i32) -> Result<(), String> {
        self.set_property_string("volume", &volume.clamp(0, 100).to_string())
    }
    pub(crate) fn state(&self) -> Result<PlayerState, String> {
        Ok(PlayerState {
            time_pos: self.get_double("time-pos")?,
            duration: self.get_double("duration")?,
            paused: self.get_flag("pause")?.unwrap_or(false),
            volume: self.get_double("volume")?,
            stopped: self.get_flag("idle-active")?.unwrap_or(false)
                && self.get_double("time-pos")?.is_none(),
        })
    }
    pub(crate) fn stop(&self) -> Result<(), String> {
        self.command(&["stop"])
    }
    pub(crate) fn server_id(&self) -> &str {
        &self.server_id
    }
    pub(crate) fn audio_stream_index(&self) -> Option<i32> {
        self.audio_stream_index
    }
    pub(crate) fn subtitle_stream_index(&self) -> Option<i32> {
        self.subtitle_stream_index
    }
    pub(crate) fn terminate(&mut self) {
        if !self.destroyed.swap(true, Ordering::AcqRel) && !self.handle.is_null() {
            unsafe { (self.loader.terminate_destroy)(self.handle) };
            self.handle = ptr::null_mut();
        }
    }

    fn set_option(&self, name: &str, value: &str) -> Result<(), String> {
        let name = c_string(name)?;
        let value = c_string(value)?;
        let status =
            unsafe { (self.loader.set_option_string)(self.handle, name.as_ptr(), value.as_ptr()) };
        self.check(status, "set libmpv option")
    }
    fn set_property_string(&self, name: &str, value: &str) -> Result<(), String> {
        self.command(&["set", name, value])
    }
    fn set_property_flag(&self, name: &str, value: bool) -> Result<(), String> {
        self.command(&["set", name, if value { "yes" } else { "no" }])
    }
    fn get_double(&self, name: &str) -> Result<Option<f64>, String> {
        let name = c_string(name)?;
        let mut value = 0.0f64;
        let status = unsafe {
            (self.loader.get_property)(
                self.handle,
                name.as_ptr(),
                MPV_FORMAT_DOUBLE,
                (&mut value as *mut f64).cast(),
            )
        };
        if status < 0 {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }
    fn get_flag(&self, name: &str) -> Result<Option<bool>, String> {
        let name = c_string(name)?;
        let mut value = 0i32;
        let status = unsafe {
            (self.loader.get_property)(
                self.handle,
                name.as_ptr(),
                MPV_FORMAT_FLAG,
                (&mut value as *mut i32).cast(),
            )
        };
        if status < 0 {
            Ok(None)
        } else {
            Ok(Some(value != 0))
        }
    }
    fn command(&self, args: &[&str]) -> Result<(), String> {
        let strings = args
            .iter()
            .map(|s| c_string(s))
            .collect::<Result<Vec<_>, _>>()?;
        let mut pointers = strings.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        pointers.push(ptr::null());
        let status = unsafe { (self.loader.command)(self.handle, pointers.as_ptr()) };
        self.check(status, "run libmpv command")
    }
    fn check(&self, status: libc::c_int, operation: &str) -> Result<(), String> {
        if status >= 0 {
            return Ok(());
        }
        let message = unsafe {
            let ptr = (self.loader.error_string)(status);
            if ptr.is_null() {
                "unknown libmpv error".to_owned()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        Err(redact_secrets(
            &format!("Could not {operation}: {message}"),
            self.token.as_deref(),
        ))
    }
}

impl Drop for PlayerSession {
    fn drop(&mut self) {
        self.terminate();
    }
}

fn load_from_paths(paths: &PlayerPaths) -> Result<LibmpvLoader, String> {
    let candidates = library_candidates(paths);
    let mut errors = Vec::new();
    for path in candidates {
        if path.is_file() {
            match LibmpvLoader::load(&path) {
                Ok(loader) => return Ok(loader),
                Err(error) => errors.push(error),
            }
        }
    }
    if errors.is_empty() {
        Err(format!(
            "libmpv not found. Bundle it under libmpv/{} or configure its path.",
            super::bundled_platform_dir()
        ))
    } else {
        Err(errors.join("\n"))
    }
}

pub(crate) fn mpv_options(
    playback: &ResolvedPlaybackSession,
    default_volume: i32,
) -> Vec<(String, String)> {
    let mut options = vec![
        ("volume".into(), default_volume.clamp(0, 100).to_string()),
        ("input-default-bindings".into(), "no".into()),
        ("input-vo-keyboard".into(), "no".into()),
        ("osd-bar".into(), "no".into()),
        ("osd-level".into(), "0".into()),
        ("hwdec".into(), "auto-safe".into()),
        ("keep-open".into(), "no".into()),
    ];
    if let Some(headers) = playback.network_options.mpv_http_header_fields() {
        options.push(("http-header-fields".into(), headers));
    }
    if playback.network_options.disable_ytdl || playback.mode == PlaybackMode::Transcoding {
        options.push(("ytdl".into(), "no".into()));
    }
    // mpv does not expose a general IPv4/IPv6 preference for direct HTTP
    // connections. Do not pass similarly named, unrelated options that would
    // make mpv initialization fail.
    let _ = playback.network_options.address_family;
    match playback.initial_tracks.audio_stream_index {
        Some(index) if index >= 0 => options.push(("aid".into(), index.to_string())),
        _ => {}
    }
    if playback.initial_tracks.disable_subtitles {
        options.push(("sid".into(), "no".into()));
    } else if playback.subtitle_url.is_none() {
        if let Some(index) = playback.initial_tracks.subtitle_stream_index {
            options.push(("sid".into(), index.to_string()));
        }
    }
    options
}

pub(crate) fn mpv_start_value(ticks: i64) -> Option<String> {
    (ticks > 0).then(|| format!("{:.3}", ticks as f64 / 10_000_000.0))
}

pub(crate) fn redact_secrets(text: &str, secret: Option<&str>) -> String {
    let mut result = if let Some(secret) = secret.filter(|s| !s.is_empty()) {
        text.replace(secret, "***")
    } else {
        text.to_owned()
    };
    for key in ["api_key", "apikey", "access_token", "token"] {
        result = redact_query_value(&result, key);
    }
    result
}

fn redact_query_value(input: &str, key: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let needle = format!("{key}=");
    let Some(start) = lower.find(&needle) else {
        return input.to_owned();
    };
    let value_start = start + needle.len();
    let end = input[value_start..]
        .find(['&', '#', ' ', '\n', '\r'])
        .map(|i| value_start + i)
        .unwrap_or(input.len());
    format!("{}***{}", &input[..value_start], &input[end..])
}

fn secret_from_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    parsed
        .query_pairs()
        .find(|(key, _)| {
            matches!(
                key.to_ascii_lowercase().as_str(),
                "api_key" | "apikey" | "access_token" | "token"
            )
        })
        .map(|(_, value)| value.into_owned())
}

fn c_string(value: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| "libmpv argument contains an embedded NUL byte.".into())
}
