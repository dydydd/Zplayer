//! Fluent-inspired design tokens for Zplayer.
//!
//! The palette follows the Windows 11 Fluent aesthetic: layered neutral
//! surfaces, a single brand accent, hairline strokes, and high-contrast
//! typography. `ThemeMode` persists as `"Light"` / `"Dark"` / `"Default"`
//! (case-insensitive); `"Default"` follows the OS appearance and re-resolves
//! live when the window appearance changes.

use gpui_kit::{Rgba, WindowAppearance, rgb, rgba};

/// Resolved theme colors. Every field is `Copy`, so `ThemeColors` itself is
/// `Copy` and is passed around by value (never cloned).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ThemeColors {
    /// Navigation pane background (one step above the base layer).
    pub(crate) bg_sidebar: Rgba,
    /// Main content background (base layer).
    pub(crate) bg_main: Rgba,
    /// Page header background.
    pub(crate) bg_header: Rgba,
    /// Card surface (media cards, settings rows, list tiles).
    pub(crate) bg_card: Rgba,
    /// Card hover / selected surface.
    pub(crate) bg_card_hover: Rgba,
    /// Control surface (buttons, inputs, chips).
    pub(crate) bg_control: Rgba,
    /// Control hover surface.
    pub(crate) bg_control_hover: Rgba,
    /// Primary text color.
    pub(crate) text_primary: Rgba,
    /// Secondary / muted text color.
    pub(crate) text_secondary: Rgba,
    /// Brand accent (Zplayer green).
    pub(crate) accent: Rgba,
    /// Accent hover variant.
    pub(crate) accent_hover: Rgba,
    /// Accent pressed variant.
    pub(crate) accent_pressed: Rgba,
    /// Text color painted on top of the accent fill.
    pub(crate) accent_text_on: Rgba,
    /// Hairline border color.
    pub(crate) border: Rgba,
    /// Destructive action foreground.
    pub(crate) danger: Rgba,
    /// Semi-transparent overlay used over imagery.
    pub(crate) scrim: Rgba,
}

impl ThemeColors {
    /// Dark palette: layered charcoal surfaces with the Zplayer green accent.
    pub(crate) fn dark() -> Self {
        Self {
            bg_sidebar: rgb(0x252526),
            bg_main: rgb(0x1E1E20),
            bg_header: rgb(0x232325),
            bg_card: rgb(0x2B2B2D),
            bg_card_hover: rgb(0x343436),
            bg_control: rgb(0x2E2E30),
            bg_control_hover: rgb(0x38383A),
            text_primary: rgb(0xFFFFFF),
            text_secondary: rgb(0xA6A6A6),
            accent: rgb(0x4CC38A),
            accent_hover: rgb(0x5CD39A),
            accent_pressed: rgb(0x3CB37A),
            accent_text_on: rgb(0x0C1512),
            border: rgb(0x3D3D3F),
            danger: rgb(0xE5484D),
            scrim: rgba(0x000000B3),
        }
    }

    /// Light palette: warm neutral surfaces with a deeper accent for contrast.
    pub(crate) fn light() -> Self {
        Self {
            bg_sidebar: rgb(0xECECEC),
            bg_main: rgb(0xF4F4F4),
            bg_header: rgb(0xEFEFEF),
            bg_card: rgb(0xFFFFFF),
            bg_card_hover: rgb(0xF2F2F2),
            bg_control: rgb(0xFFFFFF),
            bg_control_hover: rgb(0xEDEDED),
            text_primary: rgb(0x1A1A1A),
            text_secondary: rgb(0x616161),
            accent: rgb(0x2E9E6B),
            accent_hover: rgb(0x279061),
            accent_pressed: rgb(0x218256),
            accent_text_on: rgb(0xFFFFFF),
            border: rgb(0xE0E0E0),
            danger: rgb(0xD13438),
            scrim: rgba(0x00000099),
        }
    }
}

/// Persisted theme preference.
///
/// `FollowSystem` corresponds to the persisted `"Default"` value: the window
/// follows the OS light/dark setting.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(crate) enum ThemeMode {
    Light,
    Dark,
    #[default]
    FollowSystem,
}

impl ThemeMode {
    /// Parses the persisted preference string (case-insensitive).
    pub(crate) fn from_persisted(value: &str) -> Self {
        let value = value.trim();
        if value.eq_ignore_ascii_case("light") {
            ThemeMode::Light
        } else if value.eq_ignore_ascii_case("dark") {
            ThemeMode::Dark
        } else {
            ThemeMode::FollowSystem
        }
    }

    /// Resolves this mode into concrete colors.
    pub(crate) fn resolve(&self, system_dark: bool) -> ThemeColors {
        match self {
            ThemeMode::Light => ThemeColors::light(),
            ThemeMode::Dark => ThemeColors::dark(),
            ThemeMode::FollowSystem => {
                if system_dark {
                    ThemeColors::dark()
                } else {
                    ThemeColors::light()
                }
            }
        }
    }

    /// The value persisted for this mode.
    pub(crate) fn persisted(&self) -> &'static str {
        match self {
            ThemeMode::Light => "Light",
            ThemeMode::Dark => "Dark",
            ThemeMode::FollowSystem => "Default",
        }
    }
}

/// Text color painted on accent-filled surfaces. The Zplayer green accent is
/// bright in both themes, so a dark ink reads well on it everywhere.
pub(crate) fn on_accent_text() -> Rgba {
    rgb(0x0C1512)
}

/// Returns whether a gpui window appearance should be treated as dark.
pub(crate) fn is_dark_appearance(appearance: WindowAppearance) -> bool {
    matches!(
        appearance,
        WindowAppearance::Dark | WindowAppearance::VibrantDark
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_theme_is_parsed_case_insensitively() {
        assert_eq!(ThemeMode::from_persisted("Light"), ThemeMode::Light);
        assert_eq!(ThemeMode::from_persisted(" light "), ThemeMode::Light);
        assert_eq!(ThemeMode::from_persisted("DARK"), ThemeMode::Dark);
        assert_eq!(ThemeMode::from_persisted("dark"), ThemeMode::Dark);
    }

    #[test]
    fn default_and_unrecognized_themes_follow_the_system() {
        assert_eq!(ThemeMode::from_persisted("Default"), ThemeMode::FollowSystem);
        assert_eq!(ThemeMode::from_persisted("midnight"), ThemeMode::FollowSystem);
        assert_eq!(ThemeMode::from_persisted(""), ThemeMode::FollowSystem);
        assert_eq!(ThemeMode::default(), ThemeMode::FollowSystem);
    }

    #[test]
    fn resolve_follows_explicit_modes_and_the_system() {
        assert_eq!(
            ThemeMode::Light.resolve(false).text_primary,
            ThemeColors::light().text_primary
        );
        assert_eq!(
            ThemeMode::Light.resolve(true).bg_main,
            ThemeColors::light().bg_main
        );
        assert_eq!(
            ThemeMode::Dark.resolve(false).bg_sidebar,
            ThemeColors::dark().bg_sidebar
        );
        assert_eq!(
            ThemeMode::FollowSystem.resolve(true).bg_card,
            ThemeColors::dark().bg_card
        );
        assert_eq!(
            ThemeMode::FollowSystem.resolve(false).bg_card,
            ThemeColors::light().bg_card
        );
    }

    #[test]
    fn persisted_round_trip() {
        for mode in [ThemeMode::Light, ThemeMode::Dark, ThemeMode::FollowSystem] {
            assert_eq!(
                ThemeMode::from_persisted(mode.persisted()),
                mode
            );
        }
    }
}
