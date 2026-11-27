//! Fetches an application's code with the Git library built into the
//! launcher: the machine needs no `git` program. The result is an ordinary
//! repository, so installs made earlier with the system git keep working.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use git2::build::CheckoutBuilder;
use git2::{
    AutotagOption, Cred, ErrorClass, ErrorCode, FetchOptions, ProxyOptions, RemoteCallbacks,
    Repository,
};

use crate::error::{bail, Error, Result};

/// Where the code comes from and which version to take.
#[derive(Debug, Clone)]
pub struct Source {
    pub url: String,
    /// `tag` or `branch`.
    pub ref_kind: String,
    pub ref_name: String,
    /// GitHub token for private repositories; only handed to libgit2 when
    /// the server asks for credentials, never written to `.git/config`.
    pub token: Option<String>,
}

/// Commit currently checked out, if the folder holds a repository.
pub fn head_commit(dir: &Path) -> Option<String> {
    let repo = Repository::open(dir).ok()?;
    let commit = repo.head().ok()?.peel_to_commit().ok()?;
    Some(commit.id().to_string())
}

fn readable(e: git2::Error, has_token: bool) -> Error {
    if e.code() == ErrorCode::Auth || e.message().contains("401") {
        if has_token {
            Error::msg("Téléchargement refusé : le token GitHub a expiré, a été révoqué ou n'a pas accès à ce dépôt. Remplacez-le dans les réglages du launcher.")
        } else {
            Error::msg("Téléchargement refusé. Dépôt privé ? Ajoutez un token GitHub dans les réglages du launcher.")
        }
    } else if e.code() == ErrorCode::User {
        Error::msg("Opération annulée.")
    } else if matches!(
        e.class(),
        ErrorClass::Net | ErrorClass::Http | ErrorClass::Ssl
    ) {
        Error::msg(format!(
            "Téléchargement du code impossible : vérifiez la connexion Internet ({}).",
            e.message()
        ))
    } else {
        Error::Git(e)
    }
}

