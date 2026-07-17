use crate::models::{
    AppSettings, PlaybackPreference, SavedServer, SavedServerSummary, ServerExport,
    ServerImportResult, ServerStore,
};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

const STORE_FILE: &str = "servers.json";
const RECENT_PLAY_LIMIT: usize = 50;

pub(crate) fn save_server(
    app: &AppHandle,
    mut saved: SavedServer,
) -> Result<SavedServerSummary, String> {
    let mut store = load_store(app)?;
    let id = saved.id.clone();
    saved.active = true;

    for server in &mut store.servers {
        server.active = false;
    }

    if let Some(existing) = store.servers.iter_mut().find(|server| server.id == id) {
        *existing = saved;
    } else {
        store.servers.push(saved);
    }

    store.active_server_id = Some(id.clone());
    save_store(app, &store)?;
    let server = store
        .servers
        .iter()
        .find(|server| server.id == id)
        .ok_or_else(|| "Saved server was not found after write.".to_string())?;
    Ok(server_summary(server))
}

pub(crate) fn list_servers(app: &AppHandle) -> Result<Vec<SavedServerSummary>, String> {
    let store = load_store(app)?;
    Ok(store.servers.iter().map(server_summary).collect())
}

pub(crate) fn servers(app: &AppHandle) -> Result<Vec<SavedServer>, String> {
    Ok(load_store(app)?.servers)
}

pub(crate) fn export_servers(app: &AppHandle, path: PathBuf) -> Result<usize, String> {
    let store = load_store(app)?;
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

pub(crate) fn import_servers(app: &AppHandle, path: PathBuf) -> Result<ServerImportResult, String> {
    let raw =
        fs::read_to_string(path).map_err(|err| format!("Failed to read server import: {err}"))?;
    let imported = parse_server_import(&raw)?;
    let mut store = load_store(app)?;
    let result = merge_imported_servers(&mut store, imported)?;
    save_store(app, &store)?;
    Ok(result)
}

pub(crate) fn set_active_server(
    app: &AppHandle,
    server_id: String,
) -> Result<SavedServerSummary, String> {
    let mut store = load_store(app)?;
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
    save_store(app, &store)?;
    Ok(active.expect("active server checked above"))
}

pub(crate) fn delete_server(app: &AppHandle, server_id: String) -> Result<(), String> {
    let mut store = load_store(app)?;
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
    save_store(app, &store)
}

pub(crate) fn active_server(app: &AppHandle) -> Result<SavedServer, String> {
    let store = load_store(app)?;
    store
        .servers
        .iter()
        .find(|server| Some(&server.id) == store.active_server_id.as_ref() || server.active)
        .cloned()
        .ok_or_else(|| "No active server. Add or select a server first.".to_string())
}

pub(crate) fn server_by_id(app: &AppHandle, server_id: &str) -> Result<SavedServer, String> {
    load_store(app)?
        .servers
        .into_iter()
        .find(|server| server.id == server_id)
        .ok_or_else(|| "Server not found.".to_string())
}

pub(crate) fn settings(app: &AppHandle) -> Result<AppSettings, String> {
    Ok(load_store(app)?.settings)
}

pub(crate) fn save_settings(app: &AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    let mut store = load_store(app)?;
    store.settings = settings.clone();
    save_store(app, &store)?;
    Ok(settings)
}

fn updated_recent_plays(mut ids: Vec<String>, item_id: &str) -> Vec<String> {
    ids.retain(|id| id != item_id);
    ids.insert(0, item_id.to_string());
    ids.truncate(RECENT_PLAY_LIMIT);
    ids
}

pub(crate) fn remember_recent_play(
    app: &AppHandle,
    server_id: &str,
    item_id: &str,
) -> Result<(), String> {
    let mut store = load_store(app)?;
    let ids = store.recent_plays.remove(server_id).unwrap_or_default();
    store
        .recent_plays
        .insert(server_id.to_string(), updated_recent_plays(ids, item_id));
    save_store(app, &store)
}

pub(crate) fn recent_play_ids(app: &AppHandle, server_id: &str) -> Result<Vec<String>, String> {
    Ok(load_store(app)?
        .recent_plays
        .get(server_id)
        .cloned()
        .unwrap_or_default())
}

pub(crate) fn playback_preference_key(series_id: Option<&str>, item_id: &str) -> String {
    match series_id.filter(|value| !value.is_empty()) {
        Some(series_id) => format!("series:{series_id}"),
        None => format!("item:{item_id}"),
    }
}

pub(crate) fn save_playback_preference(
    app: &AppHandle,
    server_id: &str,
    key: String,
    preference: PlaybackPreference,
) -> Result<(), String> {
    let mut store = load_store(app)?;
    store
        .playback_preferences
        .entry(server_id.to_string())
        .or_default()
        .insert(key, preference);
    save_store(app, &store)
}

pub(crate) fn playback_preferences(
    app: &AppHandle,
    server_id: &str,
) -> Result<std::collections::HashMap<String, PlaybackPreference>, String> {
    Ok(load_store(app)?
        .playback_preferences
        .get(server_id)
        .cloned()
        .unwrap_or_default())
}

pub(crate) fn server_summary(server: &SavedServer) -> SavedServerSummary {
    SavedServerSummary {
        id: server.id.clone(),
        name: server.name.clone(),
        url: server.url.clone(),
        username: server.username.clone(),
        active: server.active,
        use_system_proxy: server.use_system_proxy,
        movie_count: None,
        series_count: None,
    }
}

pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("Failed to locate app data directory: {err}"))?;
    fs::create_dir_all(&dir)
        .map_err(|err| format!("Failed to create app data directory: {err}"))?;
    Ok(dir.join(STORE_FILE))
}

