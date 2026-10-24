use super::{AppPaths, AppStore};
use crate::api;
use crate::models::{
    AppSettings, LoginInput, LoginResult, SaveServerInput, SaveSettingsInput, SavedServer,
    SavedServerSummary, ServerIconInput, ServerImportResult,
};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug)]
pub(crate) struct ServerService {
    store: AppStore,
}

impl ServerService {
    pub(crate) fn new(paths: &AppPaths) -> Result<Self, String> {
        Ok(Self {
            store: paths.store()?,
        })
    }

    pub(crate) fn list(&self) -> Result<Vec<SavedServerSummary>, String> {
        self.store.list_servers()
    }

    pub(crate) fn save(&self, input: SaveServerInput) -> Result<SavedServerSummary, String> {
        self.store.save_server(SavedServer {
            id: input.id,
            name: input.name,
            url: api::normalize_server_url(&input.url)?,
            username: input.username,
            user_id: input.user_id,
            access_token: input.access_token,
            active: false,
            saved_at: crate::storage::unix_now(),
            use_system_proxy: input.use_system_proxy,
            proxy_url: input.proxy_url,
            icon_url: input.icon_url,
            icon_name: input.icon_name,
        })
    }

    pub(crate) fn update_icon(&self, input: ServerIconInput) -> Result<SavedServerSummary, String> {
        self.store.update_server_icon(input)
    }

    pub(crate) fn set_active(&self, server_id: String) -> Result<SavedServerSummary, String> {
        self.store.set_active_server(server_id)
    }

    pub(crate) fn delete(&self, server_id: String) -> Result<(), String> {
        self.store.delete_server(server_id)
    }

    pub(crate) fn export(&self, path: PathBuf) -> Result<usize, String> {
        self.store.export_servers(path)
    }

    pub(crate) fn import(&self, path: PathBuf) -> Result<ServerImportResult, String> {
        self.store.import_servers(path)
    }

    pub(crate) fn settings(&self) -> Result<AppSettings, String> {
        self.store.settings()
    }

    pub(crate) fn save_settings(&self, input: SaveSettingsInput) -> Result<AppSettings, String> {
        self.store
            .save_settings(crate::models::normalize_settings(input))
    }

    pub(crate) fn login(&self, input: LoginInput) -> Result<LoginResult, String> {
        let url = api::normalize_server_url(&input.url)?;
        let client = api::http_client_with_proxy_url(
            input.proxy_url.as_deref(),
            input.use_system_proxy,
            Duration::from_secs(20),
        )?;
        let auth = api::authenticate_by_name(
            &client,
            &url,
            &input.server_type,
            input.username.trim(),
            &input.password,
        )?;
        let public_name = if input.name.trim().is_empty() {
            api::fetch_server_public_info(&client, &url)
                .map(|info| info.server_name.or(info.product_name))
                .unwrap_or_default()
        } else {
            None
        };
        let username = auth
            .user
            .name
            .unwrap_or_else(|| input.username.trim().to_string());
        Ok(LoginResult {
            id: api::server_id(&url, &auth.user.id),
            name: api::clean_name(&input.name, public_name.as_deref().unwrap_or(""), &url),
            url,
            username,
            user_id: auth.user.id,
            access_token: auth.access_token,
            use_system_proxy: input.use_system_proxy,
            proxy_url: input.proxy_url,
        })
    }
}
