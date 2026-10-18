//! Reads versions, changelogs and the catalog from GitHub. Works without a
//! token for public repositories; private ones need a token (repo read).

use std::time::Duration;

use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::catalog::{AppDef, ChangelogSource, VersionSource};
use crate::error::{bail, Error, Result};
use crate::version;

const API: &str = "https://api.github.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Latest {
    pub version: String,
    /// `tag` or `branch`.
    pub ref_kind: String,
    pub ref_name: String,
    #[serde(default)]
    pub commit: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
}

pub struct Gh {
    client: Client,
    token: Option<String>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    published_at: Option<String>,
    name: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
    commit: TagCommit,
}

#[derive(Deserialize)]
struct TagCommit {
    sha: String,
}

#[derive(Deserialize)]
struct CommitInfo {
    sha: String,
    commit: CommitDetail,
}

#[derive(Deserialize)]
struct CommitDetail {
    message: String,
    committer: Option<Person>,
    author: Option<Person>,
}

#[derive(Deserialize)]
struct Person {
    date: Option<String>,
}

#[derive(Deserialize)]
struct User {
    login: String,
}

impl CommitInfo {
    fn date(&self) -> Option<String> {
        self.commit
            .committer
            .as_ref()
            .and_then(|p| p.date.clone())
            .or_else(|| self.commit.author.as_ref().and_then(|p| p.date.clone()))
    }
    /// First line of the message, shortened for display.
    fn title(&self) -> String {
        let line = self.commit.message.lines().next().unwrap_or("").trim();
        if line.chars().count() <= 140 {
            return line.to_string();
        }
        let cut: String = line.chars().take(139).collect();
        format!("{}…", cut.trim_end())
    }
}

