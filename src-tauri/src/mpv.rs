use crate::models::{AppSettings, PlayResult, PlaybackStateResult, SavedServer};
use crate::store;
use libloading::Library;
use std::collections::{HashMap, HashSet};
use std::ffi::{CStr, CString};
use std::fs;
use std::os::raw::{c_char, c_double, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

type MpvCreate = unsafe extern "C" fn() -> *mut c_void;
type MpvInitialize = unsafe extern "C" fn(*mut c_void) -> c_int;
type MpvTerminateDestroy = unsafe extern "C" fn(*mut c_void);
type MpvCommand = unsafe extern "C" fn(*mut c_void, *const *const c_char) -> c_int;
type MpvSetOptionString = unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> c_int;
type MpvGetProperty = unsafe extern "C" fn(*mut c_void, *const c_char, c_int, *mut c_void) -> c_int;
type MpvWaitEvent = unsafe extern "C" fn(*mut c_void, c_double) -> *mut MpvEvent;
type MpvErrorString = unsafe extern "C" fn(c_int) -> *const c_char;
type MpvRenderContextCreate =
    unsafe extern "C" fn(*mut *mut c_void, *mut c_void, *mut MpvRenderParam) -> c_int;
type MpvRenderContextSetUpdateCallback =
    unsafe extern "C" fn(*mut c_void, Option<MpvRenderUpdateFn>, *mut c_void);
type MpvRenderContextUpdate = unsafe extern "C" fn(*mut c_void) -> u64;
type MpvRenderContextRender = unsafe extern "C" fn(*mut c_void, *mut MpvRenderParam) -> c_int;
type MpvRenderContextReportSwap = unsafe extern "C" fn(*mut c_void);
type MpvRenderContextFree = unsafe extern "C" fn(*mut c_void);
type MpvRenderUpdateFn = unsafe extern "C" fn(*mut c_void);
#[cfg(target_os = "linux")]
type MpvGlGetProcAddress = unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void;
#[cfg(target_os = "linux")]
type EpoxyGetProcAddress = unsafe extern "C" fn(*const c_char) -> *mut c_void;
#[cfg(target_os = "linux")]
type GlGetIntegerv = unsafe extern "C" fn(c_int, *mut c_int);

#[repr(C)]
struct MpvEvent {
    event_id: c_int,
    error: c_int,
    reply_userdata: u64,
    data: *mut c_void,
}

#[repr(C)]
struct MpvEventEndFile {
    reason: c_int,
    error: c_int,
    playlist_entry_id: i64,
    playlist_insert_id: i64,
    playlist_insert_num_entries: c_int,
}

#[repr(C)]
struct MpvRenderParam {
    param_type: c_int,
    data: *mut c_void,
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct MpvOpenGlInitParams {
    get_proc_address: Option<MpvGlGetProcAddress>,
    get_proc_address_ctx: *mut c_void,
}

#[cfg(target_os = "linux")]
#[repr(C)]
struct MpvOpenGlFbo {
    fbo: c_int,
    width: c_int,
    height: c_int,
    internal_format: c_int,
}

static SESSIONS: OnceLock<Mutex<HashMap<String, Arc<MpvSession>>>> = OnceLock::new();
static PROGRESS_STATES: OnceLock<Mutex<HashMap<String, PlaybackStateResult>>> = OnceLock::new();
static EXPLICIT_STOPS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
type ApiLoadResult = Result<Arc<MpvApi>, String>;
type ApiCache = Mutex<HashMap<PathBuf, ApiLoadResult>>;

static API_CACHE: OnceLock<ApiCache> = OnceLock::new();
#[cfg(target_os = "linux")]
static ACTIVE_RENDER_CONTEXT: OnceLock<Mutex<Option<Arc<MpvRenderContext>>>> = OnceLock::new();
#[cfg(target_os = "linux")]
static EPOXY_LIBRARY: OnceLock<Option<libloading::os::unix::Library>> = OnceLock::new();
#[cfg(target_os = "linux")]
static LAST_RENDER_FRAMEBUFFER: AtomicI32 = AtomicI32::new(0);
#[cfg(target_os = "linux")]
static LAST_RENDER_STATUS: AtomicI32 = AtomicI32::new(0);

const MPV_FORMAT_FLAG: c_int = 3;
const MPV_FORMAT_INT64: c_int = 4;
const MPV_FORMAT_DOUBLE: c_int = 5;
const MPV_EVENT_NONE: c_int = 0;
const MPV_EVENT_SHUTDOWN: c_int = 1;
const MPV_EVENT_END_FILE: c_int = 7;
const MPV_END_FILE_REASON_ERROR: c_int = 4;
const MPV_RENDER_PARAM_INVALID: c_int = 0;
const MPV_RENDER_PARAM_API_TYPE: c_int = 1;
const MPV_RENDER_PARAM_OPENGL_INIT_PARAMS: c_int = 2;
const MPV_RENDER_PARAM_OPENGL_FBO: c_int = 3;
const MPV_RENDER_PARAM_FLIP_Y: c_int = 4;
const MPV_RENDER_PARAM_WL_DISPLAY: c_int = 9;
const MPV_RENDER_UPDATE_FRAME: u64 = 1;
const MPV_RENDER_API_TYPE_OPENGL: &str = "opengl";
#[cfg(target_os = "linux")]
const GL_RGBA8: c_int = 0x8058;
#[cfg(target_os = "linux")]
const GL_FRAMEBUFFER_BINDING: c_int = 0x8CA6;
const MPV_CREATE_FAILURE_MESSAGE: &str =
    "Failed to create libmpv handle. libmpv returns NULL when allocation fails or LC_NUMERIC is not C.";

const DISABLE_MPV_UI_OPTIONS: &[(&str, &str)] = &[
    ("input-default-bindings", "no"),
    ("input-vo-keyboard", "no"),
    ("osd-bar", "no"),
    ("osd-level", "0"),
];
const MPV_ENGINE_OPTIONS: &[(&str, &str)] = &[("keep-open", "no")];
const MPV_ACCELERATION_OPTIONS: &[(&str, &str)] = &[("hwdec", "auto-safe")];
const MPV_SYNC_START_OPTIONS: &[(&str, &str)] = &[("pause", "yes")];
const MPV_LOG_LEVEL: &str = "all=warn";

pub(crate) struct Launch {
    pub(crate) result: PlayResult,
}

pub(crate) struct LaunchRequest<'a> {
    pub(crate) item_id: &'a str,
    pub(crate) media_source_id: Option<String>,
    pub(crate) play_session_id: String,
    pub(crate) stream_url: &'a str,
    pub(crate) subtitle_track_position: Option<i32>,
    pub(crate) subtitle_url: Option<&'a str>,
    pub(crate) start_position_ticks: Option<i64>,
}

struct SessionConfig<'a> {
    settings: &'a AppSettings,
    server: &'a SavedServer,
    log_path: &'a Path,
    subtitle_track_position: Option<i32>,
    subtitle_url: Option<&'a str>,
    start_position_ticks: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlaybackEnd {
    pub(crate) failed: bool,
}

struct MpvApi {
    _library: Library,
    #[cfg(target_os = "linux")]
    _sibling_libraries: Vec<Library>,
    library_path: PathBuf,
    mpv_create: MpvCreate,
    mpv_initialize: MpvInitialize,
    mpv_terminate_destroy: MpvTerminateDestroy,
    mpv_command: MpvCommand,
    mpv_set_option_string: MpvSetOptionString,
    mpv_get_property: MpvGetProperty,
    mpv_wait_event: MpvWaitEvent,
    mpv_error_string: MpvErrorString,
    mpv_render_context_create: MpvRenderContextCreate,
    mpv_render_context_set_update_callback: MpvRenderContextSetUpdateCallback,
    mpv_render_context_update: MpvRenderContextUpdate,
    mpv_render_context_render: MpvRenderContextRender,
    mpv_render_context_report_swap: MpvRenderContextReportSwap,
    mpv_render_context_free: MpvRenderContextFree,
}

impl MpvApi {
    unsafe fn load(path: &Path) -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        let sibling_libraries = preload_linux_sibling_libraries(path)?;
        #[cfg(target_os = "linux")]
        let library = open_linux_global_library(path)
            .map_err(|err| format!("Failed to load libmpv from {}: {err}", path.display()))?;
        #[cfg(not(target_os = "linux"))]
        let library = Library::new(path)
            .map_err(|err| format!("Failed to load libmpv from {}: {err}", path.display()))?;
        let mpv_create = *library
            .get::<MpvCreate>(b"mpv_create\0")
            .map_err(|err| format!("libmpv is missing mpv_create: {err}"))?;
        let mpv_initialize = *library
            .get::<MpvInitialize>(b"mpv_initialize\0")
            .map_err(|err| format!("libmpv is missing mpv_initialize: {err}"))?;
        let mpv_terminate_destroy = *library
            .get::<MpvTerminateDestroy>(b"mpv_terminate_destroy\0")
            .map_err(|err| format!("libmpv is missing mpv_terminate_destroy: {err}"))?;
        let mpv_command = *library
            .get::<MpvCommand>(b"mpv_command\0")
            .map_err(|err| format!("libmpv is missing mpv_command: {err}"))?;
        let mpv_set_option_string = *library
            .get::<MpvSetOptionString>(b"mpv_set_option_string\0")
            .map_err(|err| format!("libmpv is missing mpv_set_option_string: {err}"))?;
        let mpv_get_property = *library
            .get::<MpvGetProperty>(b"mpv_get_property\0")
            .map_err(|err| format!("libmpv is missing mpv_get_property: {err}"))?;
        let mpv_wait_event = *library
            .get::<MpvWaitEvent>(b"mpv_wait_event\0")
            .map_err(|err| format!("libmpv is missing mpv_wait_event: {err}"))?;
        let mpv_error_string = *library
            .get::<MpvErrorString>(b"mpv_error_string\0")
            .map_err(|err| format!("libmpv is missing mpv_error_string: {err}"))?;
        let mpv_render_context_create = *library
            .get::<MpvRenderContextCreate>(b"mpv_render_context_create\0")
            .map_err(|err| format!("libmpv is missing mpv_render_context_create: {err}"))?;
        let mpv_render_context_set_update_callback = *library
            .get::<MpvRenderContextSetUpdateCallback>(b"mpv_render_context_set_update_callback\0")
            .map_err(|err| {
                format!("libmpv is missing mpv_render_context_set_update_callback: {err}")
            })?;
        let mpv_render_context_update = *library
            .get::<MpvRenderContextUpdate>(b"mpv_render_context_update\0")
            .map_err(|err| format!("libmpv is missing mpv_render_context_update: {err}"))?;
        let mpv_render_context_render = *library
            .get::<MpvRenderContextRender>(b"mpv_render_context_render\0")
            .map_err(|err| format!("libmpv is missing mpv_render_context_render: {err}"))?;
        let mpv_render_context_report_swap = *library
            .get::<MpvRenderContextReportSwap>(b"mpv_render_context_report_swap\0")
            .map_err(|err| format!("libmpv is missing mpv_render_context_report_swap: {err}"))?;
        let mpv_render_context_free = *library
            .get::<MpvRenderContextFree>(b"mpv_render_context_free\0")
            .map_err(|err| format!("libmpv is missing mpv_render_context_free: {err}"))?;

        let api = Self {
            _library: library,
            #[cfg(target_os = "linux")]
            _sibling_libraries: sibling_libraries,
            library_path: path.to_path_buf(),
            mpv_create,
            mpv_initialize,
            mpv_terminate_destroy,
            mpv_command,
            mpv_set_option_string,
            mpv_get_property,
            mpv_wait_event,
            mpv_error_string,
            mpv_render_context_create,
            mpv_render_context_set_update_callback,
            mpv_render_context_update,
            mpv_render_context_render,
            mpv_render_context_report_swap,
            mpv_render_context_free,
        };
        debug_assert!(api.render_api_symbols_loaded());
        debug_assert_eq!(render_api_abi_values()[0], MPV_RENDER_PARAM_INVALID);
        debug_assert_eq!(MPV_RENDER_UPDATE_FRAME, 1);
        debug_assert_eq!(MPV_RENDER_API_TYPE_OPENGL, "opengl");
        Ok(api)
    }

    fn error_message(&self, code: c_int) -> String {
        unsafe {
            let message = (self.mpv_error_string)(code);
            if message.is_null() {
                format!("libmpv error {code}")
            } else {
                CStr::from_ptr(message).to_string_lossy().into_owned()
            }
        }
    }

    fn render_api_symbols_loaded(&self) -> bool {
        [
            self.mpv_render_context_create as usize,
            self.mpv_render_context_set_update_callback as usize,
            self.mpv_render_context_update as usize,
            self.mpv_render_context_render as usize,
            self.mpv_render_context_report_swap as usize,
            self.mpv_render_context_free as usize,
        ]
        .into_iter()
        .all(|address| address != 0)
    }
}

