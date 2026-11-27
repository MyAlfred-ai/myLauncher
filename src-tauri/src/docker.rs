//! Docker on this machine: finds the desktop app that provides it, starts it
//! when it is stopped, and installs Docker Desktop for people who have none.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::error::{bail, Error, Result};

/// Docker Inc's Apple developer team: the downloaded app must carry it.
const DOCKER_TEAM_ID: &str = "9BNSXJN65R";

static INSTALLING: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

fn quiet(cmd: &mut Command) -> &mut Command {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Desktop apps that run a Docker engine, in order of preference.
pub fn desktop_apps() -> Vec<(&'static str, PathBuf)> {
    let mut found = Vec::new();
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        for (name, bundle) in [
            ("Docker Desktop", "Docker.app"),
            ("OrbStack", "OrbStack.app"),
            ("Rancher Desktop", "Rancher Desktop.app"),
            ("Podman Desktop", "Podman Desktop.app"),
        ] {
            for base in ["/Applications".to_string(), format!("{home}/Applications")] {
                let p = Path::new(&base).join(bundle);
                if p.exists() {
                    found.push((name, p));
                    break;
                }
            }
        }
    } else if cfg!(windows) {
        let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".into());
        let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
        for (name, p) in [
            (
                "Docker Desktop",
                Path::new(&pf).join(r"Docker\Docker\Docker Desktop.exe"),
            ),
            (
                "Rancher Desktop",
                Path::new(&pf).join(r"Rancher Desktop\Rancher Desktop.exe"),
            ),
            (
                "Podman Desktop",
                Path::new(&local).join(r"Programs\podman-desktop\Podman Desktop.exe"),
            ),
        ] {
            if p.exists() {
                found.push((name, p));
            }
        }
    }
    found
}

/// True when `docker info` answers, i.e. the engine is running.
pub async fn daemon_ok() -> bool {
    let mut cmd = Command::new("docker");
    quiet(&mut cmd).arg("info");
    matches!(
        tokio::time::timeout(Duration::from_secs(15), cmd.status()).await,
        Ok(Ok(s)) if s.success()
    )
}

fn launch(app: &Path) -> Result<()> {
    if cfg!(target_os = "macos") {
        // -g: in the background, the launcher keeps the focus.
        std::process::Command::new("open")
            .arg("-g")
            .arg("-a")
            .arg(app)
            .status()?;
    } else {
        std::process::Command::new(app)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
    }
    Ok(())
}

