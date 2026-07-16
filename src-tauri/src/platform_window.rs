use serde::Serialize;
#[cfg(target_os = "linux")]
use std::cell::RefCell;
use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering},
    OnceLock,
};
use tauri::Runtime;

static NATIVE_VIDEO_OVERLAY_INSTALLED: AtomicBool = AtomicBool::new(false);
static NATIVE_VIDEO_RENDER_COUNT: AtomicU64 = AtomicU64::new(0);
static NATIVE_VIDEO_RENDER_WIDTH: AtomicI32 = AtomicI32::new(0);
static NATIVE_VIDEO_RENDER_HEIGHT: AtomicI32 = AtomicI32::new(0);
#[cfg(target_os = "linux")]
static NATIVE_VIDEO_THREAD: OnceLock<std::thread::ThreadId> = OnceLock::new();

#[cfg(target_os = "linux")]
thread_local! {
    static NATIVE_VIDEO_AREA: RefCell<Option<gtk::GLArea>> = const { RefCell::new(None) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DesktopPlatform {
    Windows,
    Linux,
    Other,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LinuxWindowDiagnostics {
    pub(crate) xdg_session_type: Option<String>,
    pub(crate) wayland_display_set: bool,
    pub(crate) gdk_backend: Option<String>,
    pub(crate) winit_unix_backend: Option<String>,
    pub(crate) wayland_required: bool,
    pub(crate) gdk_backend_wayland: bool,
    pub(crate) winit_backend_wayland: bool,
    pub(crate) native_video_overlay: bool,
    pub(crate) native_video_render_count: u64,
    pub(crate) native_video_render_width: i32,
    pub(crate) native_video_render_height: i32,
    pub(crate) native_video_render_context: bool,
    pub(crate) opaque_window: bool,
}

pub(crate) fn prepare_linux_wayland_environment() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("GDK_BACKEND", "wayland");
        std::env::set_var("WINIT_UNIX_BACKEND", "wayland");
        let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        if !is_wayland_display_set(wayland_display.as_deref()) {
            return Err(
                "Zplayer Linux desktop only supports Wayland. WAYLAND_DISPLAY is not set."
                    .to_string(),
            );
        }
    }
    Ok(())
}

pub(crate) fn create_main_window<R: Runtime>(
    app: &mut tauri::App<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .cloned()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Main window config was not found.",
            )
        })?;
    let builder = tauri::WebviewWindowBuilder::from_config(app, &config)?;
    let window = builder.build()?;
    #[cfg(target_os = "linux")]
    install_native_video_overlay(&window)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn install_native_video_overlay<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
) -> Result<(), Box<dyn std::error::Error>> {
    use gtk::prelude::*;
    use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};

    let webview_window = window.clone();
    let container_window = window.clone();
    webview_window.with_webview(move |webview| {
        let Ok(container) = container_window.default_vbox() else {
            return;
        };
        let Some(gtk_window) = container
            .parent()
            .and_then(|parent| parent.downcast::<gtk::Window>().ok())
        else {
            return;
        };
        let webview = webview.inner();
        let overlay = gtk::Overlay::new();
        let video_area = gtk::GLArea::new();
        let transparent = gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0);

        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        overlay.set_app_paintable(true);
        video_area.set_widget_name("zplayer-native-video");
        video_area.set_hexpand(true);
        video_area.set_vexpand(true);
        video_area.set_auto_render(false);
        video_area.set_has_alpha(false);
        video_area.connect_render(|area, _context| handle_native_video_render(area));
        webview.set_app_paintable(true);
        webview.set_background_color(&transparent);
        if let Some(settings) = WebViewExt::settings(&webview) {
            settings.set_hardware_acceleration_policy(HardwareAccelerationPolicy::Never);
        }
        gtk_window.set_app_paintable(true);
        if let Some(visual) =
            gtk::prelude::GtkWindowExt::screen(&gtk_window).and_then(|screen| screen.rgba_visual())
        {
            gtk_window.set_visual(Some(&visual));
            overlay.set_visual(Some(&visual));
            webview.set_visual(Some(&visual));
        }

        container.remove(&webview);
        overlay.add(&video_area);
        overlay.add_overlay(&webview);
        overlay.set_overlay_pass_through(&webview, false);
        gtk_window.remove(&container);
        gtk_window.add(&overlay);
        NATIVE_VIDEO_AREA.with(|stored| {
            *stored.borrow_mut() = Some(video_area);
        });
        let _ = NATIVE_VIDEO_THREAD.set(std::thread::current().id());
        gtk_window.show_all();
        NATIVE_VIDEO_OVERLAY_INSTALLED.store(true, Ordering::SeqCst);
    })?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn handle_native_video_render(area: &gtk::GLArea) -> gtk::glib::Propagation {
    use gtk::prelude::*;

    NATIVE_VIDEO_RENDER_COUNT.fetch_add(1, Ordering::SeqCst);
    NATIVE_VIDEO_RENDER_WIDTH.store(area.allocated_width(), Ordering::SeqCst);
    NATIVE_VIDEO_RENDER_HEIGHT.store(area.allocated_height(), Ordering::SeqCst);
    crate::mpv::render_native_video(area)
}

