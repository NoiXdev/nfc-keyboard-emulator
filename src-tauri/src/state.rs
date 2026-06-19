use crate::config::Config;
use crate::reader::WorkerCommand;
use crate::scan::ScanRecord;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;

pub const STORE_FILE: &str = "settings.json";
pub const CONFIG_KEY: &str = "config";

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub typing_enabled: Arc<AtomicBool>,
    pub worker_tx: Mutex<Option<Sender<WorkerCommand>>>,
    pub history: Arc<Mutex<VecDeque<ScanRecord>>>,
    pub readers: Arc<Mutex<Vec<String>>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let typing = config.typing_enabled_on_start;
        Self {
            config: Arc::new(Mutex::new(config)),
            typing_enabled: Arc::new(AtomicBool::new(typing)),
            worker_tx: Mutex::new(None),
            history: Arc::new(Mutex::new(VecDeque::new())),
            readers: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

pub fn load_config<R: Runtime>(app: &AppHandle<R>) -> Config {
    let Ok(store) = app.store(STORE_FILE) else {
        return Config::default();
    };
    store
        .get(CONFIG_KEY)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

pub fn save_config<R: Runtime>(app: &AppHandle<R>, config: &Config) {
    if let Ok(store) = app.store(STORE_FILE) {
        if let Ok(value) = serde_json::to_value(config) {
            store.set(CONFIG_KEY, value);
            let _ = store.save();
        }
    }
}
