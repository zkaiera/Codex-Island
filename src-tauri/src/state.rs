use std::sync::{Arc, Mutex, RwLock};

use notify::RecommendedWatcher;
use crate::store::SessionStore;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<SessionStore>>,
    pub watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            store: Arc::new(RwLock::new(SessionStore::default())),
            watcher: Arc::new(Mutex::new(None)),
        }
    }
}