#[cfg(target_os = "linux")]
fn preload_linux_sibling_libraries(path: &Path) -> Result<Vec<Library>, String> {
    let Some(dir) = path.parent() else {
        return Ok(Vec::new());
    };
    if dir.as_os_str().is_empty() {
        return Ok(Vec::new());
    }
    let mut remaining = fs::read_dir(dir)
        .map_err(|err| format!("Failed to inspect bundled libmpv directory: {err}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|candidate| linux_sibling_library_candidate(candidate, path))
        .collect::<Vec<_>>();
    remaining.sort();

    let mut loaded = Vec::new();
    while !remaining.is_empty() {
        let mut next_remaining = Vec::new();
        let mut errors = Vec::new();
        let mut loaded_any = false;

        for candidate in remaining {
            match open_linux_global_library(&candidate) {
                Ok(library) => {
                    loaded.push(library);
                    loaded_any = true;
                }
                Err(err) => {
                    errors.push(format!("{}: {err}", candidate.display()));
                    next_remaining.push(candidate);
                }
            }
        }

        if !loaded_any {
            return Err(format!(
                "Failed to load bundled libmpv dependencies:\n{}",
                errors.join("\n")
            ));
        }
        remaining = next_remaining;
    }

    Ok(loaded)
}

#[cfg(target_os = "linux")]
fn linux_sibling_library_candidate(candidate: &Path, primary: &Path) -> bool {
    if candidate == primary || !candidate.is_file() {
        return false;
    }
    candidate
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(linux_sibling_library_name)
}

#[cfg(target_os = "linux")]
fn linux_sibling_library_name(name: &str) -> bool {
    (name.ends_with(".so") || name.contains(".so.")) && !name.starts_with("libmpv.so")
}

#[cfg(target_os = "linux")]
fn open_linux_global_library(path: &Path) -> Result<Library, libloading::Error> {
    let flags = libloading::os::unix::RTLD_NOW | libloading::os::unix::RTLD_GLOBAL;
    unsafe { libloading::os::unix::Library::open(Some(path), flags).map(Library::from) }
}

fn render_api_abi_values() -> [c_int; 6] {
    [
        MPV_RENDER_PARAM_INVALID,
        MPV_RENDER_PARAM_API_TYPE,
        MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
        MPV_RENDER_PARAM_OPENGL_FBO,
        MPV_RENDER_PARAM_FLIP_Y,
        MPV_RENDER_PARAM_WL_DISPLAY,
    ]
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn ensure_mpv_numeric_locale() -> Result<(), String> {
    let locale = b"C\0";
    let current = unsafe { libc::setlocale(libc::LC_NUMERIC, locale.as_ptr() as *const c_char) };
    if current.is_null() {
        Err("Failed to set LC_NUMERIC=C before creating libmpv handle.".to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn ensure_mpv_numeric_locale() -> Result<(), String> {
    Ok(())
}

struct MpvSession {
    api: Arc<MpvApi>,
    handle: *mut c_void,
    destroyed: AtomicBool,
    calls: Mutex<()>,
    #[cfg(target_os = "linux")]
    render_context: Mutex<Option<Arc<MpvRenderContext>>>,
    seek_back_seconds: i32,
    seek_forward_seconds: i32,
}

unsafe impl Send for MpvSession {}
unsafe impl Sync for MpvSession {}

#[cfg(target_os = "linux")]
struct MpvRenderContext {
    api: Arc<MpvApi>,
    context: usize,
    freed: AtomicBool,
}

#[cfg(target_os = "linux")]
unsafe impl Send for MpvRenderContext {}
#[cfg(target_os = "linux")]
unsafe impl Sync for MpvRenderContext {}

impl MpvSession {
    fn create(
        api: Arc<MpvApi>,
        seek_back_seconds: i32,
        seek_forward_seconds: i32,
    ) -> Result<Self, String> {
        ensure_mpv_numeric_locale()?;
        let handle = unsafe { (api.mpv_create)() };
        if handle.is_null() {
            return Err(format!(
                "{MPV_CREATE_FAILURE_MESSAGE} Loaded library: {}",
                api.library_path.display()
            ));
        }

        Ok(Self {
            api,
            handle,
            destroyed: AtomicBool::new(false),
            calls: Mutex::new(()),
            #[cfg(target_os = "linux")]
            render_context: Mutex::new(None),
            seek_back_seconds,
            seek_forward_seconds,
        })
    }

    fn set_option(&self, name: &str, value: &str) -> Result<(), String> {
        let option_name = name.to_string();
        let name = c_string(name, "Invalid libmpv option name")?;
        let value = c_string(value, "Invalid libmpv option value")?;
        let _guard = self
            .calls
            .lock()
            .map_err(|_| "Failed to lock libmpv session.".to_string())?;
        self.ensure_live()?;
        let status =
            unsafe { (self.api.mpv_set_option_string)(self.handle, name.as_ptr(), value.as_ptr()) };
        self.status(
            status,
            &format!("Failed to set libmpv option `{option_name}`"),
        )
    }

    fn initialize(&self) -> Result<(), String> {
        let _guard = self
            .calls
            .lock()
            .map_err(|_| "Failed to lock libmpv session.".to_string())?;
        self.ensure_live()?;
        let status = unsafe { (self.api.mpv_initialize)(self.handle) };
        self.status(status, "Failed to initialize libmpv")
    }

    #[cfg(target_os = "linux")]
    fn initialize_render_context(self: &Arc<Self>, app: &AppHandle) -> Result<(), String> {
        let window = app
            .get_webview_window("main")
            .ok_or_else(|| "Failed to find the app window for embedded libmpv.".to_string())?;
        let session = self.clone();
        let (tx, rx) = mpsc::channel();

        let render_window = window.clone();
        window
            .run_on_main_thread(move || {
                let result = crate::platform_window::ensure_native_video_overlay(&render_window)
                    .map_err(|err| format!("Failed to install Linux native video layer: {err}"))
                    .and_then(|_| {
                        crate::platform_window::with_native_video_area(|area| {
                            use gtk::prelude::*;

                            area.make_current();
                            area.attach_buffers();
                            let wayland_display = wayland_display_for_gl_area(area)?;
                            let context =
                                session.create_render_context_on_current_thread(wayland_display)?;
                            session.set_render_context_on_current_thread(context)?;
                            area.queue_render();
                            Ok(())
                        })
                        .unwrap_or_else(|| {
                            Err("Linux native video layer is not available.".to_string())
                        })
                    });
                let _ = tx.send(result);
            })
            .map_err(|err| format!("Failed to initialize libmpv render thread: {err}"))?;

        rx.recv_timeout(Duration::from_secs(5))
            .map_err(|err| match err {
                mpsc::RecvTimeoutError::Timeout => {
                    "Timed out while initializing libmpv OpenGL render context.".to_string()
                }
                mpsc::RecvTimeoutError::Disconnected => {
                    "Failed to receive libmpv render initialization result.".to_string()
                }
            })?
    }

    #[cfg(target_os = "linux")]
    fn create_render_context_on_current_thread(
        &self,
        wayland_display: *mut c_void,
    ) -> Result<Arc<MpvRenderContext>, String> {
        let _guard = self
            .calls
            .lock()
            .map_err(|_| "Failed to lock libmpv session.".to_string())?;
        self.ensure_live()?;

        let api_type = c_string(MPV_RENDER_API_TYPE_OPENGL, "Invalid libmpv render API type")?;
        let mut init_params = MpvOpenGlInitParams {
            get_proc_address: Some(mpv_gl_get_proc_address),
            get_proc_address_ctx: ptr::null_mut(),
        };
        let mut context = ptr::null_mut();
        let mut params = [
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_API_TYPE,
                data: api_type.as_ptr() as *mut c_void,
            },
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
                data: &mut init_params as *mut MpvOpenGlInitParams as *mut c_void,
            },
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_WL_DISPLAY,
                data: wayland_display,
            },
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];

        let status = unsafe {
            (self.api.mpv_render_context_create)(&mut context, self.handle, params.as_mut_ptr())
        };
        self.status(status, "Failed to initialize libmpv OpenGL render context")?;
        if context.is_null() {
            return Err("libmpv returned an empty render context.".to_string());
        }

        let context = Arc::new(MpvRenderContext {
            api: self.api.clone(),
            context: context as usize,
            freed: AtomicBool::new(false),
        });
        unsafe {
            (context.api.mpv_render_context_set_update_callback)(
                context.as_ptr(),
                Some(mpv_render_update_callback),
                ptr::null_mut(),
            );
        }
        Ok(context)
    }

    #[cfg(target_os = "linux")]
    fn set_render_context_on_current_thread(
        &self,
        context: Arc<MpvRenderContext>,
    ) -> Result<(), String> {
        let mut stored = self
            .render_context
            .lock()
            .map_err(|_| "Failed to lock libmpv render context.".to_string())?;
        if let Some(old) = stored.replace(context.clone()) {
            clear_active_render_context(old.as_ptr());
            old.free_on_current_thread();
        }
        set_active_render_context(context);
        Ok(())
    }

    fn command(&self, args: &[&str]) -> Result<(), String> {
        let cstrings = args
            .iter()
            .map(|arg| c_string(arg, "Invalid libmpv command argument"))
            .collect::<Result<Vec<_>, _>>()?;
        let mut pointers = cstrings
            .iter()
            .map(|arg| arg.as_ptr())
            .collect::<Vec<*const c_char>>();
        pointers.push(ptr::null());

        let _guard = self
            .calls
            .lock()
            .map_err(|_| "Failed to lock libmpv session.".to_string())?;
        self.ensure_live()?;
        let status = unsafe { (self.api.mpv_command)(self.handle, pointers.as_ptr()) };
        self.status(status, "Failed to send libmpv command")
    }

    fn state(&self) -> Result<PlaybackStateResult, String> {
        let _guard = self
            .calls
            .lock()
            .map_err(|_| "Failed to lock libmpv session.".to_string())?;
        self.ensure_live()?;

        let time_pos = self.get_double_locked("time-pos");
        let duration = self.get_double_locked("duration");
        let paused = self.get_flag_locked("pause").unwrap_or(false);
        let muted = self.get_flag_locked("mute").unwrap_or(false);
        let volume = self
            .get_double_locked("volume")
            .map(|volume| volume.round().clamp(0.0, 100.0) as i32);
        let speed = self.get_double_locked("speed");
        let cache_speed = self
            .get_double_locked("cache-speed")
            .filter(|speed| speed.is_finite() && *speed >= 0.0);
        let video_ready = self
            .get_i64_locked("dwidth")
            .zip(self.get_i64_locked("dheight"))
            .map(|(width, height)| width > 0 && height > 0)
            .unwrap_or(false);

        Ok(PlaybackStateResult {
            time_pos,
            duration,
            paused,
            muted,
            volume,
            speed,
            cache_speed,
            video_ready,
        })
    }

    fn poll_end(&self, timeout_seconds: f64) -> Result<Option<PlaybackEnd>, String> {
        let _guard = self
            .calls
            .lock()
            .map_err(|_| "Failed to lock libmpv session.".to_string())?;
        self.ensure_live()?;

        let mut timeout = timeout_seconds.max(0.0);
        loop {
            let event = unsafe { (self.api.mpv_wait_event)(self.handle, timeout) };
            if event.is_null() {
                return Ok(None);
            }
            let event = unsafe { &*event };
            match event.event_id {
                MPV_EVENT_NONE => return Ok(None),
                MPV_EVENT_SHUTDOWN => return Ok(Some(PlaybackEnd { failed: false })),
                MPV_EVENT_END_FILE => {
                    let failed = if event.data.is_null() {
                        event.error < 0
                    } else {
                        let end_file = unsafe { &*(event.data as *const MpvEventEndFile) };
                        event.error < 0
                            || end_file.error < 0
                            || end_file.reason == MPV_END_FILE_REASON_ERROR
                    };
                    return Ok(Some(PlaybackEnd { failed }));
                }
                _ => timeout = 0.0,
            }
        }
    }

    fn destroy(&self) {
        let Ok(_guard) = self.calls.lock() else {
            return;
        };
        if !self.destroyed.swap(true, Ordering::SeqCst) {
            #[cfg(target_os = "linux")]
            if !self.free_render_context_locked() {
                return;
            }
            unsafe {
                (self.api.mpv_terminate_destroy)(self.handle);
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn free_render_context_locked(&self) -> bool {
        let context = self
            .render_context
            .lock()
            .ok()
            .and_then(|mut context| context.take());
        if let Some(context) = context {
            clear_active_render_context(context.as_ptr());
            context.free_on_native_video_thread()
        } else {
            true
        }
    }

    fn ensure_live(&self) -> Result<(), String> {
        if self.destroyed.load(Ordering::SeqCst) {
            Err("Playback session is not active.".to_string())
        } else {
            Ok(())
        }
    }

    fn status(&self, status: c_int, context: &str) -> Result<(), String> {
        if status < 0 {
            Err(format!("{context}: {}", self.api.error_message(status)))
        } else {
            Ok(())
        }
    }

    fn get_double_locked(&self, name: &str) -> Option<f64> {
        let name = CString::new(name).ok()?;
        let mut value = 0.0;
        let status = unsafe {
            (self.api.mpv_get_property)(
                self.handle,
                name.as_ptr(),
                MPV_FORMAT_DOUBLE,
                &mut value as *mut f64 as *mut c_void,
            )
        };
        (status >= 0).then_some(value)
    }

    fn get_flag_locked(&self, name: &str) -> Option<bool> {
        let name = CString::new(name).ok()?;
        let mut value: c_int = 0;
        let status = unsafe {
            (self.api.mpv_get_property)(
                self.handle,
                name.as_ptr(),
                MPV_FORMAT_FLAG,
                &mut value as *mut c_int as *mut c_void,
            )
        };
        (status >= 0).then_some(value != 0)
    }

    fn get_i64_locked(&self, name: &str) -> Option<i64> {
        let name = CString::new(name).ok()?;
        let mut value: i64 = 0;
        let status = unsafe {
            (self.api.mpv_get_property)(
                self.handle,
                name.as_ptr(),
                MPV_FORMAT_INT64,
                &mut value as *mut i64 as *mut c_void,
            )
        };
        (status >= 0).then_some(value)
    }
}

impl Drop for MpvSession {
    fn drop(&mut self) {
        self.destroy();
    }
}

#[cfg(target_os = "linux")]
impl MpvRenderContext {
    fn as_ptr(&self) -> *mut c_void {
        self.context as *mut c_void
    }

    fn render_gl_area(&self, area: &gtk::GLArea) {
        if self.freed.load(Ordering::SeqCst) {
            return;
        }

        use gtk::prelude::*;

        area.make_current();
        area.attach_buffers();
        let (width, height) = scaled_gl_area_size(
            area.allocated_width(),
            area.allocated_height(),
            area.scale_factor(),
        );
        let framebuffer = current_gl_framebuffer();
        LAST_RENDER_FRAMEBUFFER.store(framebuffer, Ordering::SeqCst);
        let mut fbo = MpvOpenGlFbo {
            fbo: framebuffer,
            width,
            height,
            internal_format: GL_RGBA8,
        };
        let mut flip_y: c_int = 1;
        let mut params = [
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_OPENGL_FBO,
                data: &mut fbo as *mut MpvOpenGlFbo as *mut c_void,
            },
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_FLIP_Y,
                data: &mut flip_y as *mut c_int as *mut c_void,
            },
            MpvRenderParam {
                param_type: MPV_RENDER_PARAM_INVALID,
                data: ptr::null_mut(),
            },
        ];

        unsafe {
            let _ = (self.api.mpv_render_context_update)(self.as_ptr());
            let status = (self.api.mpv_render_context_render)(self.as_ptr(), params.as_mut_ptr());
            LAST_RENDER_STATUS.store(status, Ordering::SeqCst);
            if status >= 0 {
                (self.api.mpv_render_context_report_swap)(self.as_ptr());
            }
        }
    }

    fn free_on_native_video_thread(self: Arc<Self>) -> bool {
        if self.freed.load(Ordering::SeqCst) {
            return true;
        }

        if crate::platform_window::native_video_thread_is_current() {
            return self.free_with_native_video_area();
        }

        let (tx, rx) = mpsc::channel();
        gtk::glib::MainContext::default().invoke(move || {
            let freed = self.free_with_native_video_area();
            let _ = tx.send(freed);
        });
        rx.recv_timeout(Duration::from_secs(5)).unwrap_or(false)
    }

    fn free_with_native_video_area(&self) -> bool {
        crate::platform_window::with_native_video_area(|area| {
            use gtk::prelude::*;

            area.make_current();
            self.free_on_current_thread()
        })
        .unwrap_or(false)
    }

    fn free_on_current_thread(&self) -> bool {
        if self.freed.swap(true, Ordering::SeqCst) {
            return true;
        }

        unsafe {
            (self.api.mpv_render_context_set_update_callback)(self.as_ptr(), None, ptr::null_mut());
            (self.api.mpv_render_context_free)(self.as_ptr());
        }
        true
    }
}

pub(crate) fn launch(
    app: &AppHandle,
    server: &SavedServer,
    request: LaunchRequest<'_>,
) -> Result<Launch, String> {
    let LaunchRequest {
        item_id,
        media_source_id,
        play_session_id,
        stream_url,
        subtitle_track_position,
        subtitle_url,
        start_position_ticks,
    } = request;
    let redacted_url = redact_secret(stream_url, &server.access_token);
    let settings = store::settings(app)?;
    let log_path = mpv_log_path(app, item_id)?;
    let api = load_libmpv_api(app, &settings)?;
    let session = Arc::new(MpvSession::create(
        api,
        seek_back_seconds(&settings),
        seek_forward_seconds(&settings),
    )?);

    configure_session(
        app,
        &session,
        SessionConfig {
            settings: &settings,
            server,
            log_path: &log_path,
            subtitle_track_position,
            subtitle_url,
            start_position_ticks,
        },
    )?;
    session.initialize()?;
    #[cfg(target_os = "linux")]
    session.initialize_render_context(app)?;
    session.command(&["loadfile", stream_url, "replace"])?;
    if let Some(url) = subtitle_url {
        let _ = session.command(&["sub-add", url, "select"]);
    }

    remember_session(play_session_id.clone(), session);

    Ok(Launch {
        result: PlayResult {
            item_id: item_id.to_string(),
            server_id: server.id.clone(),
            server_name: server.name.clone(),
            media_source_id,
            play_session_id,
            url: redacted_url,
            log_path: log_path.display().to_string(),
            log_tail: read_log_excerpt(&log_path, &server.access_token),
        },
    })
}

fn configure_session(
    app: &AppHandle,
    session: &MpvSession,
    config: SessionConfig<'_>,
) -> Result<(), String> {
    session.set_option("config", "yes")?;
    if let Some(config_dir) = find_libmpv_config_dir(app) {
        session.set_option("config-dir", &config_dir.display().to_string())?;
    }
    session.set_option("log-file", &config.log_path.display().to_string())?;
    session.set_option("msg-level", MPV_LOG_LEVEL)?;
    session.set_option(
        "http-header-fields",
        &format!("X-Emby-Token: {}", config.server.access_token),
    )?;
    for (name, value) in DISABLE_MPV_UI_OPTIONS {
        session.set_option(name, value)?;
    }
    for (name, value) in MPV_ENGINE_OPTIONS {
        session.set_option(name, value)?;
    }
    for (name, value) in MPV_ACCELERATION_OPTIONS {
        session.set_option(name, value)?;
    }
    for (name, value) in MPV_SYNC_START_OPTIONS {
        session.set_option(name, value)?;
    }
    for (name, value) in mpv_settings_options(config.settings) {
        session.set_option(&name, &value)?;
    }
    for (name, value) in mpv_subtitle_options(
        config.settings,
        config.subtitle_track_position,
        config.subtitle_url,
    ) {
        session.set_option(&name, &value)?;
    }
    if let Some(start) = mpv_start_value(config.start_position_ticks) {
        session.set_option("start", &start)?;
    }
    add_embed_options(app, session)?;
    Ok(())
}

pub(crate) fn control(play_session_id: &str, command: &str) -> Result<(), String> {
    let session = active_session(play_session_id)?;
    let command = normalize_command(command)?;
    if command == "stop" {
        remember_explicit_stop(play_session_id);
    }
    execute_control(&session, &command)
}

fn execute_control(session: &MpvSession, command: &str) -> Result<(), String> {
    match command {
        "toggle_pause" => session.command(&["cycle", "pause"]),
        "seek_back" => session.command(&[
            "seek",
            &format!("-{}", session.seek_back_seconds),
            "relative+exact",
        ]),
        "seek_forward" => session.command(&[
            "seek",
            &session.seek_forward_seconds.to_string(),
            "relative+exact",
        ]),
        "volume_down" => session.command(&["add", "volume", "-5"]),
        "volume_up" => session.command(&["add", "volume", "5"]),
        "toggle_mute" => session.command(&["cycle", "mute"]),
        "audio_next" => session.command(&["cycle", "audio"]),
        "subtitle_next" => session.command(&["cycle", "sub"]),
        "speed_down" => session.command(&["add", "speed", "-0.1"]),
        "speed_up" => session.command(&["add", "speed", "0.1"]),
        "resume" => session.command(&["set", "pause", "no"]),
        "stop" => session.command(&["quit"]),
        value if value.starts_with("seek_absolute:") => {
            let seconds = value.trim_start_matches("seek_absolute:");
            session.command(&["seek", seconds, "absolute+exact"])
        }
        value if value.starts_with("audio_set:") => {
            let index = value.trim_start_matches("audio_set:");
            session.command(&["set", "aid", index])
        }
        value if value.starts_with("subtitle_set:") => {
            let index = value.trim_start_matches("subtitle_set:");
            let target = if index.starts_with('-') { "no" } else { index };
            session.command(&["set", "sid", target])
        }
        value if value.starts_with("volume_set:") => {
            let volume = value.trim_start_matches("volume_set:");
            session.command(&["set", "volume", volume])
        }
        value if value.starts_with("speed_set:") => {
            let speed = value.trim_start_matches("speed_set:");
            session.command(&["set", "speed", speed])
        }
        value if value.starts_with("audio_delay_set:") => {
            let delay = value.trim_start_matches("audio_delay_set:");
            session.command(&["set", "audio-delay", delay])
        }
        value if value.starts_with("subtitle_delay_set:") => {
            let delay = value.trim_start_matches("subtitle_delay_set:");
            session.command(&["set", "sub-delay", delay])
        }
        value if value.starts_with("external_subtitle:") => {
            let target = value.trim_start_matches("external_subtitle:");
            session.command(&["sub-add", target, "select"])
        }
        _ => Err("Unknown playback command.".to_string()),
    }
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
            if !is_remote_subtitle(target) && !Path::new(target).is_file() {
                return Err("External subtitle was not found.".to_string());
            }
            Ok(format!("external_subtitle:{target}"))
        }
        _ => Err("Unknown playback command.".to_string()),
    }
}

fn is_remote_subtitle(target: &str) -> bool {
    target.starts_with("http://") || target.starts_with("https://")
}

pub(crate) fn forget_control(play_session_id: &str) {
    if let Some(session) = remove_session(play_session_id) {
        session.destroy();
    }
    if let Some(states) = PROGRESS_STATES.get() {
        if let Ok(mut states) = states.lock() {
            states.remove(play_session_id);
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

pub(crate) fn terminate_all() -> bool {
    let sessions = active_sessions();
    let had_sessions = !sessions.is_empty();
    for session in &sessions {
        let _ = session.command(&["quit"]);
    }
    clear_sessions();
    for session in sessions {
        session.destroy();
    }
    had_sessions
}

fn clear_sessions() {
    if let Some(sessions) = SESSIONS.get() {
        if let Ok(mut sessions) = sessions.lock() {
            sessions.clear();
        }
    }
    if let Some(states) = PROGRESS_STATES.get() {
        if let Ok(mut states) = states.lock() {
            states.clear();
        }
    }
}

pub(crate) fn restack_all(_app: &AppHandle) {}

#[cfg(target_os = "linux")]
pub(crate) fn render_native_video(area: &gtk::GLArea) -> gtk::glib::Propagation {
    if let Some(context) = active_render_context() {
        context.render_gl_area(area);
    }
    gtk::glib::Propagation::Stop
}

#[cfg(target_os = "linux")]
pub(crate) fn native_video_render_context_active() -> bool {
    active_render_context().is_some()
}

#[cfg(target_os = "linux")]
fn active_render_context() -> Option<Arc<MpvRenderContext>> {
    ACTIVE_RENDER_CONTEXT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .ok()
        .and_then(|context| context.clone())
}

#[cfg(target_os = "linux")]
fn set_active_render_context(context: Arc<MpvRenderContext>) {
    if let Ok(mut active) = ACTIVE_RENDER_CONTEXT
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        *active = Some(context);
    }
    crate::platform_window::queue_native_video_render();
}

#[cfg(target_os = "linux")]
fn clear_active_render_context(context: *mut c_void) {
    if let Ok(mut active) = ACTIVE_RENDER_CONTEXT
        .get_or_init(|| Mutex::new(None))
        .lock()
    {
        let should_clear = active
            .as_ref()
            .map(|active| active.as_ptr() == context)
            .unwrap_or(false);
        if should_clear {
            *active = None;
        }
    }
    crate::platform_window::queue_native_video_render();
}

#[cfg(target_os = "linux")]
fn wayland_display_for_gl_area(area: &gtk::GLArea) -> Result<*mut c_void, String> {
    use gtk::gdk::prelude::*;
    use gtk::glib::object::ObjectType;
    use gtk::prelude::*;

    let display = area.display();
    if !display.backend().is_wayland() {
        return Err("Linux playback requires a Wayland GDK display.".to_string());
    }

    let wayland_display = unsafe {
        gdk_wayland_sys::gdk_wayland_display_get_wl_display(
            display.as_ptr() as *mut gdk_wayland_sys::GdkWaylandDisplay
        )
    } as *mut c_void;
    if wayland_display.is_null() {
        Err("Failed to get the Wayland display for libmpv rendering.".to_string())
    } else {
        Ok(wayland_display)
    }
}

#[cfg(target_os = "linux")]
unsafe extern "C" fn mpv_render_update_callback(_context: *mut c_void) {
    crate::platform_window::queue_native_video_render();
}

#[cfg(target_os = "linux")]
fn scaled_gl_area_size(width: i32, height: i32, scale_factor: i32) -> (c_int, c_int) {
    let scale_factor = scale_factor.max(1);
    (
        width.saturating_mul(scale_factor).max(1),
        height.saturating_mul(scale_factor).max(1),
    )
}

#[cfg(target_os = "linux")]
fn current_gl_framebuffer() -> c_int {
    let address = unsafe { mpv_gl_get_proc_address(ptr::null_mut(), c"glGetIntegerv".as_ptr()) };
    if address.is_null() {
        return 0;
    }

    let gl_get_integerv = unsafe { std::mem::transmute::<*mut c_void, GlGetIntegerv>(address) };
    let mut framebuffer = 0;
    unsafe {
        gl_get_integerv(GL_FRAMEBUFFER_BINDING, &mut framebuffer);
    }
    framebuffer
}

#[cfg(target_os = "linux")]
pub(crate) fn native_video_render_framebuffer() -> i32 {
    LAST_RENDER_FRAMEBUFFER.load(Ordering::SeqCst)
}

#[cfg(target_os = "linux")]
pub(crate) fn native_video_render_status() -> i32 {
    LAST_RENDER_STATUS.load(Ordering::SeqCst)
}

#[cfg(target_os = "linux")]
unsafe extern "C" fn mpv_gl_get_proc_address(
    _context: *mut c_void,
    name: *const c_char,
) -> *mut c_void {
    if name.is_null() {
        return ptr::null_mut();
    }

    if let Some(address) = epoxy_get_proc_address(name) {
        return address;
    }

    process_symbol_address(name).unwrap_or(ptr::null_mut())
}

#[cfg(target_os = "linux")]
fn epoxy_get_proc_address(name: *const c_char) -> Option<*mut c_void> {
    let library = EPOXY_LIBRARY
        .get_or_init(|| unsafe { libloading::os::unix::Library::new("libepoxy.so.0").ok() })
        .as_ref()?;
    let loader = unsafe {
        library
            .get::<EpoxyGetProcAddress>(b"epoxy_get_proc_address\0")
            .ok()?
    };
    let address = unsafe { loader(name) };
    (!address.is_null()).then_some(address)
}

#[cfg(target_os = "linux")]
fn process_symbol_address(name: *const c_char) -> Option<*mut c_void> {
    let symbol_name = unsafe { CStr::from_ptr(name).to_bytes_with_nul() };
    let library = libloading::os::unix::Library::this();
    let symbol = unsafe { library.get::<unsafe extern "C" fn()>(symbol_name).ok()? };
    Some(*symbol as *mut c_void)
}

fn remember_session(play_session_id: String, session: Arc<MpvSession>) {
    if let Ok(mut sessions) = SESSIONS.get_or_init(|| Mutex::new(HashMap::new())).lock() {
        sessions.insert(play_session_id, session);
    }
}

fn active_session(play_session_id: &str) -> Result<Arc<MpvSession>, String> {
    SESSIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "Failed to access playback sessions.".to_string())?
        .get(play_session_id)
        .cloned()
        .ok_or_else(|| "Playback session is not active.".to_string())
}

fn active_sessions() -> Vec<Arc<MpvSession>> {
    SESSIONS
        .get()
        .and_then(|sessions| {
            sessions
                .lock()
                .ok()
                .map(|sessions| sessions.values().cloned().collect::<Vec<_>>())
        })
        .unwrap_or_default()
}

fn remove_session(play_session_id: &str) -> Option<Arc<MpvSession>> {
    SESSIONS
        .get()
        .and_then(|sessions| sessions.lock().ok()?.remove(play_session_id))
}

pub(crate) fn refresh_state(play_session_id: &str) -> Result<PlaybackStateResult, String> {
    let state = active_session(play_session_id)?.state()?;
    cache_state(play_session_id, state.clone());
    Ok(state)
}

#[cfg(test)]
pub(crate) fn state(play_session_id: &str) -> Result<PlaybackStateResult, String> {
    if let Some(state) = cached_state(play_session_id)? {
        return Ok(state);
    }
    let session = active_session(play_session_id)?;
    session.state()
}

pub(crate) fn poll_playback_end(play_session_id: &str) -> Result<Option<PlaybackEnd>, String> {
    active_session(play_session_id)?.poll_end(0.0)
}

pub(crate) fn cache_state(play_session_id: &str, state: PlaybackStateResult) {
    if let Ok(mut states) = PROGRESS_STATES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        states.insert(play_session_id.to_string(), state);
    }
}

#[cfg(test)]
fn cached_state(play_session_id: &str) -> Result<Option<PlaybackStateResult>, String> {
    PROGRESS_STATES
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "Failed to access playback state.".to_string())
        .map(|states| states.get(play_session_id).cloned())
}

#[cfg(test)]
fn parse_state(raw: &str) -> Result<PlaybackStateResult, String> {
    serde_json::from_str::<PlaybackStateResult>(raw)
        .map_err(|err| format!("Failed to parse playback state: {err}"))
}

fn mpv_start_value(position_ticks: Option<i64>) -> Option<String> {
    let ticks = position_ticks.filter(|ticks| *ticks > 0)?;
    Some(format!("{:.3}", ticks as f64 / 10_000_000.0))
}

fn mpv_settings_options(settings: &AppSettings) -> Vec<(String, String)> {
    vec![(
        "volume".to_string(),
        settings.default_volume.clamp(0, 100).to_string(),
    )]
}

fn mpv_subtitle_options(
    settings: &AppSettings,
    subtitle_track_position: Option<i32>,
    subtitle_url: Option<&str>,
) -> Vec<(String, String)> {
    let mut options = Vec::new();
    match subtitle_track_position {
        Some(position) if position < 0 => options.push(("sid".to_string(), "no".to_string())),
        Some(position) if position > 0 && subtitle_url.is_none() => {
            options.push(("sid".to_string(), position.to_string()));
        }
        None if settings.subtitle_mode == "off" && subtitle_url.is_none() => {
            options.push(("sid".to_string(), "no".to_string()));
        }
        _ => {}
    }
    options
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

#[cfg(target_os = "windows")]
fn add_embed_options(app: &AppHandle, session: &MpvSession) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Failed to find the app window for embedded libmpv.".to_string())?;
    let hwnd = window
        .hwnd()
        .map_err(|err| format!("Failed to get app window handle: {err}"))?;
    session.set_option("wid", &(hwnd.0 as isize).to_string())?;
    session.set_option("force-window", "yes")?;
    session.set_option("border", "no")?;
    session.set_option("ontop", "no")
}

#[cfg(target_os = "linux")]
fn add_embed_options(_app: &AppHandle, session: &MpvSession) -> Result<(), String> {
    for (name, value) in LINUX_RENDER_API_OPTIONS {
        session.set_option(name, value)?;
    }
    Ok(())
}

#[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
fn add_embed_options(_app: &AppHandle, session: &MpvSession) -> Result<(), String> {
    session.set_option("force-window", "yes")
}

#[cfg(target_os = "linux")]
const LINUX_RENDER_API_OPTIONS: &[(&str, &str)] = &[("vo", "libmpv"), ("force-window", "no")];

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

fn c_string(value: &str, context: &str) -> Result<CString, String> {
    CString::new(value).map_err(|_| format!("{context}: value contains an embedded NUL byte."))
}

fn load_api(path: &Path) -> Result<Arc<MpvApi>, String> {
    let mut cache = API_CACHE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "Failed to access libmpv loader cache.".to_string())?;
    if let Some(cached) = cache.get(path) {
        return cached.clone();
    }

    let loaded = unsafe { MpvApi::load(path).map(Arc::new) };
    cache.insert(path.to_path_buf(), loaded.clone());
    loaded
}

