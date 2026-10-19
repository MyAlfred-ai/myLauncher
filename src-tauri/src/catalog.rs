//! The catalog describes each MyAlfred app: where its code lives, how its
//! version is read, and the shell steps that install / update / run it.
//! A copy is embedded at build time; a newer one can be fetched from GitHub.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{bail, Result};

pub const SUPPORTED_SCHEMA: u32 = 1;
pub const EMBEDDED: &str = include_str!("../../catalog/apps.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub schema: u32,
    pub updated: String,
    pub apps: Vec<AppDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDef {
    pub id: String,
    pub name: String,
    pub tagline: String,
    pub description: String,
    pub icon: String,
    pub color: String,
    #[serde(default)]
    pub features: Vec<String>,
    pub repo: RepoDef,
    pub version: VersionDef,
    #[serde(default)]
    pub changelog: Option<ChangelogDef>,
    #[serde(default)]
    pub requirements: Vec<String>,
    #[serde(default)]
    pub dotenv_quote: DotenvQuote,
    pub targets: Targets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoDef {
    pub owner: String,
    pub name: String,
    #[serde(default = "default_branch")]
    pub branch: String,
}

fn default_branch() -> String {
    "main".into()
}

impl RepoDef {
    pub fn clone_url(&self) -> String {
        format!("https://github.com/{}/{}.git", self.owner, self.name)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VersionSource {
    /// Latest GitHub release.
    Releases,
    /// Highest git tag matching `pattern`.
    Tags,
    /// A file on the branch (`VERSION`, or a JSON file with `jsonKey`).
    File,
    /// Head commit of the branch.
    Commit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDef {
    pub source: VersionSource,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub json_key: Option<String>,
    #[serde(default)]
    pub pattern: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChangelogSource {
    File,
    Releases,
    Commits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangelogDef {
    pub source: ChangelogSource,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DotenvQuote {
    #[default]
    None,
    Single,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Targets {
    /// Installation on this computer, the only one the launcher performs:
    /// installs on a server are done by MyAlfred.ai.
    #[serde(default)]
    pub local: Option<TargetDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDef {
    /// Short warning or hint shown before installing.
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub experimental: bool,
    /// Directory (relative to the repo) where `docker compose` runs.
    #[serde(default)]
    pub workdir: Option<String>,
    /// URL template; `{{KEY}}` is replaced by form values.
    pub url: String,
    /// Rough duration of a first install, shown to the user.
    #[serde(default)]
    pub duration: Option<String>,
    #[serde(default)]
    pub fields: Vec<FieldDef>,
    pub install: Vec<Step>,
    pub update: Vec<Step>,
    #[serde(default)]
    pub start: Vec<Step>,
    #[serde(default)]
    pub stop: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub label: String,
    /// Shell snippet. `@sync` fetches the code at the target version.
    pub run: String,
}

impl Step {
    pub fn is_sync(&self) -> bool {
        self.run.trim() == "@sync"
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldKind {
    Text,
    Email,
    Url,
    Number,
    Secret,
    Password,
    Select,
}

impl FieldKind {
    /// Secret values are only passed to the install script, never stored.
    pub fn is_secret(self) -> bool {
        matches!(self, FieldKind::Secret | FieldKind::Password)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldDef {
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub help: Option<String>,
    /// `password` or `hex32`: the form offers a "generate" button.
    #[serde(default)]
    pub generate: Option<String>,
    #[serde(default)]
    pub options: Vec<SelectOption>,
    #[serde(default)]
    pub min_length: Option<usize>,
    /// Only shown (and required) when another field has one of these values.
    #[serde(default)]
    pub when: Option<FieldCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldCondition {
    pub key: String,
    #[serde(rename = "in")]
    pub values: Vec<String>,
}

impl FieldDef {
    pub fn is_active(&self, values: &std::collections::BTreeMap<String, String>) -> bool {
        match &self.when {
            None => true,
            Some(c) => values
                .get(&c.key)
                .is_some_and(|v| c.values.iter().any(|x| x == v)),
        }
    }
}

impl Catalog {
    pub fn parse(json: &str) -> Result<Catalog> {
        let catalog: Catalog = serde_json::from_str(json)?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn app(&self, id: &str) -> Option<&AppDef> {
        self.apps.iter().find(|a| a.id == id)
    }

    fn validate(&self) -> Result<()> {
        if self.schema > SUPPORTED_SCHEMA {
            bail!(
                "Catalogue au format {} : mettez à jour le launcher (format pris en charge : {}).",
                self.schema,
                SUPPORTED_SCHEMA
            );
        }
        let id_re = regex_lite::Regex::new(r"^[a-z][a-z0-9-]{1,31}$").unwrap();
        let key_re = regex_lite::Regex::new(r"^[A-Z][A-Z0-9_]{0,63}$").unwrap();
        let slug_re = regex_lite::Regex::new(r"^[A-Za-z0-9_.-]+$").unwrap();
        let mut ids = HashSet::new();
        for app in &self.apps {
            if !id_re.is_match(&app.id) || !ids.insert(app.id.clone()) {
                bail!(
                    "Catalogue : identifiant d'application invalide ou en double « {} ».",
                    app.id
                );
            }
            for s in [&app.repo.owner, &app.repo.name, &app.repo.branch] {
                if !slug_re.is_match(s) {
                    bail!("Catalogue : dépôt invalide pour {}.", app.id);
                }
            }
            if app.version.source == VersionSource::File && app.version.path.is_none() {
                bail!(
                    "Catalogue : {} lit sa version dans un fichier sans préciser lequel.",
                    app.id
                );
            }
            if let Some(p) = &app.version.path {
                check_rel_path(p, &app.id)?;
            }
            if let Some(k) = &app.version.json_key {
                if !slug_re.is_match(k) {
                    bail!("Catalogue : clé de version invalide pour {}.", app.id);
                }
            }
            if let Some(t) = &app.targets.local {
                if let Some(w) = &t.workdir {
                    check_rel_path(w, &app.id)?;
                }
                let mut keys = HashSet::new();
                for f in &t.fields {
                    if !key_re.is_match(&f.key) || !keys.insert(f.key.clone()) {
                        bail!("Catalogue : champ invalide « {} » pour {}.", f.key, app.id);
                    }
                }
                if t.install.is_empty() || t.update.is_empty() {
                    bail!(
                        "Catalogue : étapes d'installation manquantes pour {}.",
                        app.id
                    );
                }
            }
        }
        Ok(())
    }
}

fn check_rel_path(p: &str, app: &str) -> Result<()> {
    let path = Path::new(p);
    if path.is_absolute() || p.split(['/', '\\']).any(|c| c == "..") || p.contains('\'') {
        bail!("Catalogue : chemin invalide « {p} » pour {app}.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_is_valid() {
        let c = Catalog::parse(EMBEDDED).expect("embedded catalog must parse");
        let ids: Vec<_> = c.apps.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["mia", "mybrief", "mytrix", "mybrain"]);
        for app in &c.apps {
            let local = app.targets.local.as_ref().expect("local target");
            assert!(
                local.install.iter().any(Step::is_sync),
                "{} install must sync",
                app.id
            );
            assert!(
                local.update.iter().any(Step::is_sync),
                "{} update must sync",
                app.id
            );
        }
    }

    #[test]
    fn rejects_bad_paths() {
        assert!(check_rel_path("../etc", "x").is_err());
        assert!(check_rel_path("/etc/passwd", "x").is_err());
        assert!(check_rel_path("docs/NOUVEAUTES.md", "x").is_ok());
    }
}