#[cfg(target_os = "linux")]
pub(crate) fn with_native_video_area<T>(action: impl FnOnce(&gtk::GLArea) -> T) -> Option<T> {
    NATIVE_VIDEO_AREA.with(|stored| stored.borrow().as_ref().map(action))
}

#[cfg(target_os = "linux")]
pub(crate) fn native_video_thread_is_current() -> bool {
    NATIVE_VIDEO_THREAD
        .get()
        .is_some_and(|thread| *thread == std::thread::current().id())
}

#[cfg(target_os = "linux")]
pub(crate) fn queue_native_video_render() {
    if !gtk::glib::MainContext::default().is_owner() {
        gtk::glib::idle_add_once(queue_native_video_render);
        return;
    }

    let _ = with_native_video_area(|area| {
        use gtk::prelude::*;

        area.queue_render();
    });
}

pub(crate) fn diagnostics() -> LinuxWindowDiagnostics {
    let xdg_session_type = std::env::var("XDG_SESSION_TYPE").ok();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    let gdk_backend = std::env::var("GDK_BACKEND").ok();
    let winit_unix_backend = std::env::var("WINIT_UNIX_BACKEND").ok();
    #[cfg(target_os = "linux")]
    let native_video_render_context = crate::mpv::native_video_render_context_active();
    #[cfg(not(target_os = "linux"))]
    let native_video_render_context = false;

    LinuxWindowDiagnostics {
        xdg_session_type,
        wayland_display_set: is_wayland_display_set(wayland_display.as_deref()),
        wayland_required: current_platform() == DesktopPlatform::Linux,
        gdk_backend_wayland: is_wayland_backend(gdk_backend.as_deref()),
        winit_backend_wayland: is_winit_wayland_backend(winit_unix_backend.as_deref()),
        native_video_overlay: NATIVE_VIDEO_OVERLAY_INSTALLED.load(Ordering::SeqCst),
        native_video_render_count: NATIVE_VIDEO_RENDER_COUNT.load(Ordering::SeqCst),
        native_video_render_width: NATIVE_VIDEO_RENDER_WIDTH.load(Ordering::SeqCst),
        native_video_render_height: NATIVE_VIDEO_RENDER_HEIGHT.load(Ordering::SeqCst),
        native_video_render_context,
        gdk_backend,
        winit_unix_backend,
        opaque_window: false,
    }
}

pub(crate) fn is_wayland_display_set(wayland_display: Option<&str>) -> bool {
    wayland_display.is_some_and(|value| !value.trim().is_empty())
}

pub(crate) fn is_wayland_backend(gdk_backend: Option<&str>) -> bool {
    gdk_backend.is_some_and(|value| {
        value
            .split(',')
            .map(str::trim)
            .any(|backend| backend.eq_ignore_ascii_case("wayland"))
    })
}

pub(crate) fn is_winit_wayland_backend(winit_unix_backend: Option<&str>) -> bool {
    winit_unix_backend.is_some_and(|value| value.eq_ignore_ascii_case("wayland"))
}

fn current_platform() -> DesktopPlatform {
    if cfg!(target_os = "windows") {
        DesktopPlatform::Windows
    } else if cfg!(target_os = "linux") {
        DesktopPlatform::Linux
    } else {
        DesktopPlatform::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wayland_from_gdk_backend() {
        assert!(is_wayland_backend(Some("wayland")));
        assert!(is_wayland_backend(Some("wayland,x11")));
        assert!(is_wayland_backend(Some("x11, wayland")));
        assert!(!is_wayland_backend(Some("x11")));
        assert!(!is_wayland_backend(Some("waylandish")));
        assert!(!is_wayland_backend(None));
    }

    #[test]
    fn validates_wayland_display_presence() {
        assert!(is_wayland_display_set(Some("wayland-0")));
        assert!(!is_wayland_display_set(Some("")));
        assert!(!is_wayland_display_set(Some("   ")));
        assert!(!is_wayland_display_set(None));
    }

    #[test]
    fn detects_winit_wayland_backend() {
        assert!(is_winit_wayland_backend(Some("wayland")));
        assert!(is_winit_wayland_backend(Some("WAYLAND")));
        assert!(!is_winit_wayland_backend(Some("x11")));
        assert!(!is_winit_wayland_backend(None));
    }

    #[test]
    fn diagnostics_keep_window_transparent_for_native_video() {
        assert!(!diagnostics().opaque_window);
    }
}