fn load_libmpv_api(app: &AppHandle, settings: &AppSettings) -> Result<Arc<MpvApi>, String> {
    if let Some(path) = configured_libmpv_path(settings)? {
        return load_api(&path);
    }

    let mut errors = Vec::new();
    for candidate in default_libmpv_load_candidates(app) {
        match load_api(&candidate) {
            Ok(api) => return Ok(api),
            Err(err) => errors.push(err),
        }
    }

    if errors.is_empty() {
        Err(libmpv_not_found_message())
    } else {
        Err(format!(
            "Failed to load libmpv from default locations:\n{}",
            errors.join("\n")
        ))
    }
}

#[cfg(target_os = "windows")]
fn default_libmpv_library_names() -> &'static [&'static str] {
    &["libmpv-2.dll", "mpv-2.dll"]
}

#[cfg(target_os = "macos")]
fn default_libmpv_library_names() -> &'static [&'static str] {
    &[
        "Mpv.xcframework/macos-arm64_x86_64/Mpv.framework/Versions/A/Mpv",
        "Mpv.framework/Versions/A/Mpv",
        "libmpv.2.dylib",
        "libmpv.dylib",
    ]
}

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn default_libmpv_library_names() -> &'static [&'static str] {
    &["libmpv.so.2", "libmpv.so"]
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn bundled_libmpv_platform_dir() -> &'static str {
    "windows-x86_64"
}

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
fn bundled_libmpv_platform_dir() -> &'static str {
    "windows-aarch64"
}

