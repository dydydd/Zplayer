mod api;
mod input;
mod media;
mod payload;
mod server;

pub(crate) use api::*;
pub(crate) use input::*;
pub(crate) use media::*;
pub(crate) use payload::*;
pub(crate) use server::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_server_defaults_to_system_proxy_for_old_records() {
        let raw = r#"{
          "id": "server",
          "name": "Server",
          "url": "http://127.0.0.1:8096",
          "username": "user",
          "userId": "user-id",
          "accessToken": "token",
          "active": true,
          "savedAt": 0
        }"#;

        let server: SavedServer = serde_json::from_str(raw).unwrap();

        assert!(server.use_system_proxy);
    }

    #[test]
    fn app_settings_default_to_real_feature_values() {
        let settings = AppSettings::default();

        assert_eq!(settings.default_volume, 100);
        assert_eq!(settings.seek_back_seconds, 10);
        assert_eq!(settings.seek_forward_seconds, 30);
        assert_eq!(settings.subtitle_mode, "auto");
        assert_eq!(settings.poster_density, "comfortable");
        assert!(settings.metadata_cache_enabled);
        assert_eq!(settings.theme, "dark");
        assert!(!settings.diagnostics_enabled);
        assert!(settings.autoplay_next_episode);
        assert_eq!(settings.language, "auto");
    }

    #[test]
    fn normalize_settings_clamps_and_whitelists_user_input() {
        let settings = normalize_settings(SaveSettingsInput {
            mpv_path: Some("  C:/libmpv/libmpv-2.dll  ".to_string()),
            default_volume: Some(140),
            seek_back_seconds: Some(1),
            seek_forward_seconds: Some(999),
            subtitle_mode: Some("off".to_string()),
            poster_density: Some("tiny".to_string()),
            metadata_cache_enabled: Some(false),
            theme: Some("midnight".to_string()),
            diagnostics_enabled: Some(true),
            autoplay_next_episode: Some(false),
            language: Some("fr-FR".to_string()),
        });

        assert_eq!(settings.mpv_path.as_deref(), Some("C:/libmpv/libmpv-2.dll"));
        assert_eq!(settings.default_volume, 100);
        assert_eq!(settings.seek_back_seconds, 5);
        assert_eq!(settings.seek_forward_seconds, 180);
        assert_eq!(settings.subtitle_mode, "off");
        assert_eq!(settings.poster_density, "comfortable");
        assert!(!settings.metadata_cache_enabled);
        assert_eq!(settings.theme, "midnight");
        assert!(settings.diagnostics_enabled);
        assert!(!settings.autoplay_next_episode);
        assert_eq!(settings.language, "auto");
    }
}
