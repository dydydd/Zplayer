use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Runtime;

static NATIVE_VIDEO_OVERLAY_INSTALLED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DesktopPlatform {
    Windows,
    Linux,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WindowTransparency {
    Transparent,
    Opaque,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LinuxWindowDiagnostics {
    pub(crate) xdg_session_type: Option<String>,
    pub(crate) wayland_display_set: bool,
    pub(crate) gdk_backend: Option<String>,
    pub(crate) wayland_required: bool,
    pub(crate) gdk_backend_wayland: bool,
    pub(crate) native_video_overlay: bool,
    pub(crate) opaque_window: bool,
}

pub(crate) fn prepare_linux_wayland_environment() {
    #[cfg(target_os = "linux")]
    std::env::set_var("GDK_BACKEND", "wayland");
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
    #[cfg(target_os = "linux")]
    let builder = if diagnostics().opaque_window {
        builder
            .transparent(false)
            .background_color(tauri::utils::config::Color(5, 5, 5, 255))
    } else {
        builder
    };
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

    let webview_window = window.clone();
    let container_window = window.clone();
    webview_window.with_webview(move |webview| {
        let Ok(container) = container_window.default_vbox() else {
            return;
        };
        let webview = webview.inner();
        let overlay = gtk::Overlay::new();
        let video_area = gtk::GLArea::new();

        overlay.set_hexpand(true);
        overlay.set_vexpand(true);
        video_area.set_widget_name("zplayer-native-video");
        video_area.set_hexpand(true);
        video_area.set_vexpand(true);
        video_area.set_auto_render(false);
        video_area.set_has_alpha(false);

        container.remove(&webview);
        overlay.add(&video_area);
        overlay.add_overlay(&webview);
        overlay.set_overlay_pass_through(&webview, false);
        container.pack_start(&overlay, true, true, 0);
        container.show_all();
        NATIVE_VIDEO_OVERLAY_INSTALLED.store(true, Ordering::SeqCst);
    })?;
    Ok(())
}

pub(crate) fn diagnostics() -> LinuxWindowDiagnostics {
    let xdg_session_type = std::env::var("XDG_SESSION_TYPE").ok();
    let wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
    let gdk_backend = std::env::var("GDK_BACKEND").ok();
    let opaque_window = window_transparency(
        current_platform(),
        xdg_session_type.as_deref(),
        wayland_display.as_deref(),
        gdk_backend.as_deref(),
    ) == WindowTransparency::Opaque;

    LinuxWindowDiagnostics {
        xdg_session_type,
        wayland_display_set: wayland_display
            .as_deref()
            .is_some_and(|value| !value.is_empty()),
        wayland_required: current_platform() == DesktopPlatform::Linux,
        gdk_backend_wayland: is_wayland_backend(gdk_backend.as_deref()),
        native_video_overlay: NATIVE_VIDEO_OVERLAY_INSTALLED.load(Ordering::SeqCst),
        gdk_backend,
        opaque_window,
    }
}

pub(crate) fn is_wayland_session(
    xdg_session_type: Option<&str>,
    wayland_display: Option<&str>,
) -> bool {
    xdg_session_type.is_some_and(|value| value.eq_ignore_ascii_case("wayland"))
        || wayland_display.is_some_and(|value| !value.is_empty())
}

pub(crate) fn is_wayland_backend(gdk_backend: Option<&str>) -> bool {
    gdk_backend.is_some_and(|value| {
        value
            .split(',')
            .map(str::trim)
            .any(|backend| backend.eq_ignore_ascii_case("wayland"))
    })
}

pub(crate) fn window_transparency(
    platform: DesktopPlatform,
    xdg_session_type: Option<&str>,
    wayland_display: Option<&str>,
    gdk_backend: Option<&str>,
) -> WindowTransparency {
    if platform == DesktopPlatform::Linux
        && (is_wayland_session(xdg_session_type, wayland_display)
            || is_wayland_backend(gdk_backend))
    {
        WindowTransparency::Opaque
    } else {
        WindowTransparency::Transparent
    }
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
    fn detects_wayland_from_session_type_or_display() {
        assert!(is_wayland_session(Some("wayland"), None));
        assert!(is_wayland_session(None, Some("wayland-0")));
        assert!(!is_wayland_session(Some("x11"), None));
        assert!(!is_wayland_session(None, Some("")));
    }

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
    fn chooses_opaque_window_only_for_linux_wayland() {
        assert_eq!(
            window_transparency(DesktopPlatform::Linux, Some("wayland"), None, None),
            WindowTransparency::Opaque
        );
        assert_eq!(
            window_transparency(DesktopPlatform::Linux, Some("x11"), None, Some("wayland")),
            WindowTransparency::Opaque
        );
        assert_eq!(
            window_transparency(DesktopPlatform::Linux, Some("x11"), None, None),
            WindowTransparency::Transparent
        );
        assert_eq!(
            window_transparency(
                DesktopPlatform::Windows,
                Some("wayland"),
                Some("wayland-0"),
                Some("wayland"),
            ),
            WindowTransparency::Transparent
        );
    }
}