#[cfg(all(
    target_os = "macos",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn bundled_libmpv_platform_dir() -> &'static str {
    "macos-universal"
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn bundled_libmpv_platform_dir() -> &'static str {
    "linux-x86_64"
}

#[cfg(all(target_os = "linux", target_arch = "aarch64"))]
fn bundled_libmpv_platform_dir() -> &'static str {
    "linux-aarch64"
}

#[cfg(not(any(
    all(
        target_os = "windows",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ),
    all(
        target_os = "macos",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ),
    all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )
)))]
fn bundled_libmpv_platform_dir() -> &'static str {
    "unknown"
}

fn libmpv_not_found_message() -> String {
    format!(
        "libmpv was not found. Bundle it under libmpv/{} or set the libmpv path in settings.",
        bundled_libmpv_platform_dir()
    )
}

fn configured_libmpv_path(settings: &AppSettings) -> Result<Option<PathBuf>, String> {
    if let Some(path) = settings
        .mpv_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        let custom = PathBuf::from(path);
        if custom.is_file() {
            return Ok(Some(custom));
        }
        if custom.is_dir() {
            if let Some(library) = find_library_in_dir(&custom) {
                return Ok(Some(library));
            }
        }
        return Err(format!(
            "Configured libmpv was not found: {}",
            custom.display()
        ));
    }

    Ok(None)
}