async fn wait_ready(timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if daemon_ok().await {
            return true;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    false
}

/// Starts the Docker desktop app when the engine does not answer.
/// Returns whether Docker is running at the end.
pub async fn ensure_running(mut log: impl FnMut(String)) -> bool {
    if daemon_ok().await {
        return true;
    }
    let Some((name, app)) = desktop_apps().into_iter().next() else {
        return false;
    };
    log(format!(
        "Docker ne répond pas : démarrage de {name} (jusqu'à 3 minutes)…"
    ));
    if let Err(e) = launch(&app) {
        log(format!("Impossible de démarrer {name} : {e}"));
        return false;
    }
    let ok = wait_ready(Duration::from_secs(180)).await;
    log(if ok {
        format!("{name} est prêt.")
    } else {
        format!("{name} ne répond toujours pas.")
    });
    ok
}

/// Official Docker Desktop installer for this machine, when one exists.
pub fn installer() -> Option<(&'static str, &'static str)> {
    if cfg!(target_os = "macos") {
        // A launcher built for Intel may run on Apple Silicon (Rosetta):
        // ask the hardware, not the binary.
        let arm = std::process::Command::new("sysctl")
            .args(["-n", "hw.optional.arm64"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
            .unwrap_or(cfg!(target_arch = "aarch64"));
        Some(if arm {
            (
                "https://desktop.docker.com/mac/main/arm64/Docker.dmg",
                "Docker.dmg",
            )
        } else {
            (
                "https://desktop.docker.com/mac/main/amd64/Docker.dmg",
                "Docker.dmg",
            )
        })
    } else if cfg!(windows) {
        Some(if cfg!(target_arch = "aarch64") {
            (
                "https://desktop.docker.com/win/main/arm64/Docker%20Desktop%20Installer.exe",
                "Docker Desktop Installer.exe",
            )
        } else {
            (
                "https://desktop.docker.com/win/main/amd64/Docker%20Desktop%20Installer.exe",
                "Docker Desktop Installer.exe",
            )
        })
    } else {
        None
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupProgress {
    /// `download`, `verify`, `install`, `start` or `done`.
    pub phase: String,
    pub received: u64,
    pub total: u64,
}

fn emit(app: &AppHandle, phase: &str, received: u64, total: u64) {
    let _ = app.emit(
        "docker-setup",
        SetupProgress {
            phase: phase.into(),
            received,
            total,
        },
    );
}

async fn download(app: &AppHandle, url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent(concat!("MyAlfred-Launcher/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(20))
        .build()?;
    let mut resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        bail!("Téléchargement de Docker refusé ({}).", resp.status());
    }
    let total = resp.content_length().unwrap_or(0);
    let part = dest.with_extension("part");
    let mut file = tokio::fs::File::create(&part).await?;
    let mut received = 0u64;
    let mut last = Instant::now();
    loop {
        if CANCEL.load(Ordering::Relaxed) {
            drop(file);
            let _ = tokio::fs::remove_file(&part).await;
            bail!("Installation de Docker annulée.");
        }
        let chunk = tokio::time::timeout(Duration::from_secs(60), resp.chunk())
            .await
            .map_err(|_| {
                Error::msg(
                    "Le téléchargement de Docker est bloqué : vérifiez la connexion Internet.",
                )
            })??;
        let Some(chunk) = chunk else { break };
        file.write_all(&chunk).await?;
        received += chunk.len() as u64;
        if last.elapsed() > Duration::from_millis(300) {
            emit(app, "download", received, total);
            last = Instant::now();
        }
    }
    file.flush().await?;
    drop(file);
    if total > 0 && received != total {
        bail!("Téléchargement de Docker incomplet : réessayez.");
    }
    tokio::fs::rename(&part, dest).await?;
    emit(app, "download", received, received);
    Ok(())
}

/// Escapes a shell command for an AppleScript string literal.
fn applescript_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// The privileged part, run once the user has typed the administrator
/// password in the system's own dialog. The signature is checked again as
/// root, right before running Docker's installer.
fn mac_install_command(dmg: &Path, user: &str) -> String {
    let q = crate::script::sh_quote;
    format!(
        "set -e; M=$(mktemp -d /tmp/ml-docker.XXXXXX); \
         hdiutil attach -nobrowse -readonly -noautoopen -quiet -mountpoint \"$M\" {dmg}; \
         trap 'hdiutil detach -quiet \"$M\" || true' EXIT; \
         A=\"$M/Docker.app\"; \
         codesign --verify --deep --strict \"$A\"; \
         codesign -dv \"$A\" 2>&1 | grep -q 'TeamIdentifier={DOCKER_TEAM_ID}'; \
         \"$A/Contents/MacOS/install\" --accept-license --user={user}",
        dmg = q(&dmg.to_string_lossy()),
        user = q(user),
    )
}

async fn install_mac(dmg: &Path) -> Result<()> {
    let user = std::env::var("USER").unwrap_or_default();
    if user.is_empty()
        || !user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
    {
        bail!("Nom d'utilisateur macOS inattendu : installation de Docker impossible.");
    }
    let script = format!(
        "do shell script {} with administrator privileges with prompt \"MyAlfred Launcher installe Docker Desktop.\"",
        applescript_string(&mac_install_command(dmg, &user))
    );
    let mut cmd = Command::new("osascript");
    quiet(&mut cmd).arg("-e").arg(script);
    let out = cmd.output().await?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    if err.contains("-128") {
        bail!("Installation de Docker annulée : le mot de passe administrateur n'a pas été saisi.");
    }
    bail!("L'installation de Docker a échoué : {}", err.trim());
}

async fn install_windows(exe: &Path) -> Result<()> {
    let script = format!(
        "$f = {f}; $s = Get-AuthenticodeSignature -FilePath $f; \
         if ($s.Status -ne 'Valid' -or $s.SignerCertificate.Subject -notmatch 'Docker Inc') {{ exit 90 }}; \
         try {{ $p = Start-Process -FilePath $f -ArgumentList 'install','--quiet','--accept-license' -Verb RunAs -Wait -PassThru }} catch {{ exit 91 }}; \
         exit $p.ExitCode",
        f = ps_quote(&exe.to_string_lossy())
    );
    let mut cmd = Command::new("powershell");
    quiet(&mut cmd).args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &script,
    ]);
    let out = cmd.output().await?;
    match out.status.code() {
        Some(0) => Ok(()),
        Some(90) => {
            bail!("L'installeur téléchargé n'est pas signé par Docker : installation arrêtée.")
        }
        Some(91) => {
            bail!("Installation de Docker annulée : l'autorisation administrateur a été refusée.")
        }
        Some(3010) => {
            bail!("Docker est installé : redémarrez l'ordinateur, puis relancez le launcher.")
        }
        code => bail!(
            "L'installation de Docker a échoué (code {}). {}",
            code.unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr).trim()
        ),
    }
}

/// Downloads, checks and installs Docker Desktop, then starts it.
pub async fn install(app: &AppHandle) -> Result<()> {
    if INSTALLING.swap(true, Ordering::SeqCst) {
        bail!("L'installation de Docker est déjà en cours.");
    }
    CANCEL.store(false, Ordering::SeqCst);
    let result = install_inner(app).await;
    INSTALLING.store(false, Ordering::SeqCst);
    result
}

async fn install_inner(app: &AppHandle) -> Result<()> {
    let Some((url, file_name)) = installer() else {
        bail!("Installation automatique indisponible sur ce système : suivez docs.docker.com/engine/install.");
    };
    let dir = app.path().app_cache_dir()?.join("docker");
    tokio::fs::create_dir_all(&dir).await?;
    let file = dir.join(file_name);
    emit(app, "download", 0, 0);
    download(app, url, &file).await?;

    emit(app, "install", 0, 0);
    let installed = if cfg!(target_os = "macos") {
        install_mac(&file).await
    } else {
        install_windows(&file).await
    };
    let _ = tokio::fs::remove_file(&file).await;
    installed?;

    emit(app, "start", 0, 0);
    let Some((name, path)) = desktop_apps().into_iter().next() else {
        bail!("Docker est installé mais introuvable : ouvrez-le depuis vos applications.");
    };
    launch(&path)?;
    // The first start prepares Docker's virtual machine: it takes longer.
    if !wait_ready(Duration::from_secs(300)).await {
        bail!("{name} est installé mais ne répond pas encore : terminez sa première ouverture, puis vérifiez à nouveau.");
    }
    emit(app, "done", 0, 0);
    Ok(())
}

pub fn cancel_install() {
    CANCEL.store(true, Ordering::SeqCst);
}

/// Started from the interface: same as before an operation, with an error
/// message when Docker cannot be started.
pub async fn start() -> Result<()> {
    if daemon_ok().await {
        return Ok(());
    }
    let apps = desktop_apps();
    let Some((name, _)) = apps.first() else {
        bail!("Docker n'est pas installé sur ce poste.");
    };
    if !ensure_running(|_| {}).await {
        bail!("{name} n'a pas démarré. Ouvrez-le depuis vos applications et terminez sa configuration, puis vérifiez à nouveau.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_for_applescript_and_powershell() {
        assert_eq!(applescript_string(r#"a "b" \c"#), r#""a \"b\" \\c""#);
        assert_eq!(
            ps_quote("C:\\Users\\O'Neil\\x.exe"),
            "'C:\\Users\\O''Neil\\x.exe'"
        );
    }

    #[test]
    fn mac_command_checks_signature_before_installing() {
        let cmd = mac_install_command(Path::new("/Users/a b/Library/Caches/x/Docker.dmg"), "demo");
        let verify = cmd.find("codesign --verify").unwrap();
        let team = cmd.find("TeamIdentifier=9BNSXJN65R").unwrap();
        let install = cmd.find("--accept-license").unwrap();
        assert!(verify < team && team < install);
        assert!(cmd.contains("'/Users/a b/Library/Caches/x/Docker.dmg'"));
        assert!(cmd.contains("--user='demo'"));
    }

    #[test]
    fn mac_command_is_valid_sh() {
        if !cfg!(unix) {
            return;
        }
        let cmd = mac_install_command(Path::new("/tmp/Docker.dmg"), "demo");
        let out = std::process::Command::new("/bin/sh")
            .args(["-n", "-c", &cmd])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// The Docker Desktop already on a Mac passes the same check as a download.
    #[test]
    fn installed_docker_desktop_passes_the_signature_check() {
        let app = Path::new("/Applications/Docker.app");
        if !cfg!(target_os = "macos") || !app.exists() {
            return;
        }
        let out = std::process::Command::new("codesign")
            .arg("-dv")
            .arg(app)
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&out.stderr)
            .contains(&format!("TeamIdentifier={DOCKER_TEAM_ID}")));
    }
}