pub fn http_client() -> Client {
    Client::builder()
        .user_agent(concat!("MyAlfred-Launcher/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("http client")
}

/// The token is only ever sent to GitHub hosts.
pub fn is_github_host(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
        .is_some_and(|h| {
            h == "api.github.com" || h == "raw.githubusercontent.com" || h == "github.com"
        })
}

impl Gh {
    pub fn new(token: Option<String>) -> Self {
        Gh {
            client: http_client(),
            token: token.filter(|t| !t.trim().is_empty()),
        }
    }

    fn get(&self, url: &str, accept: &str) -> reqwest::RequestBuilder {
        let mut req = self
            .client
            .get(url)
            .header(header::ACCEPT, accept)
            .header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(t) = &self.token {
            if is_github_host(url) {
                req = req.bearer_auth(t);
            }
        }
        req
    }

    async fn send(&self, url: &str, accept: &str, what: &str) -> Result<reqwest::Response> {
        let resp = self.get(url, accept).send().await.map_err(|e| {
            if e.is_timeout() || e.is_connect() {
                Error::msg("GitHub est injoignable : vérifiez la connexion Internet.")
            } else {
                Error::Http(e)
            }
        })?;
        match resp.status() {
            s if s.is_success() => Ok(resp),
            StatusCode::NOT_FOUND => {
                if self.token.is_some() {
                    bail!("{what} introuvable, ou le token GitHub n'y a pas accès.")
                } else {
                    bail!(
                        "{what} introuvable ou privé : ajoutez un token GitHub dans les réglages."
                    )
                }
            }
            StatusCode::UNAUTHORIZED => bail!("Token GitHub refusé (expiré ou révoqué)."),
            StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => {
                let remaining = resp
                    .headers()
                    .get("x-ratelimit-remaining")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");
                if remaining == "0" {
                    bail!("Limite de requêtes GitHub atteinte : réessayez plus tard ou ajoutez un token.")
                }
                bail!("Accès refusé par GitHub à {what}.")
            }
            s => bail!("GitHub a répondu {s} pour {what}."),
        }
    }

    async fn json<T: for<'de> Deserialize<'de>>(&self, path: &str, what: &str) -> Result<T> {
        let url = format!("{API}{path}");
        Ok(self
            .send(&url, "application/vnd.github+json", what)
            .await?
            .json::<T>()
            .await?)
    }

    /// Raw content of a file on a branch.
    pub async fn file(&self, owner: &str, repo: &str, path: &str, git_ref: &str) -> Result<String> {
        let url = format!("{API}/repos/{owner}/{repo}/contents/{path}?ref={git_ref}");
        let resp = self
            .send(
                &url,
                "application/vnd.github.raw",
                &format!("Le fichier {path}"),
            )
            .await?;
        Ok(resp.text().await?)
    }

    /// Any URL (catalog); the token is attached only for GitHub hosts.
    pub async fn fetch_text(&self, url: &str) -> Result<String> {
        let accept = if url.starts_with(API) {
            "application/vnd.github.raw"
        } else {
            "application/json"
        };
        Ok(self.send(url, accept, "Le catalogue").await?.text().await?)
    }

    pub async fn whoami(&self) -> Result<String> {
        if self.token.is_none() {
            bail!("Aucun token GitHub enregistré.");
        }
        let u: User = self.json("/user", "Le compte").await?;
        Ok(u.login)
    }

    async fn head_commit(&self, app: &AppDef) -> Result<CommitInfo> {
        let r = &app.repo;
        self.json(
            &format!("/repos/{}/{}/commits/{}", r.owner, r.name, r.branch),
            &format!("Le dépôt {}", r.name),
        )
        .await
    }

    pub async fn latest(&self, app: &AppDef) -> Result<Latest> {
        let r = &app.repo;
        let repo_label = format!("Le dépôt {}", r.name);
        match app.version.source {
            VersionSource::Releases => {
                let rel: Release = self
                    .json(
                        &format!("/repos/{}/{}/releases/latest", r.owner, r.name),
                        &repo_label,
                    )
                    .await?;
                Ok(Latest {
                    version: rel.tag_name.trim_start_matches('v').to_string(),
                    ref_kind: "tag".into(),
                    ref_name: rel.tag_name.clone(),
                    commit: None,
                    date: rel.published_at,
                    summary: rel.name,
                })
            }
            VersionSource::Tags => {
                let tags: Vec<Tag> = self
                    .json(
                        &format!("/repos/{}/{}/tags?per_page=100", r.owner, r.name),
                        &repo_label,
                    )
                    .await?;
                let pattern = app.version.pattern.as_deref().unwrap_or(r"^v?\d");
                let re = regex_lite::Regex::new(pattern)
                    .map_err(|_| Error::msg(format!("Motif de tag invalide pour {}.", app.name)))?;
                let best = tags
                    .into_iter()
                    .filter(|t| re.is_match(&t.name) && version::parse_numeric(&t.name).is_some())
                    .max_by(|a, b| {
                        version::compare(&a.name, &b.name).unwrap_or(std::cmp::Ordering::Equal)
                    });
                let Some(tag) = best else {
                    bail!("Aucune version publiée pour {}.", app.name);
                };
                Ok(Latest {
                    version: tag.name.trim_start_matches('v').to_string(),
                    ref_kind: "tag".into(),
                    ref_name: tag.name,
                    commit: Some(tag.commit.sha),
                    date: None,
                    summary: None,
                })
            }
            VersionSource::File => {
                let path = app.version.path.as_deref().unwrap_or("VERSION");
                let head = self.head_commit(app).await?;
                let content = self.file(&r.owner, &r.name, path, &head.sha).await?;
                let version = match &app.version.json_key {
                    Some(k) => serde_json::from_str::<serde_json::Value>(&content)?
                        .get(k)
                        .and_then(|v| v.as_str())
                        .map(str::to_owned)
                        .ok_or_else(|| Error::msg(format!("Version absente de {path}.")))?,
                    None => content.trim().to_string(),
                };
                Ok(Latest {
                    version,
                    ref_kind: "branch".into(),
                    ref_name: r.branch.clone(),
                    date: head.date(),
                    summary: Some(head.title()),
                    commit: Some(head.sha),
                })
            }
            VersionSource::Commit => {
                let head = self.head_commit(app).await?;
                Ok(Latest {
                    version: head.sha.chars().take(7).collect(),
                    ref_kind: "branch".into(),
                    ref_name: r.branch.clone(),
                    date: head.date(),
                    summary: Some(head.title()),
                    commit: Some(head.sha),
                })
            }
        }
    }

    /// Release notes as Markdown.
    pub async fn changelog(&self, app: &AppDef) -> Result<String> {
        let r = &app.repo;
        let repo_label = format!("Le dépôt {}", r.name);
        let source = app
            .changelog
            .as_ref()
            .map(|c| c.source)
            .unwrap_or(ChangelogSource::Commits);
        match source {
            ChangelogSource::File => {
                let path = app
                    .changelog
                    .as_ref()
                    .and_then(|c| c.path.as_deref())
                    .unwrap_or("CHANGELOG.md");
                let text = self.file(&r.owner, &r.name, path, &r.branch).await?;
                Ok(truncate(&text, 40_000))
            }
            ChangelogSource::Releases => {
                let rels: Vec<Release> = self
                    .json(
                        &format!("/repos/{}/{}/releases?per_page=10", r.owner, r.name),
                        &repo_label,
                    )
                    .await?;
                let mut md = String::new();
                for rel in rels.into_iter().filter(|r| !r.draft && !r.prerelease) {
                    let title = rel
                        .name
                        .filter(|n| !n.is_empty())
                        .unwrap_or(rel.tag_name.clone());
                    md.push_str(&format!("## {title}\n\n"));
                    if let Some(d) = rel.published_at.as_deref() {
                        md.push_str(&format!("*{}*\n\n", &d[..d.len().min(10)]));
                    }
                    md.push_str(rel.body.as_deref().unwrap_or(""));
                    md.push_str("\n\n");
                }
                Ok(md)
            }
            ChangelogSource::Commits => {
                let commits: Vec<CommitInfo> = self
                    .json(
                        &format!(
                            "/repos/{}/{}/commits?sha={}&per_page=20",
                            r.owner, r.name, r.branch
                        ),
                        &repo_label,
                    )
                    .await?;
                let mut md = String::from("## Dernières modifications\n\n");
                for c in commits.iter().filter(|c| !c.title().starts_with("Merge ")) {
                    let date = c.date().unwrap_or_default();
                    md.push_str(&format!(
                        "- {} — *{}* `{}`\n",
                        escape_md(&c.title()),
                        &date[..date.len().min(10)],
                        &c.sha[..7]
                    ));
                }
                Ok(md)
            }
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n\n…", &s[..end])
}

fn escape_md(s: &str) -> String {
    s.replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_only_for_github() {
        assert!(is_github_host("https://api.github.com/repos/x/y"));
        assert!(is_github_host(
            "https://raw.githubusercontent.com/x/y/main/a.json"
        ));
        assert!(!is_github_host("https://evil.example.com/api.github.com"));
        assert!(!is_github_host("https://api.github.com.evil.io/x"));
    }

    #[test]
    fn truncates_on_char_boundary() {
        let s = "é".repeat(10);
        let t = truncate(&s, 5);
        assert!(t.starts_with("éé"));
    }
}

/// Live checks against GitHub (read-only). Run with:
/// `GITHUB_TOKEN=… cargo test -- --ignored live_`
#[cfg(test)]
mod live {
    use super::*;
    use crate::catalog::{Catalog, EMBEDDED};

    #[tokio::test]
    #[ignore]
    async fn live_latest_versions_and_changelogs() {
        let token = std::env::var("GITHUB_TOKEN").ok();
        let gh = Gh::new(token);
        let catalog = Catalog::parse(EMBEDDED).unwrap();
        for app in &catalog.apps {
            let l = gh
                .latest(app)
                .await
                .unwrap_or_else(|e| panic!("{}: {e}", app.id));
            println!(
                "{:8} {:12} {} {:?} {:?}",
                app.id, l.version, l.ref_name, l.date, l.summary
            );
            assert!(!l.version.is_empty());
            let c = gh
                .changelog(app)
                .await
                .unwrap_or_else(|e| panic!("{} changelog: {e}", app.id));
            println!("         changelog: {} caractères", c.len());
            assert!(c.len() > 20);
        }
        let catalog_text = gh.fetch_text(crate::store::DEFAULT_CATALOG_URL).await;
        println!(
            "catalogue distant : {}",
            catalog_text
                .map(|t| format!("{} octets", t.len()))
                .unwrap_or_else(|e| e.to_string())
        );
    }
}