fn default_libmpv_load_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut candidates = libmpv_candidates(app)
        .into_iter()
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    append_default_loader_candidates(&mut candidates);
    candidates
}

fn append_default_loader_candidates(candidates: &mut Vec<PathBuf>) {
    candidates.extend(default_libmpv_library_names().iter().map(PathBuf::from));
    dedupe_libmpv_candidates(candidates);
}

fn dedupe_libmpv_candidates(candidates: &mut Vec<PathBuf>) {
    let mut seen = HashSet::new();
    candidates.retain(|path| seen.insert(path.clone()));
}

fn libmpv_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        push_libmpv_dir_candidates(&mut candidates, &cwd.join("libmpv"));
        push_libmpv_dir_candidates(
            &mut candidates,
            &cwd.join("libmpv").join(bundled_libmpv_platform_dir()),
        );
        push_libmpv_dir_candidates(&mut candidates, &cwd.join("..").join("libmpv"));
        push_libmpv_dir_candidates(
            &mut candidates,
            &cwd.join("..")
                .join("libmpv")
                .join(bundled_libmpv_platform_dir()),
        );
        push_libmpv_dir_candidates(&mut candidates, &cwd.join("mpv"));
        push_libmpv_dir_candidates(&mut candidates, &cwd.join("..").join("mpv"));
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        push_libmpv_dir_candidates(&mut candidates, &resource_dir.join("libmpv"));
        push_libmpv_dir_candidates(
            &mut candidates,
            &resource_dir
                .join("libmpv")
                .join(bundled_libmpv_platform_dir()),
        );
        push_libmpv_dir_candidates(&mut candidates, &resource_dir.join("mpv"));
    }
    candidates
}

