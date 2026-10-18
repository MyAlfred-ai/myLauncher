//! Runs a bash script on this machine or on a server through the system
//! `ssh` client (keys, agent and ~/.ssh/config are honoured as usual).

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::Notify;

use crate::error::{bail, Error, Result};
use crate::store::Server;

#[derive(Debug, Clone)]
pub enum Target {
    Local,
    Ssh(Server),
}

impl Target {
    /// Git Bash on Windows needs a few compatibility switches.
    pub fn is_windows_local(&self) -> bool {
        matches!(self, Target::Local) && cfg!(windows)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stream {
    Out,
    Err,
}

/// Locates bash: always present on macOS/Linux; Git for Windows on Windows.
pub fn find_bash() -> Option<PathBuf> {
    if !cfg!(windows) {
        return Some(PathBuf::from("bash"));
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    for var in ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)"] {
        if let Ok(base) = std::env::var(var) {
            candidates.push(PathBuf::from(base).join("Git").join("bin").join("bash.exe"));
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(local)
                .join("Programs")
                .join("Git")
                .join("bin")
                .join("bash.exe"),
        );
    }
    candidates.into_iter().find(|p| p.exists())
}

pub fn ssh_args(server: &Server) -> Vec<String> {
    let mut args = vec![
        "-T".to_string(),
        "-o".into(),
        "BatchMode=yes".into(),
        "-o".into(),
        "ConnectTimeout=15".into(),
        "-o".into(),
        "StrictHostKeyChecking=accept-new".into(),
        "-o".into(),
        "ServerAliveInterval=30".into(),
        "-p".into(),
        server.port.to_string(),
    ];
    if let Some(key) = server
        .identity_file
        .as_deref()
        .filter(|k| !k.trim().is_empty())
    {
        args.push("-i".into());
        args.push(key.to_string());
        args.push("-o".into());
        args.push("IdentitiesOnly=yes".into());
    }
    args.push(format!("{}@{}", server.user, server.host));
    args.push("bash -s".into());
    args
}

fn command_for(target: &Target) -> Result<Command> {
    let mut cmd = match target {
        Target::Local => {
            let Some(bash) = find_bash() else {
                bail!("Bash est introuvable. Sous Windows, installez Git for Windows (git-scm.com), puis relancez le launcher.");
            };
            let mut c = Command::new(bash);
            c.arg("-s");
            c
        }
        Target::Ssh(server) => {
            let mut c = Command::new("ssh");
            c.args(ssh_args(server));
            c
        }
    };
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if let Target::Local = target {
        // Avoid any interactive prompt from git or docker.
        cmd.env("GIT_TERMINAL_PROMPT", "0")
            .env("DOCKER_CLI_HINTS", "false");
    }
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    Ok(cmd)
}

/// Removes ANSI colour codes and keeps the last frame of `\r` progress lines.
pub fn clean_line(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    let text = text.trim_end_matches(['\n', '\r']);
    let text = text.rsplit('\r').next().unwrap_or("");
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// Streams the script output line by line. Returns the exit code
/// (`None` when cancelled).
pub async fn run_streaming<F>(
    target: &Target,
    script: &str,
    cancel: Arc<Notify>,
    mut on_line: F,
) -> Result<Option<i32>>
where
    F: FnMut(Stream, String) + Send,
{
    let mut child = command_for(target)?.spawn().map_err(|e| {
        Error::msg(match target {
            Target::Local => format!("Impossible de lancer bash : {e}"),
            Target::Ssh(_) => {
                format!("Impossible de lancer ssh : {e}. Le client OpenSSH est-il installé ?")
            }
        })
    })?;

    let mut stdin = child.stdin.take().expect("stdin piped");
    let script = script.to_owned();
    let writer = tokio::spawn(async move {
        let _ = stdin.write_all(script.as_bytes()).await;
        let _ = stdin.shutdown().await;
    });

    let mut out = BufReader::new(child.stdout.take().expect("stdout piped"));
    let mut err = BufReader::new(child.stderr.take().expect("stderr piped"));
    let (mut out_buf, mut err_buf) = (Vec::new(), Vec::new());
    let (mut out_open, mut err_open) = (true, true);

    while out_open || err_open {
        tokio::select! {
            r = out.read_until(b'\n', &mut out_buf), if out_open => {
                match r {
                    Ok(0) | Err(_) => out_open = false,
                    Ok(_) => { on_line(Stream::Out, clean_line(&out_buf)); out_buf.clear(); }
                }
            }
            r = err.read_until(b'\n', &mut err_buf), if err_open => {
                match r {
                    Ok(0) | Err(_) => err_open = false,
                    Ok(_) => { on_line(Stream::Err, clean_line(&err_buf)); err_buf.clear(); }
                }
            }
            _ = cancel.notified() => {
                let _ = child.kill().await;
                writer.abort();
                return Ok(None);
            }
        }
    }
    let _ = writer.await;
    let status = child.wait().await?;
    Ok(Some(status.code().unwrap_or(-1)))
}

pub struct Captured {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a short read-only script and collects its output.
pub async fn run_capture(target: &Target, script: &str, timeout: Duration) -> Result<Captured> {
    let cancel = Arc::new(Notify::new());
    let mut stdout = String::new();
    let mut stderr = String::new();
    let fut = run_streaming(target, script, cancel.clone(), |stream, line| {
        let buf = if stream == Stream::Out {
            &mut stdout
        } else {
            &mut stderr
        };
        buf.push_str(&line);
        buf.push('\n');
    });
    match tokio::time::timeout(timeout, fut).await {
        Ok(Ok(Some(code))) => Ok(Captured {
            code,
            stdout,
            stderr,
        }),
        Ok(Ok(None)) => bail!("Opération annulée."),
        Ok(Err(e)) => Err(e),
        Err(_) => {
            cancel.notify_one();
            bail!("La machine ne répond pas (délai dépassé).")
        }
    }
}

/// Extracts `::ml::result::k=v` pairs from captured output.
pub fn results(stdout: &str) -> std::collections::BTreeMap<String, String> {
    stdout
        .lines()
        .filter_map(|l| l.strip_prefix("::ml::result::"))
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.trim().to_string()))
        .collect()
}

/// Turns an ssh failure into a readable message.
pub fn ssh_error(stderr: &str) -> String {
    let last = stderr
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();
    if last.contains("Permission denied") {
        "Connexion refusée : la clé SSH n'est pas acceptée par le serveur.".into()
    } else if last.contains("Could not resolve hostname") {
        "Adresse du serveur introuvable.".into()
    } else if last.contains("Connection refused") {
        "Le serveur refuse la connexion SSH (port fermé ?).".into()
    } else if last.contains("timed out") {
        "Le serveur ne répond pas (délai dépassé).".into()
    } else if last.contains("Host key verification failed")
        || last.contains("REMOTE HOST IDENTIFICATION")
    {
        "L'empreinte du serveur a changé depuis la dernière connexion : vérifiez ~/.ssh/known_hosts.".into()
    } else if last.is_empty() {
        "Connexion SSH impossible.".into()
    } else {
        format!("Connexion SSH impossible : {last}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_terminal_noise() {
        assert_eq!(clean_line(b"\x1b[32mok\x1b[0m done\n"), "ok done");
        assert_eq!(clean_line(b"10%\r50%\r100%\n"), "100%");
        assert_eq!(clean_line(b"plain\r\n"), "plain");
    }

    #[test]
    fn parses_results() {
        let r = results("x\n::ml::result::version=1.2.3\n::ml::result::commit=abc\n");
        assert_eq!(r.get("version").unwrap(), "1.2.3");
        assert_eq!(r.get("commit").unwrap(), "abc");
    }

    #[tokio::test]
    async fn runs_a_local_script() {
        if cfg!(windows) {
            return;
        }
        let cap = run_capture(
            &Target::Local,
            "{ echo out; echo err >&2; read -r x || echo no-stdin; exit 3; } < /dev/null\n",
            Duration::from_secs(10),
        )
        .await
        .unwrap();
        assert_eq!(cap.code, 3);
        assert!(cap.stdout.contains("out"));
        assert!(cap.stdout.contains("no-stdin"));
        assert!(cap.stderr.contains("err"));
    }
}
