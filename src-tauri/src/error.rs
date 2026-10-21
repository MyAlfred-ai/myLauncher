use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Msg(String),
    #[error("Erreur de fichier : {0}")]
    Io(#[from] std::io::Error),
    #[error("Données invalides : {0}")]
    Json(#[from] serde_json::Error),
    #[error("Erreur réseau : {0}")]
    Http(#[from] reqwest::Error),
    #[error("Erreur interne : {0}")]
    Tauri(#[from] tauri::Error),
    #[error("Erreur Git : {0}")]
    Git(#[from] git2::Error),
}

impl Error {
    pub fn msg(s: impl Into<String>) -> Self {
        Error::Msg(s.into())
    }
}

// Commands return errors to the UI as plain strings.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::error::Error::Msg(format!($($arg)*)))
    };
}
pub(crate) use bail;