fn push_libmpv_dir_candidates(candidates: &mut Vec<PathBuf>, dir: &Path) {
    for name in default_libmpv_library_names() {
        candidates.push(dir.join(name));
    }
}

fn find_library_in_dir(dir: &Path) -> Option<PathBuf> {
    default_libmpv_library_names()
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

fn find_libmpv_config_dir(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        for bundled in [
            resource_dir.join("mpv"),
            resource_dir.join("libmpv"),
            resource_dir
                .join("libmpv")
                .join(bundled_libmpv_platform_dir()),
        ] {
            if is_dir(&bundled) {
                return Some(bundled);
            }
        }
    }
    let cwd = std::env::current_dir().ok()?;
    [
        cwd.join("mpv"),
        cwd.join("..").join("mpv"),
        cwd.join("libmpv"),
        cwd.join("libmpv").join(bundled_libmpv_platform_dir()),
        cwd.join("..").join("libmpv"),
        cwd.join("..")
            .join("libmpv")
            .join(bundled_libmpv_platform_dir()),
    ]
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
            normalize_command("external_subtitle:https://example.test/movie.srt").unwrap(),
            "external_subtitle:https://example.test/movie.srt"
        );
        assert!(normalize_command("external_subtitle:/definitely/missing/movie.srt").is_err());
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
    fn parses_progress_written_by_libmpv_state_poll() {
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
            r#"{"timePos":12.5,"duration":120.0,"paused":false,"muted":true,"volume":42,"videoReady":true,"speed":1.25,"cacheSpeed":1048576.0}"#,
        )
        .unwrap();
        assert_eq!(state.speed, Some(1.25));
        assert_eq!(state.cache_speed, Some(1_048_576.0));
    }

    #[test]
    fn converts_resume_ticks_to_libmpv_start_value() {
        assert_eq!(mpv_start_value(Some(90_500_000)), Some("9.050".to_string()));
        assert_eq!(mpv_start_value(Some(0)), None);
        assert_eq!(mpv_start_value(None), None);
    }

    #[test]
    fn converts_app_settings_to_libmpv_options() {
        let settings = AppSettings {
            default_volume: 65,
            subtitle_mode: "off".to_string(),
            ..Default::default()
        };

        let options = mpv_settings_options(&settings);

        assert!(options.contains(&("volume".to_string(), "65".to_string())));
        assert!(!options.contains(&("sid".to_string(), "no".to_string())));
        assert_eq!(
            mpv_subtitle_options(&settings, None, None),
            vec![("sid".to_string(), "no".to_string())]
        );
    }

    #[test]
    fn converts_explicit_subtitle_selection_to_libmpv_options() {
        let settings = AppSettings {
            subtitle_mode: "off".to_string(),
            ..Default::default()
        };

        assert_eq!(
            mpv_subtitle_options(&settings, Some(2), None),
            vec![("sid".to_string(), "2".to_string())]
        );
        assert_eq!(
            mpv_subtitle_options(&settings, Some(-1), None),
            vec![("sid".to_string(), "no".to_string())]
        );
        assert_eq!(
            mpv_subtitle_options(&settings, Some(1), Some("http://example.test/sub.srt")),
            Vec::<(String, String)>::new()
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
            cache_speed: Some(512.0),
            video_ready: true,
        };

        cache_state(session_id, state.clone());

        let cached = super::state(session_id).unwrap();
        assert_eq!(cached.time_pos, state.time_pos);
        assert_eq!(cached.duration, state.duration);
        assert_eq!(cached.paused, state.paused);
        assert_eq!(cached.muted, state.muted);
        assert_eq!(cached.volume, state.volume);
        assert_eq!(cached.cache_speed, state.cache_speed);
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
                cache_speed: None,
                video_ready: false,
            },
        );

        forget_control(session_id);

        assert!(cached_state(session_id).unwrap().is_none());
    }

    #[test]
    fn terminate_all_reports_when_no_session_was_active() {
        clear_sessions();

        assert!(!terminate_all());
    }

    #[test]
    fn default_libmpv_library_names_match_current_platform() {
        #[cfg(target_os = "windows")]
        assert_eq!(
            default_libmpv_library_names(),
            &["libmpv-2.dll", "mpv-2.dll"]
        );

        #[cfg(target_os = "macos")]
        assert_eq!(
            default_libmpv_library_names(),
            &[
                "Mpv.xcframework/macos-arm64_x86_64/Mpv.framework/Versions/A/Mpv",
                "Mpv.framework/Versions/A/Mpv",
                "libmpv.2.dylib",
                "libmpv.dylib",
            ]
        );

        #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
        assert_eq!(
            default_libmpv_library_names(),
            &["libmpv.so.2", "libmpv.so"]
        );
    }

    #[test]
    fn libmpv_not_found_message_mentions_platform_bundle() {
        let message = libmpv_not_found_message();

        assert!(message.contains("libmpv/"));
        assert!(message.contains(bundled_libmpv_platform_dir()));
        assert!(message.contains("set the libmpv path in settings"));
    }

    #[test]
    fn default_loader_candidates_preserve_bundled_priority_and_add_loader_names() {
        let bundled = PathBuf::from("/app/libmpv/linux-x86_64/libmpv.so.2");
        let mut candidates = vec![bundled.clone(), bundled.clone()];

        append_default_loader_candidates(&mut candidates);

        assert_eq!(candidates.first(), Some(&bundled));
        assert_eq!(
            candidates
                .iter()
                .filter(|candidate| **candidate == bundled)
                .count(),
            1
        );
        for name in default_libmpv_library_names() {
            assert!(candidates.contains(&PathBuf::from(name)));
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_sibling_preload_skips_dynamic_loader_search_names() {
        let loaded = preload_linux_sibling_libraries(Path::new("libmpv.so.2")).unwrap();

        assert!(loaded.is_empty());
    }

    #[test]
    fn disables_mpv_builtin_controls() {
        for expected in [
            ("input-default-bindings", "no"),
            ("input-vo-keyboard", "no"),
            ("osd-bar", "no"),
            ("osd-level", "0"),
        ] {
            assert!(DISABLE_MPV_UI_OPTIONS.contains(&expected));
        }
    }

    #[test]
    fn runs_libmpv_as_playback_engine() {
        assert!(MPV_ENGINE_OPTIONS.contains(&("keep-open", "no")));
    }

    #[test]
    fn enables_safe_hardware_decoding_when_available() {
        assert!(MPV_ACCELERATION_OPTIONS.contains(&("hwdec", "auto-safe")));
    }

    #[test]
    fn starts_libmpv_paused_until_the_webview_is_ready() {
        assert!(MPV_SYNC_START_OPTIONS.contains(&("pause", "yes")));
    }

    #[test]
    fn libmpv_create_failure_message_mentions_numeric_locale() {
        assert!(MPV_CREATE_FAILURE_MESSAGE.contains("LC_NUMERIC"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_uses_libmpv_render_vo_instead_of_external_window() {
        assert!(LINUX_RENDER_API_OPTIONS.contains(&("vo", "libmpv")));
        assert!(LINUX_RENDER_API_OPTIONS.contains(&("force-window", "no")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_preloads_sibling_shared_libraries_but_not_libmpv() {
        assert!(linux_sibling_library_name("libavcodec.so.60"));
        assert!(linux_sibling_library_name("libplacebo.so"));
        assert!(!linux_sibling_library_name("libmpv.so.2"));
        assert!(!linux_sibling_library_name("libmpv.so"));
        assert!(!linux_sibling_library_name("README.md"));
    }

    #[test]
    fn libmpv_render_api_abi_constants_match_headers() {
        assert_eq!(MPV_RENDER_PARAM_INVALID, 0);
        assert_eq!(MPV_RENDER_PARAM_API_TYPE, 1);
        assert_eq!(MPV_RENDER_PARAM_OPENGL_INIT_PARAMS, 2);
        assert_eq!(MPV_RENDER_PARAM_OPENGL_FBO, 3);
        assert_eq!(MPV_RENDER_PARAM_FLIP_Y, 4);
        assert_eq!(MPV_RENDER_PARAM_WL_DISPLAY, 9);
        assert_eq!(MPV_RENDER_UPDATE_FRAME, 1);
        assert_eq!(MPV_RENDER_API_TYPE_OPENGL, "opengl");
        #[cfg(target_os = "linux")]
        assert_eq!(GL_FRAMEBUFFER_BINDING, 0x8CA6);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn gtk_gl_area_render_size_uses_physical_pixels() {
        assert_eq!(scaled_gl_area_size(1920, 1080, 2), (3840, 2160));
        assert_eq!(scaled_gl_area_size(0, 0, 0), (1, 1));
    }

    #[test]
    fn avoids_verbose_mpv_log_writes_during_playback() {
        assert_eq!(MPV_LOG_LEVEL, "all=warn");
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
