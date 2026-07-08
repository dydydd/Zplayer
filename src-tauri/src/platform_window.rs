use serde::Serialize;
use tauri::Runtime;

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
    pub(crate) opaque_window: bool,
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
    builder.build()?;
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
    ) == WindowTransparency::Opaque;

    LinuxWindowDiagnostics {
        xdg_session_type,
        wayland_display_set: wayland_display
            .as_deref()
            .is_some_and(|value| !value.is_empty()),
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

pub(crate) fn window_transparency(
    platform: DesktopPlatform,
    xdg_session_type: Option<&str>,
    wayland_display: Option<&str>,
) -> WindowTransparency {
    if platform == DesktopPlatform::Linux && is_wayland_session(xdg_session_type, wayland_display) {
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
    fn chooses_opaque_window_only_for_linux_wayland() {
        assert_eq!(
            window_transparency(DesktopPlatform::Linux, Some("wayland"), None),
            WindowTransparency::Opaque
        );
        assert_eq!(
            window_transparency(DesktopPlatform::Linux, Some("x11"), None),
            WindowTransparency::Transparent
        );
        assert_eq!(
            window_transparency(DesktopPlatform::Windows, Some("wayland"), Some("wayland-0")),
            WindowTransparency::Transparent
        );
    }
}
