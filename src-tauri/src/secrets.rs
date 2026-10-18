//! The GitHub token lives in the OS keychain (macOS Keychain, Windows
//! Credential Manager, Secret Service on Linux), never in a file.

use crate::error::{Error, Result};

const SERVICE: &str = "ai.myalfred.launcher";
const ACCOUNT: &str = "github-token";

fn entry() -> Result<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT)
        .map_err(|e| Error::msg(format!("Trousseau indisponible : {e}")))
}

pub fn get_token() -> Option<String> {
    match entry().and_then(|e| e.get_password().map_err(|err| Error::msg(err.to_string()))) {
        Ok(t) if !t.trim().is_empty() => Some(t),
        _ => None,
    }
}

pub fn set_token(token: &str) -> Result<()> {
    entry()?.set_password(token.trim()).map_err(|e| {
        Error::msg(format!(
            "Impossible d'enregistrer le token dans le trousseau : {e}"
        ))
    })
}

pub fn clear_token() -> Result<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(Error::msg(format!(
            "Impossible de supprimer le token : {e}"
        ))),
    }
}
