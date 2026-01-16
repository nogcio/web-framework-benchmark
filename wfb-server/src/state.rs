use std::sync::{Arc, RwLock};
use wfb_storage::{Config, Storage};

pub struct AppState {
    pub storage: Arc<Storage>,
    pub config: Arc<RwLock<Config>>,
}