fn load_store(app: &AppHandle) -> Result<ServerStore, String> {
    let path = store_path(app)?;
    if !path.exists() {
        return Ok(ServerStore::default());
    }
    let raw =
        fs::read_to_string(&path).map_err(|err| format!("Failed to read saved servers: {err}"))?;
    serde_json::from_str(&raw).map_err(|err| format!("Failed to parse saved servers: {err}"))
}

fn save_store(app: &AppHandle, store: &ServerStore) -> Result<(), String> {
    let path = store_path(app)?;
    let raw = serde_json::to_string_pretty(store)
        .map_err(|err| format!("Failed to serialize saved servers: {err}"))?;
    fs::write(path, raw).map_err(|err| format!("Failed to save servers: {err}"))
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
    }
    let active_missing = match &store.active_server_id {
        Some(active_id) => !store.servers.iter().any(|server| server.id == *active_id),
        None => true,
    };
    if active_missing {
        store.active_server_id = store.servers.first().map(|server| server.id.clone());
    }
    for server in &mut store.servers {
        server.active = store.active_server_id.as_ref() == Some(&server.id);
    }

    Ok(ServerImportResult {
        imported,
        added,
        updated,
    })
}

fn validate_imported_server(server: &SavedServer) -> Result<(), String> {
    if server.id.trim().is_empty() {
        return Err("Imported server is missing an id.".to_string());
    }
    if server.name.trim().is_empty() {
        return Err(format!("Imported server {} is missing a name.", server.id));
    }
    if server.url.trim().is_empty() {
        return Err(format!("Imported server {} is missing a URL.", server.id));
    }
    if server.username.trim().is_empty() {
        return Err(format!(
            "Imported server {} is missing a username.",
            server.id
        ));
    }
    if server.user_id.trim().is_empty() {
        return Err(format!(
            "Imported server {} is missing a user id.",
            server.id
        ));
    }
    if server.access_token.trim().is_empty() {
        return Err(format!(
            "Imported server {} is missing an access token.",
            server.id
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn saved_server(id: &str, active: bool) -> SavedServer {
        SavedServer {
            id: id.to_string(),
            name: format!("Server {id}"),
            url: format!("http://{id}.example.test"),
            username: "user".to_string(),
            user_id: format!("user-{id}"),
            access_token: format!("token-{id}"),
            active,
            saved_at: 1,
            use_system_proxy: true,
        }
    }

    #[test]
    fn server_summary_includes_proxy_choice() {
        let summary = server_summary(&SavedServer {
            id: "server".to_string(),
            name: "Server".to_string(),
            url: "http://127.0.0.1:8096".to_string(),
            username: "user".to_string(),
            user_id: "user-id".to_string(),
            access_token: "token".to_string(),
            active: true,
            saved_at: 0,
            use_system_proxy: false,
        });

        assert!(!summary.use_system_proxy);
    }

    #[test]
    fn updated_recent_plays_dedupes_and_trims() {
        let ids = (0..55).map(|index| format!("item-{index}")).collect();
        let updated = updated_recent_plays(ids, "item-20");

        assert_eq!(updated.first(), Some(&"item-20".to_string()));
        assert_eq!(updated.len(), 50);
        assert_eq!(updated.iter().filter(|id| *id == "item-20").count(), 1);
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
    fn parse_server_import_reads_export_wrapper() {
        let raw = serde_json::to_string(&ServerExport {
            version: 1,
            exported_at: 1,
            servers: vec![saved_server("server-a", true)],
        })
        .unwrap();

        let servers = parse_server_import(&raw).unwrap();

        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].id, "server-a");
    }

    #[test]
    fn merge_imported_servers_updates_existing_and_sets_active() {
        let mut store = ServerStore {
            active_server_id: Some("server-a".to_string()),
            servers: vec![saved_server("server-a", true)],
            ..ServerStore::default()
        };
        let mut updated = saved_server("server-a", false);
        updated.name = "Updated".to_string();

        let result =
            merge_imported_servers(&mut store, vec![updated, saved_server("server-b", true)])
                .unwrap();

        assert_eq!(
            result,
            ServerImportResult {
                imported: 2,
                added: 1,
                updated: 1,
            }
        );
        assert_eq!(store.servers.len(), 2);
        assert_eq!(store.active_server_id.as_deref(), Some("server-b"));
        assert_eq!(
            store
                .servers
                .iter()
                .find(|server| server.id == "server-a")
                .unwrap()
                .name,
            "Updated"
        );
        assert!(
            store
                .servers
                .iter()
                .find(|server| server.id == "server-b")
                .unwrap()
                .active
        );
    }
}
