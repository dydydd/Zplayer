//! FluentUI design tokens for Zplayer.
//!
//! The palette is aligned to the official microsoft/fluentui token values
//! (webDarkTheme / webLightTheme): layered neutral surfaces, a single brand
//! accent, hairline strokes, and high-contrast typography. `ThemeMode`
//! persists as `"Light"` / `"Dark"` / `"Default"` (case-insensitive);
//! `"Default"` follows the OS appearance and re-resolves live when the
//! window appearance changes.

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
    /// Dark palette (webDarkTheme actual values): layered neutral surfaces
    /// with the Zplayer green accent.
    pub(crate) fn dark() -> Self {
        Self {
            // colorNeutralBackground3 (grey8).
            bg_sidebar: rgb(0x141414),
            // colorNeutralBackground2 (grey12).
            bg_main: rgb(0x1F1F1F),
            bg_header: rgb(0x1F1F1F),
            // colorNeutralCardBackground (grey20).
            bg_card: rgb(0x333333),
            // colorNeutralCardBackgroundHover (grey24).
            bg_card_hover: rgb(0x3D3D3D),
            // colorNeutralBackground1 (grey16) — control/input surface.
            bg_control: rgb(0x292929),
            // colorNeutralBackground1Hover (grey24).
            bg_control_hover: rgb(0x3D3D3D),
            // colorNeutralForeground1.
            text_primary: rgb(0xFFFFFF),
            // colorNeutralForeground3 (grey68).
            text_secondary: rgb(0xADADAD),
            // colorBrandBackground (Zplayer green, not MS blue).
            accent: rgb(0x4CC38A),
            accent_hover: rgb(0x5CD39A),
            accent_pressed: rgb(0x3CB37A),
            // Dark ink on the bright green reads better than white.
            accent_text_on: rgb(0x0C1512),
            // colorNeutralStroke1 (dark stroke ≈ grey33).
            border: rgb(0x525252),
            // colorStatusDangerForeground1 (bright step for dark).
            danger: rgb(0xE5484D),
            scrim: rgba(0x000000B3),
        }
    }

    /// Light palette (webLightTheme actual values): neutral surfaces with a
    /// deeper accent for contrast.
    pub(crate) fn light() -> Self {
        Self {
            // colorNeutralBackground3 (grey96).
            bg_sidebar: rgb(0xF0F0F0),
            // colorNeutralBackground2 (grey98).
            bg_main: rgb(0xFAFAFA),
            bg_header: rgb(0xFAFAFA),
            // colorNeutralCardBackground (grey98).
            bg_card: rgb(0xFAFAFA),
            // colorNeutralCardBackgroundHover.
            bg_card_hover: rgb(0xFFFFFF),
            // colorNeutralBackground1.
            bg_control: rgb(0xFFFFFF),
            // colorNeutralBackground1Hover (grey96).
            bg_control_hover: rgb(0xF5F5F5),
            // colorNeutralForeground1 (grey14).
            text_primary: rgb(0x242424),
            // colorNeutralForeground3 (grey38).
            text_secondary: rgb(0x616161),
            // colorBrandBackground (deep green for contrast).
            accent: rgb(0x2E9E6B),
            accent_hover: rgb(0x279061),
            accent_pressed: rgb(0x218256),
            // colorNeutralForegroundOnBrand.
            accent_text_on: rgb(0xFFFFFF),
            // colorNeutralStroke1 (grey82).
            border: rgb(0xD1D1D1),
            // colorStatusDangerForeground1 (dark cranberry bright step).
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
    fn dark_palette_matches_fluentui_token_values() {
        let dark = ThemeColors::dark();
        // colorNeutralBackground2 / 3.
        assert_eq!(dark.bg_main, rgb(0x1F1F1F));
        assert_eq!(dark.bg_sidebar, rgb(0x141414));
        assert_eq!(dark.bg_header, rgb(0x1F1F1F));
        // colorNeutralCardBackground / Hover.
        assert_eq!(dark.bg_card, rgb(0x333333));
        assert_eq!(dark.bg_card_hover, rgb(0x3D3D3D));
        // colorNeutralBackground1 / Hover (control surface).
        assert_eq!(dark.bg_control, rgb(0x292929));
        assert_eq!(dark.bg_control_hover, rgb(0x3D3D3D));
        // colorNeutralForeground1 / 3.
        assert_eq!(dark.text_primary, rgb(0xFFFFFF));
        assert_eq!(dark.text_secondary, rgb(0xADADAD));
        // colorNeutralStroke1.
        assert_eq!(dark.border, rgb(0x525252));
        // colorStatusDangerForeground1 (bright step).
        assert_eq!(dark.danger, rgb(0xE5484D));
        // Zplayer brand green family.
        assert_eq!(dark.accent, rgb(0x4CC38A));
        assert_eq!(dark.accent_hover, rgb(0x5CD39A));
        assert_eq!(dark.accent_pressed, rgb(0x3CB37A));
        assert_eq!(dark.accent_text_on, rgb(0x0C1512));
        // 70% black scrim.
        assert_eq!(dark.scrim, rgba(0x000000B3));
    }

    #[test]
    fn light_palette_matches_fluentui_token_values() {
        let light = ThemeColors::light();
        // colorNeutralBackground2 / 3.
        assert_eq!(light.bg_main, rgb(0xFAFAFA));
        assert_eq!(light.bg_sidebar, rgb(0xF0F0F0));
        assert_eq!(light.bg_header, rgb(0xFAFAFA));
        // colorNeutralCardBackground / Hover.
        assert_eq!(light.bg_card, rgb(0xFAFAFA));
        assert_eq!(light.bg_card_hover, rgb(0xFFFFFF));
        // colorNeutralBackground1 / Hover.
        assert_eq!(light.bg_control, rgb(0xFFFFFF));
        assert_eq!(light.bg_control_hover, rgb(0xF5F5F5));
        // colorNeutralForeground1 / 3.
        assert_eq!(light.text_primary, rgb(0x242424));
        assert_eq!(light.text_secondary, rgb(0x616161));
        // colorNeutralStroke1.
        assert_eq!(light.border, rgb(0xD1D1D1));
        // colorStatusDangerForeground1.
        assert_eq!(light.danger, rgb(0xD13438));
        // Zplayer brand green family (deep for contrast).
        assert_eq!(light.accent, rgb(0x2E9E6B));
        assert_eq!(light.accent_hover, rgb(0x279061));
        assert_eq!(light.accent_pressed, rgb(0x218256));
        // colorNeutralForegroundOnBrand.
        assert_eq!(light.accent_text_on, rgb(0xFFFFFF));
        // 60% black scrim.
        assert_eq!(light.scrim, rgba(0x00000099));
    }

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
