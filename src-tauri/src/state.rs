use std::sync::{Arc, Mutex, RwLock};

use crate::store::SessionStore;
use notify::RecommendedWatcher;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<RwLock<SessionStore>>,
    pub watcher: Arc<Mutex<Option<RecommendedWatcher>>>,
    pub panel_hover: Arc<Mutex<PanelHoverState>>,
}

#[derive(Default)]
pub struct PanelHoverState {
    pub island_hovered: bool,
    pub panel_hovered: bool,
    pub watch_active: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            store: Arc::new(RwLock::new(SessionStore::default())),
            watcher: Arc::new(Mutex::new(None)),
            panel_hover: Arc::new(Mutex::new(PanelHoverState::default())),
        }
    }
}
