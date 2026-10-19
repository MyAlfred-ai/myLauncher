use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::catalog::{Catalog, EMBEDDED};
use crate::error::Result;
use crate::github::Latest;
use crate::ops::OpManager;
use crate::secrets;
use crate::store::{load_json, save_json, InstallState, Settings, Store};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestState {
    pub latest: Option<Latest>,
    pub error: Option<String>,
    pub checked_at: String,
}

pub struct CatalogState {
    pub catalog: Catalog,
    /// `embedded`, `cache` or `remote`.
    pub source: String,
    pub fetched_at: Option<String>,
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub settings: RwLock<Settings>,
    pub store: RwLock<Store>,
    pub catalog: RwLock<CatalogState>,
    pub latest: RwLock<HashMap<String, LatestState>>,
    pub last_check: RwLock<Option<String>>,
    pub token: RwLock<Option<String>>,
    pub ops: OpManager,
}

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

impl AppState {
    pub fn load(app: &AppHandle) -> Result<AppState> {
        let paths = app.path();
        let data_dir = paths.app_data_dir()?;
        let log_dir = paths
            .app_log_dir()
            .unwrap_or_else(|_| data_dir.join("logs"));
        let home_dir = paths.home_dir()?;
        std::fs::create_dir_all(&data_dir)?;

        let mut settings: Settings = load_json(&data_dir.join("settings.json"));
        if settings.install_dir.trim().is_empty() {
            settings.install_dir = home_dir.join("MyAlfred").to_string_lossy().into_owned();
        }
        let mut store: Store = load_json(&data_dir.join("state.json"));
        // Launchers up to 0.1.1 could install on servers: those installations
        // are forgotten (nothing is changed on the servers themselves).
        store.installations.retain(|i| i.is_local());
        // No operation survives a restart: an install left half-way can be resumed.
        for inst in &mut store.installations {
            if inst.state == InstallState::Installing {
                inst.state = InstallState::Failed;
            }
        }
        let catalog = load_catalog(&data_dir);

        Ok(AppState {
            data_dir,
            log_dir,
            settings: RwLock::new(settings),
            store: RwLock::new(store),
            catalog: RwLock::new(catalog),
            latest: RwLock::new(HashMap::new()),
            last_check: RwLock::new(None),
            token: RwLock::new(secrets::get_token()),
            ops: OpManager::default(),
        })
    }

    pub fn save_store(&self) -> Result<()> {
        let store = self.store.read().unwrap().clone();
        save_json(&self.data_dir.join("state.json"), &store)
    }

    pub fn save_settings(&self) -> Result<()> {
        let settings = self.settings.read().unwrap().clone();
        save_json(&self.data_dir.join("settings.json"), &settings)
    }

    pub fn catalog_cache_path(&self) -> PathBuf {
        self.data_dir.join("catalog.json")
    }

    pub fn token(&self) -> Option<String> {
        self.token.read().unwrap().clone()
    }
}

/// Embedded catalog, unless a cached remote one is at least as recent.
fn load_catalog(data_dir: &std::path::Path) -> CatalogState {
    let embedded = Catalog::parse(EMBEDDED).expect("embedded catalog is validated by tests");
    let cached = std::fs::read_to_string(data_dir.join("catalog.json"))
        .ok()
        .and_then(|s| Catalog::parse(&s).ok());
    match cached {
        Some(c) if c.updated >= embedded.updated => CatalogState {
            catalog: c,
            source: "cache".into(),
            fetched_at: None,
        },
        _ => CatalogState {
            catalog: embedded,
            source: "embedded".into(),
            fetched_at: None,
        },
    }
}

pub fn state(app: &AppHandle) -> tauri::State<'_, AppState> {
    app.state::<AppState>()
}
