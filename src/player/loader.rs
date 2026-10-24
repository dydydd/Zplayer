use libloading::Library;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Filesystem locations needed by the playback engine.
#[derive(Debug, Clone, Default)]
pub(crate) struct PlayerPaths {
    pub libmpv_path: Option<PathBuf>,
    pub config_dir: PathBuf,
    pub log_dir: PathBuf,
}

type CachedLibrary = Result<Arc<Library>, String>;
static LIBRARIES: OnceLock<Mutex<HashMap<PathBuf, CachedLibrary>>> = OnceLock::new();

/// Dynamically loaded libmpv and the function pointers resolved from it.
/// The library is retained for at least as long as every copied function pointer.
#[derive(Clone)]
pub(crate) struct LibmpvLoader {
    pub(crate) _library: Arc<Library>,
    pub(crate) create: MpvCreate,
    pub(crate) initialize: MpvInitialize,
    pub(crate) terminate_destroy: MpvTerminateDestroy,
    pub(crate) command: MpvCommand,
    pub(crate) set_option_string: MpvSetOptionString,
    pub(crate) get_property: MpvGetProperty,
    pub(crate) error_string: MpvErrorString,
    pub(crate) path: PathBuf,
}

pub(crate) type MpvCreate = unsafe extern "C" fn() -> *mut libc::c_void;
pub(crate) type MpvInitialize = unsafe extern "C" fn(*mut libc::c_void) -> libc::c_int;
pub(crate) type MpvTerminateDestroy = unsafe extern "C" fn(*mut libc::c_void);
pub(crate) type MpvCommand =
    unsafe extern "C" fn(*mut libc::c_void, *const *const libc::c_char) -> libc::c_int;
pub(crate) type MpvSetOptionString = unsafe extern "C" fn(
    *mut libc::c_void,
    *const libc::c_char,
    *const libc::c_char,
) -> libc::c_int;
pub(crate) type MpvGetProperty = unsafe extern "C" fn(
    *mut libc::c_void,
    *const libc::c_char,
    libc::c_int,
    *mut libc::c_void,
) -> libc::c_int;
pub(crate) type MpvErrorString = unsafe extern "C" fn(libc::c_int) -> *const libc::c_char;

impl LibmpvLoader {
    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        let key = canonical_cache_key(path);
        let cache = LIBRARIES.get_or_init(|| Mutex::new(HashMap::new()));
        let library = {
            let mut cache = cache
                .lock()
                .map_err(|_| "Failed to access libmpv loader cache.".to_string())?;
            if let Some(found) = cache.get(&key) {
                found.clone()?
            } else {
                // SAFETY: The loaded library is retained in the cache and each
                // function pointer is copied only after symbol lookup succeeds.
                let loaded = unsafe { open_library(path) }
                    .map(Arc::new)
                    .map_err(|e| format!("Could not load libmpv at {}: {e}", path.display()));
                cache.insert(key, loaded.clone());
                loaded?
            }
        };

        // SAFETY: Symbol types match libmpv's public C ABI. The Arc held in
        // `_library` keeps all symbols valid for this loader's lifetime.
        unsafe {
            macro_rules! symbol {
                ($name:literal, $ty:ty) => {
                    *library
                        .get::<$ty>(concat!($name, "\0").as_bytes())
                        .map_err(|e| format!("libmpv is missing {}: {e}", $name))?
                };
            }
            Ok(Self {
                _library: library.clone(),
                create: symbol!("mpv_create", MpvCreate),
                initialize: symbol!("mpv_initialize", MpvInitialize),
                terminate_destroy: symbol!("mpv_terminate_destroy", MpvTerminateDestroy),
                command: symbol!("mpv_command", MpvCommand),
                set_option_string: symbol!("mpv_set_option_string", MpvSetOptionString),
                get_property: symbol!("mpv_get_property", MpvGetProperty),
                error_string: symbol!("mpv_error_string", MpvErrorString),
                path: path.to_path_buf(),
            })
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn open_library(path: &Path) -> Result<Library, libloading::Error> {
    use libloading::os::windows::{LOAD_WITH_ALTERED_SEARCH_PATH, Library as WindowsLibrary};
    // SAFETY: Forwarding the same path to the platform loader; the resulting
    // handle is immediately wrapped by libloading and retained in the cache.
    unsafe { WindowsLibrary::load_with_flags(path, LOAD_WITH_ALTERED_SEARCH_PATH) }.map(Into::into)
}

#[cfg(not(target_os = "windows"))]
unsafe fn open_library(path: &Path) -> Result<Library, libloading::Error> {
    // SAFETY: The caller retains the returned library for every resolved symbol.
    unsafe { Library::new(path) }
}

fn canonical_cache_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(path)
        }
    })
}

pub(crate) fn library_candidates(paths: &PlayerPaths) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(configured) = paths.libmpv_path.as_deref() {
        if configured.is_dir() {
            add_dir_candidates(&mut candidates, configured);
        } else {
            candidates.push(configured.to_path_buf());
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        for base in [cwd.clone(), cwd.join(".."), paths.config_dir.clone()] {
            let root = base.join("libmpv");
            add_dir_candidates(&mut candidates, &root);
            add_dir_candidates(&mut candidates, &root.join(bundled_platform_dir()));
            add_dir_candidates(&mut candidates, &base.join("mpv"));
        }
    }
    candidates.extend(default_library_names().iter().map(PathBuf::from));
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|candidate| seen.insert(candidate.clone()));
    candidates
}

fn add_dir_candidates(out: &mut Vec<PathBuf>, dir: &Path) {
    for name in default_library_names() {
        out.push(dir.join(name));
    }
}

#[cfg(target_os = "windows")]
fn default_library_names() -> &'static [&'static str] {
    &["libmpv-2.dll", "mpv-2.dll"]
}
#[cfg(target_os = "macos")]
fn default_library_names() -> &'static [&'static str] {
    &[
        "Mpv.xcframework/macos-arm64_x86_64/Mpv.framework/Versions/A/Mpv",
        "Mpv.framework/Versions/A/Mpv",
        "libmpv.2.dylib",
        "libmpv.dylib",
    ]
}
#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
fn default_library_names() -> &'static [&'static str] {
    &["libmpv.so.2", "libmpv.so"]
}

pub(crate) fn bundled_platform_dir() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return "windows-x86_64";
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        return "windows-aarch64";
    }
    #[cfg(all(
        target_os = "macos",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    {
        return "macos-universal";
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return "linux-x86_64";
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return "linux-aarch64";
    }
    #[allow(unreachable_code)]
    "unknown"
}
