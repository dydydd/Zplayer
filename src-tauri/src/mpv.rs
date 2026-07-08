use crate::models::{AppSettings, PlayResult, PlaybackStateResult, SavedServer};
use crate::store;
use std::collections::{HashMap, HashSet};
use std::fs;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

static CONTROL_PATHS: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();
static PROGRESS_PATHS: OnceLock<Mutex<HashMap<String, PathBuf>>> = OnceLock::new();
static PROGRESS_STATES: OnceLock<Mutex<HashMap<String, PlaybackStateResult>>> = OnceLock::new();
static PROCESS_IDS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();
static EXPLICIT_STOPS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
const DISABLE_MPV_UI_ARGS: &[&str] = &[
    "--input-default-bindings=no",
    "--input-vo-keyboard=no",
    "--osc=no",
    "--osd-bar=no",
    "--osd-level=0",
];
#[cfg(any(test, target_os = "windows"))]
const EMBED_MPV_WINDOW_ARGS: &[&str] = &["--force-window=yes", "--no-border", "--ontop=no"];
const MPV_ENGINE_ARGS: &[&str] = &["--no-ytdl", "--keep-open=no"];
const MPV_SYNC_START_ARGS: &[&str] = &["--pause=yes"];
const MPV_LOG_LEVEL_ARG: &str = "--msg-level=all=warn";
const MPV_STARTUP_CHECK_INTERVAL: Duration = Duration::from_millis(25);
const MPV_STARTUP_CHECK_MAX: Duration = Duration::from_millis(300);
#[cfg(target_os = "windows")]
const EMBED_INITIAL_RESTACKS: usize = 40;
#[cfg(target_os = "windows")]
const EMBED_INITIAL_RESTACK_DELAY: Duration = Duration::from_millis(25);
#[cfg(target_os = "windows")]
const TERMINATE_GRACE_MS: u32 = 700;

pub(crate) struct Launch {
    pub(crate) result: PlayResult,
    pub(crate) child: Child,
    pub(crate) progress_path: PathBuf,
    pub(crate) control_path: PathBuf,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn launch(
    app: &AppHandle,
    server: &SavedServer,
    item_id: &str,
    media_source_id: Option<String>,
    play_session_id: String,
    stream_url: &str,
    subtitle_track_position: Option<i32>,
    subtitle_url: Option<&str>,
    start_position_ticks: Option<i64>,
) -> Result<Launch, String> {
    let redacted_url = redact_secret(stream_url, &server.access_token);
    let settings = store::settings(app)?;
    let mpv_path = find_mpv(app, &settings)?;
    let log_path = mpv_log_path(app, item_id)?;
    let progress_path = mpv_progress_path(app, item_id)?;
    let control_path = mpv_control_path(app, item_id)?;
    let script_path = mpv_progress_script_path(
        app,
        item_id,
        &progress_path,
        &control_path,
        seek_back_seconds(&settings),
        seek_forward_seconds(&settings),
    )?;
    let mut command = Command::new(&mpv_path);
    if let Some(config_dir) = find_mpv_config_dir(app) {
        command.arg(format!("--config-dir={}", config_dir.display()));
    }
    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);
    add_embed_args(app, &mut command)?;
    command
        .arg(format!("--log-file={}", log_path.display()))
        .arg(format!("--script={}", script_path.display()))
        .arg(MPV_LOG_LEVEL_ARG)
        .args(MPV_ENGINE_ARGS)
        .args(MPV_SYNC_START_ARGS)
        .arg(format!(
            "--http-header-fields=X-Emby-Token: {}",
            server.access_token
        ));
    for arg in DISABLE_MPV_UI_ARGS {
        command.arg(arg);
    }
    for arg in mpv_settings_args(&settings) {
        command.arg(arg);
    }
    for arg in mpv_subtitle_args(&settings, subtitle_track_position, subtitle_url) {
        command.arg(arg);
    }
    if let Some(arg) = mpv_start_arg(start_position_ticks) {
        command.arg(arg);
    }
    command.arg(stream_url);