/// Brings `dir` to the requested version. Tracked files are overwritten and
/// files removed upstream disappear; untracked files (`.env`, data,
/// backups) are left alone. Returns the commit now checked out.
pub fn sync(
    dir: &Path,
    src: &Source,
    cancel: &AtomicBool,
    mut progress: impl FnMut(String),
) -> Result<String> {
    let repo = if dir.join(".git").exists() {
        Repository::open(dir)?
    } else {
        if dir.exists() && std::fs::read_dir(dir)?.next().is_some() {
            bail!(
                "Le dossier {} existe déjà et ne contient pas l'application. Choisissez un autre dossier ou videz-le.",
                dir.display()
            );
        }
        std::fs::create_dir_all(dir)?;
        Repository::init(dir)?
    };
    // Scripts must keep their LF line endings, even on Windows.
    repo.config()?.set_bool("core.autocrlf", false)?;

    let mut remote = match repo.find_remote("origin") {
        Ok(r) if r.url() == Some(src.url.as_str()) => r,
        Ok(_) => {
            repo.remote_set_url("origin", &src.url)?;
            repo.find_remote("origin")?
        }
        Err(_) => repo.remote("origin", &src.url)?,
    };

    let is_tag = src.ref_kind == "tag";
    let refspec = if is_tag {
        format!("+refs/tags/{0}:refs/tags/{0}", src.ref_name)
    } else {
        format!("+refs/heads/{0}:refs/remotes/origin/{0}", src.ref_name)
    };

    progress(format!("Téléchargement du code depuis {}…", src.url));
    {
        let mut asked = false;
        let mut last = 0usize;
        let mut cb = RemoteCallbacks::new();
        cb.credentials(|_url, _user, _allowed| match src.token.as_deref() {
            // Asked twice means the token was refused: stop instead of looping.
            Some(t) if !asked => {
                asked = true;
                Cred::userpass_plaintext("x-access-token", t)
            }
            _ => Err(git2::Error::new(
                ErrorCode::Auth,
                ErrorClass::Http,
                "401 accès refusé",
            )),
        });
        cb.transfer_progress(|p| {
            if cancel.load(Ordering::Relaxed) {
                return false;
            }
            let total = p.total_objects().max(1);
            let pct = p.received_objects() * 100 / total;
            if pct >= last + 20 && pct < 100 {
                last = pct - pct % 20;
                progress(format!("Téléchargement du code : {last} %"));
            }
            true
        });
        let mut proxy = ProxyOptions::new();
        proxy.auto();
        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);
        fo.proxy_options(proxy);
        fo.download_tags(AutotagOption::None);
        remote
            .fetch(&[&refspec], Some(&mut fo), None)
            .map_err(|e| readable(e, src.token.is_some()))?;
    }
    if cancel.load(Ordering::Relaxed) {
        bail!("Opération annulée.");
    }

    let target = if is_tag {
        format!("refs/tags/{}", src.ref_name)
    } else {
        format!("refs/remotes/origin/{}", src.ref_name)
    };
    let commit = repo.revparse_single(&target)?.peel_to_commit()?;
    let mut co = CheckoutBuilder::new();
    co.force();
    repo.checkout_tree(commit.as_object(), Some(&mut co))?;
    if is_tag {
        repo.set_head_detached(commit.id())?;
    } else {
        let branch = format!("refs/heads/{}", src.ref_name);
        repo.reference(&branch, commit.id(), true, "mylauncher: mise à jour")?;
        repo.set_head(&branch)?;
    }
    let id = commit.id().to_string();
    progress(format!("Code à jour : {}", &id[..7]));
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Signature;
    use std::path::PathBuf;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ml-git-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Commits the given files (None = delete) on `main` of a source repo.
    fn commit(repo: &Repository, files: &[(&str, Option<&str>)], msg: &str) -> git2::Oid {
        let root = repo.workdir().unwrap().to_path_buf();
        let mut index = repo.index().unwrap();
        for (path, content) in files {
            let p = root.join(path);
            match content {
                Some(c) => {
                    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                    std::fs::write(&p, c).unwrap();
                    index.add_path(Path::new(path)).unwrap();
                }
                None => {
                    std::fs::remove_file(&p).unwrap();
                    index.remove_path(Path::new(path)).unwrap();
                }
            }
        }
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        let parents: Vec<git2::Commit> = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .into_iter()
            .collect();
        let parents: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("refs/heads/main"), &sig, &sig, msg, &tree, &parents)
            .unwrap()
    }

    fn source(path: &Path, kind: &str, name: &str) -> Source {
        Source {
            url: format!("file://{}", path.display()),
            ref_kind: kind.into(),
            ref_name: name.into(),
            token: None,
        }
    }

    #[test]
    fn installs_then_updates_keeping_local_files() {
        let upstream_dir = tmp("up");
        let upstream = Repository::init(&upstream_dir).unwrap();
        upstream.set_head("refs/heads/main").unwrap();
        let c1 = commit(
            &upstream,
            &[
                ("VERSION", Some("1.0\n")),
                ("old.sql", Some("x")),
                ("install.sh", Some("echo hi\n")),
            ],
            "v1",
        );

        let dest = tmp("dest").join("mia");
        let stop = AtomicBool::new(false);
        let got = sync(
            &dest,
            &source(&upstream_dir, "branch", "main"),
            &stop,
            |_| {},
        )
        .unwrap();
        assert_eq!(got, c1.to_string());
        assert_eq!(
            std::fs::read_to_string(dest.join("VERSION")).unwrap(),
            "1.0\n"
        );
        assert_eq!(head_commit(&dest).unwrap(), c1.to_string());

        // Local configuration and an edited tracked file.
        std::fs::write(dest.join(".env"), "SECRET=1\n").unwrap();
        std::fs::write(dest.join("install.sh"), "edited\n").unwrap();

        let c2 = commit(
            &upstream,
            &[("VERSION", Some("1.1\n")), ("old.sql", None)],
            "v2",
        );
        let mut messages = Vec::new();
        let got = sync(
            &dest,
            &source(&upstream_dir, "branch", "main"),
            &stop,
            |m| messages.push(m),
        )
        .unwrap();
        assert_eq!(got, c2.to_string());
        assert_eq!(
            std::fs::read_to_string(dest.join("VERSION")).unwrap(),
            "1.1\n"
        );
        assert!(!dest.join("old.sql").exists(), "fichier supprimé en amont");
        assert_eq!(
            std::fs::read_to_string(dest.join(".env")).unwrap(),
            "SECRET=1\n"
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("install.sh")).unwrap(),
            "echo hi\n"
        );
        assert!(messages.last().unwrap().starts_with("Code à jour"));
        let cfg = std::fs::read_to_string(dest.join(".git/config")).unwrap();
        assert!(cfg.contains("autocrlf = false"));
    }

    #[test]
    fn checks_out_a_tag() {
        let upstream_dir = tmp("tag");
        let upstream = Repository::init(&upstream_dir).unwrap();
        upstream.set_head("refs/heads/main").unwrap();
        let c1 = commit(&upstream, &[("a.txt", Some("1"))], "v1");
        upstream
            .tag_lightweight("v1.0.0", &upstream.find_object(c1, None).unwrap(), false)
            .unwrap();
        commit(&upstream, &[("a.txt", Some("2"))], "v2");

        let dest = tmp("tagdest").join("app");
        let got = sync(
            &dest,
            &source(&upstream_dir, "tag", "v1.0.0"),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap();
        assert_eq!(got, c1.to_string());
        assert_eq!(std::fs::read_to_string(dest.join("a.txt")).unwrap(), "1");
    }

    #[test]
    fn refused_download_with_token_blames_the_token() {
        let e = git2::Error::new(ErrorCode::Auth, ErrorClass::Http, "401 accès refusé");
        let msg = readable(e, true).to_string();
        assert!(msg.contains("expiré") && msg.contains("réglages"), "{msg}");
    }

    #[test]
    fn refuses_a_foreign_folder() {
        let dest = tmp("foreign");
        std::fs::write(dest.join("notes.txt"), "à moi").unwrap();
        let err = sync(
            &dest,
            &source(&dest, "branch", "main"),
            &AtomicBool::new(false),
            |_| {},
        )
        .unwrap_err();
        assert!(err.to_string().contains("ne contient pas l'application"));
        assert!(dest.join("notes.txt").exists());
    }
}

