mod library;
mod playback;
mod server;

pub(crate) use library::LibraryService;
pub(crate) use playback::PlaybackService;
pub(crate) use server::ServerService;

use crate::storage::AppStore;
use dirs::{data_dir, home_dir};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub(crate) struct AppPaths {
    pub(crate) data_dir: PathBuf,
    pub(crate) resource_dir: PathBuf,
}

impl AppPaths {
    pub(crate) fn new(data_dir: impl Into<PathBuf>, resource_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            resource_dir: resource_dir.into(),
        }
    }

    pub(crate) fn from_environment() -> Result<Self, String> {
        let data_dir = data_dir()
            .map(|dir| dir.join("Zplayer"))
            .ok_or_else(|| "Failed to resolve the user data directory.".to_string())?;
        let resource_dir = std::env::current_dir()
            .map(|dir| dir.join("resources"))
            .or_else(|_| {
                home_dir()
                    .map(|dir| dir.join(".local/share/Zplayer"))
                    .ok_or_else(|| {
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "Failed to resolve the home directory.",
                        )
                    })
            })
            .map_err(|err| format!("Failed to resolve the resource directory: {err}"))?;
        Ok(Self::new(data_dir, resource_dir))
    }

    pub(crate) fn store(&self) -> Result<AppStore, String> {
        AppStore::new(&self.data_dir)
    }

    pub(crate) fn player(&self) -> crate::player::PlayerPaths {
        let settings = self.store().and_then(|store| store.settings()).ok();
        crate::player::PlayerPaths {
            libmpv_path: settings.and_then(|settings| settings.mpv_path.map(Into::into)),
            config_dir: self.resource_dir.join("mpv"),
            log_dir: self.data_dir.join("logs"),
        }
    }
}