    let mut child = command
        .spawn()
        .map_err(|err| format!("Failed to launch mpv: {err}"))?;
    place_embedded_video_behind_webview(app, child.id());
    if let Some(status) = wait_for_early_exit(app, &mut child)? {
        let tail = read_log_tail(&log_path)
            .map(|text| redact_secret(&text, &server.access_token))
            .unwrap_or_else(|err| format!("Failed to read mpv log: {err}"));
        return Err(format!(
            "mpv exited immediately with status {status}. Log: {}\n{}",
            log_path.display(),
            tail
        ));
    }

    Ok(Launch {
        result: PlayResult {
            item_id: item_id.to_string(),
            media_source_id,
            play_session_id,
            url: redacted_url,
            log_path: log_path.display().to_string(),
            log_tail: read_log_excerpt(&log_path, &server.access_token),
        },
        child,
        progress_path,
        control_path,
    })
}

fn wait_for_early_exit(
    app: &AppHandle,
    child: &mut Child,
) -> Result<Option<std::process::ExitStatus>, String> {
    let checks =
        (MPV_STARTUP_CHECK_MAX.as_millis() / MPV_STARTUP_CHECK_INTERVAL.as_millis()).max(1);
    for _ in 0..checks {
        place_embedded_video_behind_webview_once(app, child.id());
        thread::sleep(MPV_STARTUP_CHECK_INTERVAL);
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("Failed to inspect mpv status: {err}"))?
        {
            return Ok(Some(status));
        }
    }
    Ok(None)
}

pub(crate) fn control(play_session_id: &str, command: &str) -> Result<(), String> {
    let path = CONTROL_PATHS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "Failed to access playback controls.".to_string())?
        .get(play_session_id)
        .cloned()
        .ok_or_else(|| "Playback session is not active.".to_string())?;
    let command = normalize_command(command)?;
    if command == "stop" {
        remember_explicit_stop(play_session_id);
    }
    fs::write(path, command).map_err(|err| format!("Failed to send playback command: {err}"))
}

fn normalize_command(command: &str) -> Result<String, String> {
    match command {
        "toggle_pause" | "seek_back" | "seek_forward" | "volume_down" | "volume_up"
        | "toggle_mute" | "audio_next" | "subtitle_next" | "speed_down" | "speed_up" | "resume"
        | "stop" => Ok(command.to_string()),
        value if value.starts_with("seek_absolute:") => {
            let seconds = value
                .trim_start_matches("seek_absolute:")
                .parse::<f64>()
                .map_err(|_| "Invalid seek target.".to_string())?;
            Ok(format!("seek_absolute:{:.3}", seconds.max(0.0)))
        }
        value if value.starts_with("audio_set:") => {
            let index = value
                .trim_start_matches("audio_set:")
                .parse::<i32>()
                .map_err(|_| "Invalid audio track.".to_string())?;
            Ok(format!("audio_set:{index}"))
        }
        value if value.starts_with("subtitle_set:") => {
            let index = value
                .trim_start_matches("subtitle_set:")
                .parse::<i32>()
                .map_err(|_| "Invalid subtitle track.".to_string())?;
            Ok(format!("subtitle_set:{index}"))
        }
        value if value.starts_with("volume_set:") => {
            let volume = value
                .trim_start_matches("volume_set:")
                .parse::<i32>()
                .map_err(|_| "Invalid volume.".to_string())?
                .clamp(0, 100);
            Ok(format!("volume_set:{volume}"))
        }
        value if value.starts_with("speed_set:") => {
            let speed = value
                .trim_start_matches("speed_set:")
                .parse::<f64>()
                .map_err(|_| "Invalid playback speed.".to_string())?
                .clamp(0.5, 2.0);
            Ok(format!("speed_set:{speed:.2}"))
        }
        value if value.starts_with("audio_delay_set:") => {
            let delay = value
                .trim_start_matches("audio_delay_set:")
                .parse::<f64>()
                .map_err(|_| "Invalid audio delay.".to_string())?
                .clamp(-10.0, 10.0);
            Ok(format!("audio_delay_set:{delay:.3}"))
        }
        value if value.starts_with("subtitle_delay_set:") => {
            let delay = value
                .trim_start_matches("subtitle_delay_set:")
                .parse::<f64>()
                .map_err(|_| "Invalid subtitle delay.".to_string())?
                .clamp(-10.0, 10.0);
            Ok(format!("subtitle_delay_set:{delay:.3}"))
        }
        value if value.starts_with("external_subtitle:") => {
            let target = value.trim_start_matches("external_subtitle:").trim();
            if target.is_empty() {
                return Err("Invalid external subtitle.".to_string());
            }
            Ok(format!("external_subtitle:{target}"))
        }
        _ => Err("Unknown playback command.".to_string()),
    }
}

