//! Persistent state: launcher settings and installations.
//! Stored as JSON in the app data folder; never contains secrets.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::error::{bail, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// `system`, `light` or `dark`.
    pub theme: String,
    /// Parent folder of local installs.
    pub install_dir: String,
    pub auto_check: bool,
    pub check_interval_hours: u32,
    pub notify: bool,
    /// Where the catalog is refreshed from. The GitHub token is only sent
    /// to api.github.com / raw.githubusercontent.com.
    pub catalog_url: String,
}

pub const DEFAULT_CATALOG_URL: &str =
    "https://api.github.com/repos/MyAlfred-ai/myLauncher/contents/catalog/apps.json?ref=main";

impl Default for Settings {
    fn default() -> Self {
        Settings {
            theme: "system".into(),
            install_dir: String::new(),
            auto_check: true,
            check_interval_hours: 6,
            notify: true,
            catalog_url: DEFAULT_CATALOG_URL.into(),
        }
    }
}

/// Install folders: absolute, or `~/…`; no quotes or control characters.
pub fn check_dir(dir: &str) -> Result<()> {
    let ok_start =
        dir.starts_with('/') || dir.starts_with("~/") || dir == "~" || is_windows_abs(dir);
    if !ok_start
        || dir.contains(['\'', '"', '`', '$', '\n', '\r'])
        || dir.split(['/', '\\']).any(|c| c == "..")
    {
        bail!(
            "Dossier invalide : indiquez un chemin absolu (ou commençant par ~/) sans guillemets."
        );
    }
    Ok(())
}

fn is_windows_abs(dir: &str) -> bool {
    let b = dir.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstallState {
    Installing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub id: String,
    pub app_id: String,
    /// Always `local`. Launchers up to 0.1.1 could also record a server id:
    /// such installations are dropped when the state is loaded.
    #[serde(default = "local_target")]
    pub target: String,
    pub dir: String,
    /// Non-secret form values, reused by later operations.
    #[serde(default)]
    pub values: BTreeMap<String, String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    pub state: InstallState,
    pub installed_at: String,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub auto_update: bool,
    /// Version for which the user was last notified, to avoid repeats.
    #[serde(default)]
    pub notified_version: Option<String>,
}

fn local_target() -> String {
    "local".into()
}

impl Installation {
    pub fn is_local(&self) -> bool {
        self.target == "local"
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Store {
    pub installations: Vec<Installation>,
    /// Launcher version already announced by a notification.
    pub launcher_notified: Option<String>,
}

impl Store {
    pub fn installation(&self, id: &str) -> Option<&Installation> {
        self.installations.iter().find(|i| i.id == id)
    }
    pub fn installation_mut(&mut self, id: &str) -> Option<&mut Installation> {
        self.installations.iter_mut().find(|i| i.id == id)
    }
}

pub fn load_json<T: DeserializeOwned + Default>(path: &Path) -> T {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            log::warn!(
                "{} illisible ({e}), valeurs par défaut utilisées",
                path.display()
            );
            // Keep the unreadable file aside instead of overwriting it.
            let _ = fs::copy(path, path.with_extension("json.bak"));
            T::default()
        }),
        Err(_) => T::default(),
    }
}

/// Atomic write: temp file then rename.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp: PathBuf = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_dirs() {
        assert!(check_dir("~/myalfred").is_ok());
        assert!(check_dir("/opt/apps").is_ok());
        assert!(check_dir("C:\\Users\\me\\MyAlfred").is_ok());
        assert!(check_dir("relative/path").is_err());
        assert!(check_dir("/opt/../etc").is_err());
        assert!(check_dir("/opt/$(rm)").is_err());
    }
}