/// Live check against GitHub. Run with:
/// `GITHUB_TOKEN=… ML_TEST_DIR=… cargo test -- --ignored live_`
#[cfg(test)]
mod live {
    use super::*;

    #[test]
    #[ignore]
    fn live_sync_private_repo() {
        // A private repository the token can read, as "owner/name": never
        // written here, this code is public.
        let Ok(repo) = std::env::var("ML_TEST_PRIVATE_REPO") else {
            println!("ML_TEST_PRIVATE_REPO absent : test du dépôt privé ignoré");
            return;
        };
        let token = std::env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN");
        let base = std::env::var("ML_TEST_DIR").expect("ML_TEST_DIR");
        let dir = std::path::Path::new(&base).join("private");
        let src = Source {
            url: format!("https://github.com/{repo}.git"),
            ref_kind: "branch".into(),
            ref_name: "main".into(),
            token: Some(token.clone()),
        };
        let stop = AtomicBool::new(false);
        let first = sync(&dir, &src, &stop, |m| println!("{m}")).unwrap();
        std::fs::write(dir.join(".env"), "LOCAL=1\n").unwrap();
        // Again: the clone is updated in place and local files stay.
        let second = sync(&dir, &src, &stop, |m| println!("{m}")).unwrap();
        assert_eq!(first, second);
        assert!(dir.join(".env").exists());
        let cfg = std::fs::read_to_string(dir.join(".git/config")).unwrap();
        assert!(!cfg.contains(&token), "le token ne doit pas être écrit");
        // Without a token, a private repository is refused with a clear message.
        let anon = Source { token: None, ..src };
        let err = sync(
            &std::path::Path::new(&base).join("anon"),
            &anon,
            &stop,
            |_| {},
        )
        .unwrap_err();
        println!("sans token : {err}");
        assert!(err.to_string().contains("token GitHub"));
    }

    /// MIA and myBrief are public: cloned without any token.
    #[test]
    #[ignore]
    fn live_sync_public_repo_without_token() {
        let base = std::env::var("ML_TEST_DIR").expect("ML_TEST_DIR");
        for (name, kind, git_ref, file) in [
            ("mia", "branch", "main", "install.sh"),
            ("my-brief", "tag", "v1.3.0", "kit/update.sh"),
        ] {
            let dir = std::path::Path::new(&base).join(name);
            let src = Source {
                url: format!("https://github.com/MyAlfred-ai/{name}.git"),
                ref_kind: kind.into(),
                ref_name: git_ref.into(),
                token: None,
            };
            let commit = sync(&dir, &src, &AtomicBool::new(false), |m| println!("{m}")).unwrap();
            assert_eq!(head_commit(&dir).unwrap(), commit);
            assert!(dir.join(file).exists(), "{name}: {file}");
        }
    }
}
