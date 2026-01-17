use std::sync::{Arc, RwLock, RwLockReadGuard};
use wfb_storage::{Config, Storage};

pub struct AppState {
    pub storage: Arc<Storage>,
    pub config: Arc<RwLock<Config>>,
}

impl AppState {
    pub fn config_read(&self) -> RwLockReadGuard<'_, Config> {
        self.config.read().unwrap_or_else(|err| err.into_inner())
    }
}