pub(crate) fn forget_control(play_session_id: &str) {
    if let Some(paths) = CONTROL_PATHS.get() {
        if let Ok(mut paths) = paths.lock() {
            paths.remove(play_session_id);
        }
    }
    if let Some(paths) = PROGRESS_PATHS.get() {
        if let Ok(mut paths) = paths.lock() {
            paths.remove(play_session_id);
        }
    }
    if let Some(states) = PROGRESS_STATES.get() {
        if let Ok(mut states) = states.lock() {
            states.remove(play_session_id);
        }
    }
    if let Some(ids) = PROCESS_IDS.get() {
        if let Ok(mut ids) = ids.lock() {
            ids.remove(play_session_id);
        }
    }
    if let Some(stops) = EXPLICIT_STOPS.get() {
        if let Ok(mut stops) = stops.lock() {
            stops.remove(play_session_id);
        }
    }
}

fn remember_explicit_stop(play_session_id: &str) {
    if let Ok(mut stops) = EXPLICIT_STOPS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
    {
        stops.insert(play_session_id.to_string());
    }
}

pub(crate) fn take_explicit_stop(play_session_id: &str) -> bool {
    EXPLICIT_STOPS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .map(|mut stops| stops.remove(play_session_id))
        .unwrap_or(false)
}

pub(crate) fn stop_all() {
    let control_paths = CONTROL_PATHS
        .get()
        .and_then(|paths| {
            paths
                .lock()
                .ok()
                .map(|paths| paths.values().cloned().collect::<Vec<_>>())
        })
        .unwrap_or_default();

    for path in control_paths {
        let _ = fs::write(path, "stop");
    }

    clear_sessions();
}

pub(crate) fn terminate_all() -> bool {
    let process_ids = PROCESS_IDS
        .get()
        .and_then(|ids| {
            ids.lock()
                .ok()
                .map(|ids| ids.values().copied().collect::<Vec<_>>())
        })
        .unwrap_or_default();
    stop_all();
    let had_processes = !process_ids.is_empty();
    for process_id in process_ids {
        terminate_process(process_id);
    }
    had_processes
}

fn clear_sessions() {
    if let Some(paths) = CONTROL_PATHS.get() {
        if let Ok(mut paths) = paths.lock() {
            paths.clear();
        }
    }
    if let Some(paths) = PROGRESS_PATHS.get() {
        if let Ok(mut paths) = paths.lock() {
            paths.clear();
        }
    }
    if let Some(states) = PROGRESS_STATES.get() {
        if let Ok(mut states) = states.lock() {
            states.clear();
        }
    }
    if let Some(ids) = PROCESS_IDS.get() {
        if let Ok(mut ids) = ids.lock() {
            ids.clear();
        }
    }
}

pub(crate) fn restack_all(app: &AppHandle) {
    if let Some(ids) = PROCESS_IDS.get() {
        if let Ok(ids) = ids.lock() {
            for process_id in ids.values().copied() {
                place_embedded_video_behind_webview_once(app, process_id);
            }
        }
    }
}

pub(crate) fn remember_session(
    play_session_id: String,
    control_path: PathBuf,
    progress_path: PathBuf,
    process_id: u32,
) {
    if let Ok(mut paths) = CONTROL_PATHS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        paths.insert(play_session_id.clone(), control_path);
    }
    if let Ok(mut paths) = PROGRESS_PATHS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        paths.insert(play_session_id.clone(), progress_path);
    }
    if let Ok(mut ids) = PROCESS_IDS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        ids.insert(play_session_id, process_id);
    }
}

#[cfg(target_os = "windows")]
fn terminate_process(process_id: u32) {
    use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE | PROCESS_SYNCHRONIZE, 0, process_id);
        if !handle.is_null() {
            if WaitForSingleObject(handle, TERMINATE_GRACE_MS) != WAIT_OBJECT_0 {
                let _ = TerminateProcess(handle, 0);
            }
            let _ = CloseHandle(handle);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn terminate_process(_process_id: u32) {}

pub(crate) fn state(play_session_id: &str) -> Result<PlaybackStateResult, String> {
    if let Some(state) = cached_state(play_session_id)? {
        return Ok(state);
    }
    let path = PROGRESS_PATHS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "Failed to access playback state.".to_string())?
        .get(play_session_id)
        .cloned()
        .ok_or_else(|| "Playback session is not active.".to_string())?;
    fs::read_to_string(path)
        .map_err(|err| format!("Failed to read playback state: {err}"))
        .and_then(|raw| parse_state(&raw))
}

