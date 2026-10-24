use crate::models::{
    AppSettings, PlaybackPreference, SavedServer, SavedServerSummary, ServerExport,
    ServerIconInput, ServerImportResult, ServerStore,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const STORE_FILE: &str = "servers.json";
const RECENT_PLAY_LIMIT: usize = 50;

#[derive(Clone, Debug)]
pub(crate) struct AppStore {
    data_dir: PathBuf,
}

impl AppStore {
    pub(crate) fn new(data_dir: impl Into<PathBuf>) -> Result<Self, String> {
        let data_dir = data_dir.into();
        fs::create_dir_all(&data_dir)
            .map_err(|err| format!("Failed to create app data directory: {err}"))?;
        Ok(Self { data_dir })
    }

    pub(crate) fn save_server(&self, mut saved: SavedServer) -> Result<SavedServerSummary, String> {
        let mut store = self.load()?;
        saved.active = true;
        for server in &mut store.servers {
            server.active = false;
        }
        let summary = server_summary(&saved);
        if let Some(existing) = store
            .servers
            .iter_mut()
            .find(|server| server.id == saved.id)
        {
            *existing = saved;
        } else {
            store.servers.push(saved);
        }
        store.active_server_id = Some(summary.id.clone());
        self.save(&store)?;
        Ok(summary)
    }

    pub(crate) fn list_servers(&self) -> Result<Vec<SavedServerSummary>, String> {
        Ok(self.load()?.servers.iter().map(server_summary).collect())
    }

    pub(crate) fn update_server_icon(
        &self,
        input: ServerIconInput,
    ) -> Result<SavedServerSummary, String> {
        let mut store = self.load()?;
        let summary = {
            let server = store
                .servers
                .iter_mut()
                .find(|server| server.id == input.server_id)
                .ok_or_else(|| "Server not found.".to_string())?;
            server.icon_url = clean_optional_string(input.icon_url);
            server.icon_name = clean_optional_string(input.icon_name);
            server_summary(server)
        };
        self.save(&store)?;
        Ok(summary)
    }

    pub(crate) fn servers(&self) -> Result<Vec<SavedServer>, String> {
        Ok(self.load()?.servers)
    }

    pub(crate) fn export_servers(&self, path: PathBuf) -> Result<usize, String> {
        let store = self.load()?;
        let count = store.servers.len();
        let export = ServerExport {
            version: 1,
            exported_at: unix_now(),
            servers: store.servers,
        };
        let raw = serde_json::to_string_pretty(&export)
            .map_err(|err| format!("Failed to serialize server export: {err}"))?;
        fs::write(path, raw).map_err(|err| format!("Failed to export servers: {err}"))?;
        Ok(count)
    }

    pub(crate) fn import_servers(&self, path: PathBuf) -> Result<ServerImportResult, String> {
        let raw = fs::read_to_string(path)
            .map_err(|err| format!("Failed to read server import: {err}"))?;
        let imported = parse_server_import(&raw)?;
        let mut store = self.load()?;
        let result = merge_imported_servers(&mut store, imported)?;
        self.save(&store)?;
        Ok(result)
    }

    pub(crate) fn set_active_server(
        &self,
        server_id: String,
    ) -> Result<SavedServerSummary, String> {
        let mut store = self.load()?;
        let mut active = None;
        for server in &mut store.servers {
            server.active = server.id == server_id;
            if server.active {
                active = Some(server_summary(server));
            }
        }
        if active.is_none() {
            return Err("Server not found.".to_string());
        }
        store.active_server_id = Some(server_id);
        self.save(&store)?;
        Ok(active.expect("active server checked above"))
    }

    pub(crate) fn delete_server(&self, server_id: String) -> Result<(), String> {
        let mut store = self.load()?;
        let old_len = store.servers.len();
        let was_active = store.active_server_id.as_ref() == Some(&server_id)
            || store
                .servers
                .iter()
                .any(|server| server.id == server_id && server.active);
        store.servers.retain(|server| server.id != server_id);
        if store.servers.len() == old_len {
            return Err("Server not found.".to_string());
        }
        if was_active {
            store.active_server_id = store.servers.first().map(|server| server.id.clone());
        }
        for server in &mut store.servers {
            server.active = Some(&server.id) == store.active_server_id.as_ref();
        }
        self.save(&store)
    }

    pub(crate) fn active_server(&self) -> Result<SavedServer, String> {
        let store = self.load()?;
        let active_id = store.active_server_id.clone();
        store
            .servers
            .into_iter()
            .find(|server| Some(&server.id) == active_id.as_ref() || server.active)
            .ok_or_else(|| "No active server. Add or select a server first.".to_string())
    }

    pub(crate) fn server_by_id(&self, server_id: &str) -> Result<SavedServer, String> {
        self.load()?
            .servers
            .into_iter()
            .find(|server| server.id == server_id)
            .ok_or_else(|| "Server not found.".to_string())
    }

    pub(crate) fn settings(&self) -> Result<AppSettings, String> {
        Ok(self.load()?.settings)
    }

    pub(crate) fn save_settings(&self, settings: AppSettings) -> Result<AppSettings, String> {
        let mut store = self.load()?;
        store.settings = settings.clone();
        self.save(&store)?;
        Ok(settings)
    }

    pub(crate) fn remember_recent_play(
        &self,
        server_id: &str,
        item_id: &str,
    ) -> Result<(), String> {
        let mut store = self.load()?;
        let ids = store.recent_plays.remove(server_id).unwrap_or_default();
        store
            .recent_plays
            .insert(server_id.to_string(), updated_recent_plays(ids, item_id));
        self.save(&store)
    }

    pub(crate) fn recent_play_ids(&self, server_id: &str) -> Result<Vec<String>, String> {
        Ok(self
            .load()?
            .recent_plays
            .get(server_id)
            .cloned()
            .unwrap_or_default())
    }

    pub(crate) fn save_playback_preference(
        &self,
        server_id: &str,
        key: String,
        preference: PlaybackPreference,
    ) -> Result<(), String> {
        let mut store = self.load()?;
        store
            .playback_preferences
            .entry(server_id.to_string())
            .or_default()
            .insert(key, preference);
        self.save(&store)
    }

    pub(crate) fn playback_preferences(
        &self,
        server_id: &str,
    ) -> Result<std::collections::HashMap<String, PlaybackPreference>, String> {
        Ok(self
            .load()?
            .playback_preferences
            .get(server_id)
            .cloned()
            .unwrap_or_default())
    }

    pub(crate) fn playback_preference(
        &self,
        server_id: &str,
        key: &str,
    ) -> Result<Option<PlaybackPreference>, String> {
        let mut store = self.load()?;
        Ok(store
            .playback_preferences
            .get_mut(server_id)
            .and_then(|preferences| preferences.remove(key)))
    }

    fn store_path(&self) -> PathBuf {
        self.data_dir.join(STORE_FILE)
    }

    fn load(&self) -> Result<ServerStore, String> {
        let path = self.store_path();
        if !path.exists() {
            return Ok(ServerStore::default());
        }
        let raw = fs::read_to_string(&path)
            .map_err(|err| format!("Failed to read saved servers: {err}"))?;
        serde_json::from_str(&raw).map_err(|err| format!("Failed to parse saved servers: {err}"))
    }

    fn save(&self, store: &ServerStore) -> Result<(), String> {
        let raw = serde_json::to_string_pretty(store)
            .map_err(|err| format!("Failed to serialize saved servers: {err}"))?;
        fs::write(self.store_path(), raw).map_err(|err| format!("Failed to save servers: {err}"))
    }
}

pub(crate) fn playback_preference_key(series_id: Option<&str>, item_id: &str) -> String {
    match series_id.filter(|value| !value.is_empty()) {
        Some(series_id) => format!("series:{series_id}"),
        None => format!("item:{item_id}"),
    }
}

pub(crate) fn server_summary(server: &SavedServer) -> SavedServerSummary {
    SavedServerSummary {
        id: server.id.clone(),
        name: server.name.clone(),
        url: server.url.clone(),
        username: server.username.clone(),
        active: server.active,
        use_system_proxy: server.use_system_proxy,
        icon_url: server.icon_url.clone(),
        icon_name: server.icon_name.clone(),
        movie_count: None,
        series_count: None,
        episode_count: None,
    }
}

fn clean_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn updated_recent_plays(mut ids: Vec<String>, item_id: &str) -> Vec<String> {
    ids.retain(|id| id != item_id);
    ids.insert(0, item_id.to_string());
    ids.truncate(RECENT_PLAY_LIMIT);
    ids
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn parse_server_import(raw: &str) -> Result<Vec<SavedServer>, String> {
    let value: Value =
        serde_json::from_str(raw).map_err(|err| format!("Failed to parse server import: {err}"))?;
    let servers = match value {
        Value::Array(servers) => Value::Array(servers),
        Value::Object(mut object) => {
            if let Some(version) = object.get("version").and_then(Value::as_u64) {
                if version != 1 {
                    return Err(format!("Unsupported server export version: {version}"));
                }
            }
            object
                .remove("servers")
                .ok_or_else(|| "Server import does not contain a servers list.".to_string())?
        }
        _ => return Err("Server import must be a JSON object or array.".to_string()),
    };
    serde_json::from_value(servers)
        .map_err(|err| format!("Failed to decode imported servers: {err}"))
}

fn merge_imported_servers(
    store: &mut ServerStore,
    imported_servers: Vec<SavedServer>,
) -> Result<ServerImportResult, String> {
    if imported_servers.is_empty() {
        return Err("Server import does not contain any servers.".to_string());
    }
    for server in &imported_servers {
        validate_imported_server(server)?;
    }

    let imported = imported_servers.len();
    let active_imported_id = imported_servers
        .iter()
        .find(|server| server.active)
        .map(|server| server.id.clone());
    let mut added = 0;
    let mut updated = 0;

    for server in imported_servers {
        if let Some(existing) = store
            .servers
            .iter_mut()
            .find(|existing| existing.id == server.id)
        {
            *existing = server;
            updated += 1;
        } else {
            store.servers.push(server);
            added += 1;
        }
    }

    if let Some(active_id) = active_imported_id {
        store.active_server_id = Some(active_id);
    } else if store.active_server_id.is_none() {
        store.active_server_id = store.servers.first().map(|server| server.id.clone());
    }
    let active_id = store.active_server_id.clone();
    for server in &mut store.servers {
        server.active = Some(&server.id) == active_id.as_ref();
    }

    Ok(ServerImportResult {
        imported,
        added,
        updated,
    })
}

fn validate_imported_server(server: &SavedServer) -> Result<(), String> {
    if server.id.trim().is_empty() {
        return Err("Imported server has no id.".to_string());
    }
    if server.name.trim().is_empty() {
        return Err("Imported server has no name.".to_string());
    }
    if server.url.trim().is_empty() {
        return Err("Imported server has no URL.".to_string());
    }
    if server.user_id.trim().is_empty() {
        return Err("Imported server has no user id.".to_string());
    }
    if server.access_token.trim().is_empty() {
        return Err("Imported server has no access token.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_server(id: &str, active: bool) -> SavedServer {
        SavedServer {
            id: id.to_string(),
            name: id.to_string(),
            url: format!("http://127.0.0.1:8096/{id}"),
            username: "user".to_string(),
            user_id: "user-id".to_string(),
            access_token: "token".to_string(),
            active,
            saved_at: 0,
            use_system_proxy: true,
            proxy_url: None,
            icon_url: None,
            icon_name: None,
        }
    }

    #[test]
    fn recent_plays_are_deduplicated_and_limited() {
        let mut ids = (0..60).map(|index| index.to_string()).collect::<Vec<_>>();
        ids = updated_recent_plays(ids, "42");

        assert_eq!(ids.first().map(String::as_str), Some("42"));
        assert_eq!(ids.len(), RECENT_PLAY_LIMIT);
        assert_eq!(ids.iter().filter(|id| id.as_str() == "42").count(), 1);
    }

    #[test]
    fn playback_preference_key_prefers_series_then_item() {
        assert_eq!(
            playback_preference_key(Some("series-1"), "episode-1"),
            "series:series-1"
        );
        assert_eq!(playback_preference_key(None, "movie-1"), "item:movie-1");
    }

    #[test]
    fn app_store_round_trips_servers_and_settings() {
        let dir = std::env::temp_dir().join(format!("zplayer-store-test-{}", unix_now()));
        let store = AppStore::new(&dir).unwrap();
        let server = SavedServer {
            id: "server-1".into(),
            name: "Server".into(),
            url: "http://127.0.0.1:8096".into(),
            username: "user".into(),
            user_id: "user-id".into(),
            access_token: "token".into(),
            active: false,
            saved_at: 0,
            use_system_proxy: true,
            proxy_url: None,
            icon_url: None,
            icon_name: None,
        };

        store.save_server(server).unwrap();
        assert_eq!(store.list_servers().unwrap().len(), 1);
        assert_eq!(store.active_server().unwrap().id, "server-1");
        let mut settings = store.settings().unwrap();
        settings.default_volume = 37;
        store.save_settings(settings).unwrap();
        assert_eq!(store.settings().unwrap().default_volume, 37);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_server_returns_summary_without_reloading_store() {
        let dir = std::env::temp_dir().join(format!("zplayer-store-summary-test-{}", unix_now()));
        let store = AppStore::new(&dir).unwrap();
        let server = SavedServer {
            id: "server-1".into(),
            name: "Server".into(),
            url: "http://127.0.0.1:8096".into(),
            username: "user".into(),
            user_id: "user-id".into(),
            access_token: "token".into(),
            active: false,
            saved_at: 0,
            use_system_proxy: true,
            proxy_url: None,
            icon_url: None,
            icon_name: None,
        };

        let summary = store.save_server(server).unwrap();

        assert_eq!(summary.id, "server-1");
        assert_eq!(summary.name, "Server");
        assert_eq!(summary.url, "http://127.0.0.1:8096");
        assert!(summary.active);
        assert_eq!(store.servers().unwrap().len(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn export_import_round_trips_servers_and_active_server() {
        let dir = std::env::temp_dir().join(format!("zplayer-export-test-{}", unix_now()));
        let store = AppStore::new(&dir).unwrap();
        store.save_server(test_server("server-1", false)).unwrap();
        store.save_server(test_server("server-2", false)).unwrap();
        let path = dir.join("servers-export.json");

        assert_eq!(store.export_servers(path.clone()).unwrap(), 2);

        let imported = AppStore::new(&dir.join("imported")).unwrap();
        let result = imported.import_servers(path).unwrap();
        assert_eq!(result.imported, 2);
        assert_eq!(result.added, 2);
        assert_eq!(result.updated, 0);
        assert_eq!(imported.servers().unwrap().len(), 2);
        assert_eq!(imported.active_server().unwrap().id, "server-2");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn deleting_active_server_falls_back_to_next() {
        let dir = std::env::temp_dir().join(format!("zplayer-delete-test-{}", unix_now()));
        let store = AppStore::new(&dir).unwrap();
        store.save_server(test_server("server-1", false)).unwrap();
        store.save_server(test_server("server-2", false)).unwrap();

        store.delete_server("server-2".to_string()).unwrap();

        assert_eq!(store.active_server().unwrap().id, "server-1");
        assert_eq!(store.servers().unwrap().len(), 1);
        let _ = fs::remove_dir_all(dir);
    }
}