pub(crate) fn cache_state(play_session_id: &str, state: PlaybackStateResult) {
    if let Ok(mut states) = PROGRESS_STATES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        states.insert(play_session_id.to_string(), state);
    }
}

fn cached_state(play_session_id: &str) -> Result<Option<PlaybackStateResult>, String> {
    PROGRESS_STATES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "Failed to access playback state.".to_string())
        .map(|states| states.get(play_session_id).cloned())
}

fn parse_state(raw: &str) -> Result<PlaybackStateResult, String> {
    serde_json::from_str::<PlaybackStateResult>(raw)
        .map_err(|err| format!("Failed to parse playback state: {err}"))
}

fn mpv_start_arg(position_ticks: Option<i64>) -> Option<String> {
    let ticks = position_ticks.filter(|ticks| *ticks > 0)?;
    Some(format!("--start={:.3}", ticks as f64 / 10_000_000.0))
}

fn mpv_settings_args(settings: &AppSettings) -> Vec<String> {
    vec![format!(
        "--volume={}",
        settings.default_volume.clamp(0, 100)
    )]
}

fn mpv_subtitle_args(
    settings: &AppSettings,
    subtitle_track_position: Option<i32>,
    subtitle_url: Option<&str>,
) -> Vec<String> {
    let mut args = Vec::new();
    match subtitle_track_position {
        Some(position) if position < 0 => args.push("--sid=no".to_string()),
        Some(position) if position > 0 && subtitle_url.is_none() => {
            args.push(format!("--sid={position}"));
        }
        None if settings.subtitle_mode == "off" => args.push("--sid=no".to_string()),
        _ => {}
    }
    if let Some(url) = subtitle_url {
        args.push(format!("--sub-file={url}"));
    }
    args
}

fn seek_back_seconds(settings: &AppSettings) -> i32 {
    settings.seek_back_seconds.clamp(5, 60)
}

fn seek_forward_seconds(settings: &AppSettings) -> i32 {
    settings.seek_forward_seconds.clamp(5, 180)
}

fn mpv_log_path(app: &AppHandle, item_id: &str) -> Result<PathBuf, String> {
    Ok(app_data_path(app)?.join(format!("mpv-{}-{}.log", safe_id(item_id), unix_millis())))
}

fn mpv_progress_path(app: &AppHandle, item_id: &str) -> Result<PathBuf, String> {
    Ok(app_data_path(app)?.join(format!("mpv-{}-{}.json", safe_id(item_id), unix_millis())))
}

fn mpv_control_path(app: &AppHandle, item_id: &str) -> Result<PathBuf, String> {
    Ok(app_data_path(app)?.join(format!(
        "mpv-control-{}-{}.txt",
        safe_id(item_id),
        unix_millis()
    )))
}

fn mpv_progress_script_path(
    app: &AppHandle,
    item_id: &str,
    progress_path: &Path,
    control_path: &Path,
    seek_back_seconds: i32,
    seek_forward_seconds: i32,
) -> Result<PathBuf, String> {
    let script_path = app_data_path(app)?.join(format!(
        "mpv-progress-{}-{}.lua",
        safe_id(item_id),
        unix_millis()
    ));
    let progress_path = progress_path.display().to_string().replace('\\', "\\\\");
    let control_path = control_path.display().to_string().replace('\\', "\\\\");
    fs::write(
        &script_path,
        format!(
            r#"local progress_path = "{}"
local control_path = "{}"
local video_ready = false

local function write_progress()
  local file = io.open(progress_path, "w")
  if not file then return end
  file:write(string.format('{{"timePos":%.3f,"duration":%.3f,"paused":%s,"muted":%s,"volume":%.0f,"videoReady":%s,"speed":%.2f}}',
    mp.get_property_number("time-pos", 0),
    mp.get_property_number("duration", 0),
    tostring(mp.get_property_bool("pause", false)),
    tostring(mp.get_property_bool("mute", false)),
    mp.get_property_number("volume", 100),
    tostring(video_ready),
    mp.get_property_number("speed", 1)))
  file:close()
end

local function mark_video_ready()
  local width = mp.get_property_number("dwidth", 0)
  local height = mp.get_property_number("dheight", 0)
  if not video_ready and width > 0 and height > 0 then
    video_ready = true
    write_progress()
  end
end

local function read_control()
  local file = io.open(control_path, "r")
  if not file then return end
  local command = file:read("*a")
  file:close()
  os.remove(control_path)
  command = command and command:gsub("^%s+", ""):gsub("%s+$", "")
  if command == "toggle_pause" then
    mp.commandv("cycle", "pause")
  elseif command == "resume" then
    mp.set_property_bool("pause", false)
  elseif command == "seek_back" then
    mp.commandv("seek", "-{}", "relative")
  elseif command == "seek_forward" then
    mp.commandv("seek", "{}", "relative")
  elseif command and command:match("^seek_absolute:") then
    local target = tonumber(command:match("^seek_absolute:(.+)$"))
    if target then mp.commandv("seek", tostring(target), "absolute") end
  elseif command == "volume_down" then
    mp.commandv("add", "volume", "-5")
  elseif command == "volume_up" then
    mp.commandv("add", "volume", "5")
  elseif command and command:match("^volume_set:") then
    local volume = tonumber(command:match("^volume_set:(.+)$"))
    if volume then
      mp.set_property_bool("mute", volume <= 0)
      mp.set_property_number("volume", volume)
    end
  elseif command == "toggle_mute" then
    mp.commandv("cycle", "mute")
  elseif command == "audio_next" then
    mp.commandv("cycle", "audio")
  elseif command == "subtitle_next" then
    mp.commandv("cycle", "sub")
  elseif command and command:match("^audio_set:") then
    local track = tonumber(command:match("^audio_set:(.+)$"))
    if track then mp.set_property_number("aid", track) end
  elseif command and command:match("^subtitle_set:") then
    local track = tonumber(command:match("^subtitle_set:(.+)$"))
    if track and track < 0 then mp.set_property_string("sid", "no") elseif track then mp.set_property_number("sid", track) end
  elseif command == "speed_down" then
    local speed = math.max(0.5, mp.get_property_number("speed", 1) - 0.1)
    mp.set_property_number("speed", speed)
  elseif command == "speed_up" then
    local speed = math.min(2.0, mp.get_property_number("speed", 1) + 0.1)
    mp.set_property_number("speed", speed)
  elseif command and command:match("^speed_set:") then
    local speed = tonumber(command:match("^speed_set:(.+)$"))
    if speed then mp.set_property_number("speed", speed) end
  elseif command and command:match("^audio_delay_set:") then
    local delay = tonumber(command:match("^audio_delay_set:(.+)$"))
    if delay then mp.set_property_number("audio-delay", delay) end
  elseif command and command:match("^subtitle_delay_set:") then
    local delay = tonumber(command:match("^subtitle_delay_set:(.+)$"))
    if delay then mp.set_property_number("sub-delay", delay) end
  elseif command and command:match("^external_subtitle:") then
    local target = command:match("^external_subtitle:(.+)$")
    if target and #target > 0 then mp.commandv("sub-add", target, "select") end
  elseif command == "stop" then
    mp.commandv("quit")
  end
end

mp.add_periodic_timer(0.25, write_progress)
mp.add_periodic_timer(0.05, read_control)
mp.observe_property("dwidth", "number", mark_video_ready)
mp.observe_property("dheight", "number", mark_video_ready)
mp.register_event("shutdown", write_progress)
mp.register_event("file-loaded", mark_video_ready)
mp.register_event("video-reconfig", mark_video_ready)
write_progress()
"#,
            progress_path, control_path, seek_back_seconds, seek_forward_seconds
        ),
    )
    .map_err(|err| format!("Failed to write mpv progress script: {err}"))?;
    Ok(script_path)
}

#[cfg(target_os = "windows")]
fn add_embed_args(app: &AppHandle, command: &mut Command) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Failed to find the app window for embedded mpv.".to_string())?;
    let hwnd = window
        .hwnd()
        .map_err(|err| format!("Failed to get app window handle: {err}"))?;
    command
        .arg(format!("--wid={}", hwnd.0 as isize))
        .args(EMBED_MPV_WINDOW_ARGS);
    Ok(())
}

#[cfg(target_os = "windows")]
fn place_embedded_video_behind_webview(app: &AppHandle, process_id: u32) {
    let Some((parent, process_id)) = embedded_video_target(app, process_id) else {
        return;
    };
    thread::spawn(move || {
        for _ in 0..EMBED_INITIAL_RESTACKS {
            send_child_windows_to_bottom(parent, process_id);
            thread::sleep(EMBED_INITIAL_RESTACK_DELAY);
        }
    });
}

#[cfg(target_os = "windows")]
fn place_embedded_video_behind_webview_once(app: &AppHandle, process_id: u32) {
    if let Some((parent, process_id)) = embedded_video_target(app, process_id) {
        send_child_windows_to_bottom(parent, process_id);
    }
}

#[cfg(target_os = "windows")]
fn embedded_video_target(app: &AppHandle, process_id: u32) -> Option<(isize, u32)> {
    let Some(window) = app.get_webview_window("main") else {
        return None;
    };
    let Ok(hwnd) = window.hwnd() else {
        return None;
    };
    Some((hwnd.0 as isize, process_id))
}

#[cfg(target_os = "windows")]
fn send_child_windows_to_bottom(parent: isize, process_id: u32) {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, RECT};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumChildWindows, GetClientRect, GetWindowThreadProcessId, SetWindowPos, HWND_BOTTOM,
        SWP_NOACTIVATE,
    };

    struct Search {
        parent: HWND,
        process_id: u32,
    }

    unsafe extern "system" fn enum_child(hwnd: HWND, lparam: LPARAM) -> i32 {
        let search = &mut *(lparam as *mut Search);
        let mut owner = 0;
        GetWindowThreadProcessId(hwnd, &mut owner);
        if owner == search.process_id {
            let mut rect = RECT::default();
            GetClientRect(search.parent, &mut rect);
            SetWindowPos(
                hwnd,
                HWND_BOTTOM,
                0,
                0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                SWP_NOACTIVATE,
            );
        }
        1
    }

    let mut search = Search {
        parent: parent as HWND,
        process_id,
    };
    unsafe {
        EnumChildWindows(
            parent as HWND,
            Some(enum_child),
            &mut search as *mut Search as LPARAM,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn place_embedded_video_behind_webview(_app: &AppHandle, _process_id: u32) {}

#[cfg(not(target_os = "windows"))]
fn place_embedded_video_behind_webview_once(_app: &AppHandle, _process_id: u32) {}

#[cfg(not(target_os = "windows"))]
fn add_embed_args(_app: &AppHandle, _command: &mut Command) -> Result<(), String> {
    Err("Embedded mpv is currently only implemented on Windows.".to_string())
}

fn app_data_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("Failed to locate app data directory: {err}"))?;
    fs::create_dir_all(&dir)
        .map_err(|err| format!("Failed to create app data directory: {err}"))?;
    Ok(dir)
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn read_log_tail(path: &Path) -> Result<String, String> {
    let raw = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut lines = raw.lines().rev().take(40).collect::<Vec<_>>();
    lines.reverse();
    Ok(lines.join("\n"))
}

fn read_log_excerpt(path: &Path, secret: &str) -> String {
    read_log_tail(path)
        .map(|text| redact_secret(&text, secret))
        .unwrap_or_default()
}

fn redact_secret(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        text.to_string()
    } else {
        text.replace(secret, "***")
    }
}

fn find_mpv(app: &AppHandle, settings: &AppSettings) -> Result<PathBuf, String> {
    if let Some(path) = settings
        .mpv_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        let custom = PathBuf::from(path);
        if custom.is_file() {
            return Ok(custom);
        }
        return Err(format!(
            "Configured mpv was not found: {}",
            custom.display()
        ));
    }

    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("mpv").join("mpv.exe"));
        candidates.push(cwd.join("mpv.exe"));
        candidates.push(cwd.join("..").join("mpv").join("mpv.exe"));
        candidates.push(cwd.join("..").join("mpv.exe"));
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("mpv").join("mpv.exe"));
        candidates.push(resource_dir.join("mpv.exe"));
    }
    candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or_else(|| "mpv.exe was not found.".to_string())
}

fn find_mpv_config_dir(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("mpv");
        if is_dir(&bundled) {
            return Some(bundled);
        }
    }
    let cwd = std::env::current_dir().ok()?;
    [cwd.join("mpv"), cwd.join("..").join("mpv")]
        .into_iter()
        .find(|path| is_dir(path))
}

fn is_dir(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_dir())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_basic_zplayer_control_commands() {
        assert_eq!(normalize_command("toggle_pause").unwrap(), "toggle_pause");
        assert_eq!(normalize_command("volume_up").unwrap(), "volume_up");
        assert_eq!(normalize_command("subtitle_next").unwrap(), "subtitle_next");
        assert_eq!(normalize_command("speed_up").unwrap(), "speed_up");
        assert_eq!(normalize_command("resume").unwrap(), "resume");
        assert_eq!(normalize_command("stop").unwrap(), "stop");
    }

    #[test]
    fn normalizes_absolute_seek_command() {
        assert_eq!(
            normalize_command("seek_absolute:12.34567").unwrap(),
            "seek_absolute:12.346"
        );
        assert_eq!(
            normalize_command("seek_absolute:-4").unwrap(),
            "seek_absolute:0.000"
        );
    }

    #[test]
    fn normalizes_professional_playback_commands() {
        assert_eq!(
            normalize_command("speed_set:1.25").unwrap(),
            "speed_set:1.25"
        );
        assert_eq!(normalize_command("speed_set:9").unwrap(), "speed_set:2.00");
        assert_eq!(
            normalize_command("audio_delay_set:-0.250").unwrap(),
            "audio_delay_set:-0.250"
        );
        assert_eq!(
            normalize_command("subtitle_delay_set:0.500").unwrap(),
            "subtitle_delay_set:0.500"
        );
        assert_eq!(
            normalize_command("external_subtitle:C:/subs/movie.srt").unwrap(),
            "external_subtitle:C:/subs/movie.srt"
        );
    }

    #[test]
    fn rejects_unknown_control_command() {
        assert_eq!(
            normalize_command("playlist_next").unwrap_err(),
            "Unknown playback command."
        );
        assert_eq!(
            normalize_command("seek_absolute:nope").unwrap_err(),
            "Invalid seek target."
        );
    }

    #[test]
    fn parses_progress_written_by_mpv_script() {
        let state = parse_state(
            r#"{"timePos":12.5,"duration":120.0,"paused":false,"muted":true,"volume":42}"#,
        )
        .unwrap();

        assert_eq!(state.time_pos, Some(12.5));
        assert_eq!(state.duration, Some(120.0));
        assert!(!state.paused);
        assert!(state.muted);
        assert_eq!(state.volume, Some(42));
        assert!(!state.video_ready);
        let state = parse_state(
            r#"{"timePos":12.5,"duration":120.0,"paused":false,"muted":true,"volume":42,"videoReady":true}"#,
        )
        .unwrap();
        assert!(state.video_ready);
        let state = parse_state(
            r#"{"timePos":12.5,"duration":120.0,"paused":false,"muted":true,"volume":42,"videoReady":true,"speed":1.25}"#,
        )
        .unwrap();
        assert_eq!(state.speed, Some(1.25));
    }

    #[test]
    fn converts_resume_ticks_to_mpv_start_arg() {
        assert_eq!(
            mpv_start_arg(Some(90_500_000)),
            Some("--start=9.050".to_string())
        );
        assert_eq!(mpv_start_arg(Some(0)), None);
        assert_eq!(mpv_start_arg(None), None);
    }

    #[test]
    fn converts_app_settings_to_mpv_args() {
        let settings = AppSettings {
            default_volume: 65,
            subtitle_mode: "off".to_string(),
            ..Default::default()
        };

        let args = mpv_settings_args(&settings);

        assert!(args.contains(&"--volume=65".to_string()));
        assert!(!args.contains(&"--sid=no".to_string()));
        assert_eq!(
            mpv_subtitle_args(&settings, None, None),
            vec!["--sid=no".to_string()]
        );
    }

    #[test]
    fn converts_explicit_subtitle_selection_to_mpv_args() {
        let settings = AppSettings {
            subtitle_mode: "off".to_string(),
            ..Default::default()
        };

        assert_eq!(
            mpv_subtitle_args(&settings, Some(2), None),
            vec!["--sid=2".to_string()]
        );
        assert_eq!(
            mpv_subtitle_args(&settings, Some(-1), None),
            vec!["--sid=no".to_string()]
        );
        assert_eq!(
            mpv_subtitle_args(&settings, Some(1), Some("http://example.test/sub.srt")),
            vec!["--sub-file=http://example.test/sub.srt".to_string()]
        );
    }

    #[test]
    fn clamps_configured_seek_seconds() {
        let settings = AppSettings {
            seek_back_seconds: 1,
            seek_forward_seconds: 999,
            ..Default::default()
        };

        assert_eq!(seek_back_seconds(&settings), 5);
        assert_eq!(seek_forward_seconds(&settings), 180);
    }

    #[test]
    fn rejects_invalid_progress_json() {
        assert!(parse_state("not json").is_err());
    }

    #[test]
    fn returns_cached_progress_state() {
        let session_id = "cached-progress-test";
        let state = PlaybackStateResult {
            time_pos: Some(9.0),
            duration: Some(90.0),
            paused: false,
            muted: true,
            volume: Some(55),
            speed: Some(1.0),
            video_ready: true,
        };

        cache_state(session_id, state.clone());

        let cached = super::state(session_id).unwrap();
        assert_eq!(cached.time_pos, state.time_pos);
        assert_eq!(cached.duration, state.duration);
        assert_eq!(cached.paused, state.paused);
        assert_eq!(cached.muted, state.muted);
        assert_eq!(cached.volume, state.volume);
        assert_eq!(cached.video_ready, state.video_ready);
        forget_control(session_id);
    }

    #[test]
    fn forget_control_clears_cached_progress_state() {
        let session_id = "clear-cached-progress-test";
        cache_state(
            session_id,
            PlaybackStateResult {
                time_pos: Some(1.0),
                duration: Some(2.0),
                paused: false,
                muted: false,
                volume: Some(100),
                speed: Some(1.0),
                video_ready: false,
            },
        );

        forget_control(session_id);

        assert!(cached_state(session_id).unwrap().is_none());
    }

    #[test]
    fn terminate_all_reports_when_no_process_was_active() {
        clear_sessions();

        assert!(!terminate_all());
    }

    #[test]
    fn startup_check_is_short_enough_for_responsive_playback() {
        assert!(MPV_STARTUP_CHECK_MAX <= Duration::from_millis(300));
        assert!(MPV_STARTUP_CHECK_INTERVAL <= MPV_STARTUP_CHECK_MAX);
    }

    #[test]
    fn disables_mpv_builtin_controls() {
        for expected in [
            "--input-default-bindings=no",
            "--input-vo-keyboard=no",
            "--osc=no",
            "--osd-bar=no",
            "--osd-level=0",
        ] {
            assert!(DISABLE_MPV_UI_ARGS.contains(&expected));
        }
    }

    #[test]
    fn embeds_mpv_as_borderless_non_topmost_window() {
        for expected in ["--force-window=yes", "--no-border", "--ontop=no"] {
            assert!(EMBED_MPV_WINDOW_ARGS.contains(&expected));
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn limits_startup_restack_work() {
        assert!(EMBED_INITIAL_RESTACKS <= 40);
        assert!(EMBED_INITIAL_RESTACK_DELAY <= Duration::from_millis(25));
        assert!(TERMINATE_GRACE_MS <= 700);
    }

    #[test]
    fn runs_mpv_as_embedded_playback_engine() {
        for expected in ["--no-ytdl", "--keep-open=no"] {
            assert!(MPV_ENGINE_ARGS.contains(&expected));
        }
    }

    #[test]
    fn starts_mpv_paused_until_the_webview_is_ready() {
        assert!(MPV_SYNC_START_ARGS.contains(&"--pause=yes"));
    }

    #[test]
    fn avoids_verbose_mpv_log_writes_during_playback() {
        assert_eq!(MPV_LOG_LEVEL_ARG, "--msg-level=all=warn");
    }

    #[test]
    fn redacts_access_token_from_mpv_debug_text() {
        assert_eq!(
            redact_secret("https://server/video?api_key=secret-token", "secret-token"),
            "https://server/video?api_key=***"
        );
        assert_eq!(redact_secret("plain text", ""), "plain text");
    }
}
