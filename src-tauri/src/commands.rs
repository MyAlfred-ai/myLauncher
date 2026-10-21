//! Commands called by the interface.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use crate::catalog::{AppDef, Catalog, FieldKind, TargetDef, VersionSource};
use crate::error::{bail, Error, Result};
use crate::github::Gh;
use crate::gitsync::Source;
use crate::ops::{self, LogLine, OpInfo, OpRequest};
use crate::script::{self, Action, Plan};
use crate::state::{now, state, LatestState, LauncherUpdateState};
use crate::store::{check_dir, InstallState, Installation, Settings};
use crate::{docker, exec};
use crate::{secrets, updates, version};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationView {
    #[serde(flatten)]
    pub inst: Installation,
    pub update_available: bool,
    pub busy_op: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub catalog: Catalog,
    pub catalog_source: String,
    pub catalog_fetched_at: Option<String>,
    pub latest: HashMap<String, LatestState>,
    pub installations: Vec<InstallationView>,
    pub ops: Vec<OpInfo>,
    pub settings: Settings,
    pub has_token: bool,
    pub last_check: Option<String>,
    pub launcher_update: LauncherUpdateState,
    pub platform: String,
    pub launcher_version: String,
}

#[tauri::command]
pub fn get_overview(app: AppHandle) -> Overview {
    let st = state(&app);
    let latest = st.latest.read().unwrap().clone();
    let store = st.store.read().unwrap().clone();
    let catalog = st.catalog.read().unwrap();
    let installations = store
        .installations
        .iter()
        .map(|i| {
            let by_commit = catalog
                .catalog
                .app(&i.app_id)
                .is_some_and(|a| a.version.source == VersionSource::Commit);
            let update_available = i.state == InstallState::Ready
                && latest
                    .get(&i.app_id)
                    .and_then(|l| l.latest.as_ref())
                    .is_some_and(|l| {
                        version::is_update(
                            &l.version,
                            l.commit.as_deref(),
                            i.version.as_deref(),
                            i.commit.as_deref(),
                            by_commit,
                        )
                    });
            InstallationView {
                inst: i.clone(),
                update_available,
                busy_op: st.ops.running_for(&i.id),
            }
        })
        .collect();
    let settings = st.settings.read().unwrap().clone();
    let last_check = st.last_check.read().unwrap().clone();
    let launcher_update = st.launcher_update.read().unwrap().clone();
    Overview {
        catalog: catalog.catalog.clone(),
        catalog_source: catalog.source.clone(),
        catalog_fetched_at: catalog.fetched_at.clone(),
        latest,
        installations,
        ops: st.ops.list(),
        settings,
        has_token: st.token().is_some(),
        last_check,
        launcher_update,
        platform: std::env::consts::OS.into(),
        // The version the updater compares with (tauri.conf.json).
        launcher_version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
pub async fn check_updates(app: AppHandle) -> Result<Overview> {
    updates::check_all(&app).await;
    Ok(get_overview(app))
}

#[tauri::command]
pub async fn check_launcher_update(app: AppHandle) -> LauncherUpdateState {
    updates::check_launcher(&app).await;
    let lu = state(&app).launcher_update.read().unwrap().clone();
    lu
}

#[tauri::command]
pub async fn install_launcher_update(app: AppHandle) -> Result<()> {
    updates::install_launcher(&app).await
}

#[tauri::command]
pub async fn get_changelog(app: AppHandle, app_id: String) -> Result<String> {
    let (def, token) = {
        let st = state(&app);
        let def = find_app(&st.catalog.read().unwrap().catalog, &app_id)?;
        (def, st.token())
    };
    Gh::new(token).changelog(&def).await
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<Settings> {
    if !matches!(settings.theme.as_str(), "system" | "light" | "dark") {
        bail!("Thème inconnu.");
    }
    check_dir(&settings.install_dir)?;
    if !settings.catalog_url.is_empty() && !settings.catalog_url.starts_with("https://") {
        bail!("L'adresse du catalogue doit commencer par https://");
    }
    let st = state(&app);
    *st.settings.write().unwrap() = Settings {
        check_interval_hours: settings.check_interval_hours.clamp(1, 168),
        ..settings
    };
    st.save_settings()?;
    let _ = app.emit("overview-changed", ());
    let saved = st.settings.read().unwrap().clone();
    Ok(saved)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenStatus {
    pub present: bool,
    pub login: Option<String>,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn github_status(app: AppHandle) -> TokenStatus {
    let token = state(&app).token();
    let Some(token) = token else {
        return TokenStatus {
            present: false,
            login: None,
            error: None,
        };
    };
    match Gh::new(Some(token)).whoami().await {
        Ok(login) => TokenStatus {
            present: true,
            login: Some(login),
            error: None,
        },
        Err(e) => TokenStatus {
            present: true,
            login: None,
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
pub async fn set_github_token(app: AppHandle, token: String) -> Result<TokenStatus> {
    let token = token.trim().to_string();
    if token.is_empty() || token.contains(char::is_whitespace) {
        bail!("Token vide ou mal copié.");
    }
    let login = Gh::new(Some(token.clone())).whoami().await?;
    secrets::set_token(&token)?;
    *state(&app).token.write().unwrap() = Some(token);
    let _ = app.emit("overview-changed", ());
    Ok(TokenStatus {
        present: true,
        login: Some(login),
        error: None,
    })
}

#[tauri::command]
pub fn clear_github_token(app: AppHandle) -> Result<()> {
    secrets::clear_token()?;
    *state(&app).token.write().unwrap() = None;
    let _ = app.emit("overview-changed", ());
    Ok(())
}

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HostCheck {
    pub reachable: bool,
    pub os: String,
    pub git: String,
    pub docker: String,
    pub compose: String,
    pub daemon: bool,
    /// Desktop app providing Docker (Docker Desktop, OrbStack…), even when
    /// its command-line tools are not found yet.
    pub docker_app: Option<String>,
    /// Docker Desktop can be installed by the launcher on this system.
    pub can_install_docker: bool,
    /// Memory of Docker's virtual machine, in bytes (0 when unknown).
    pub mem_total: u64,
    /// Free space where applications are installed, in bytes (0 when unknown).
    pub disk_free: u64,
    pub engine: String,
    pub error: Option<String>,
}

#[tauri::command]
pub async fn check_host(app: AppHandle) -> Result<HostCheck> {
    let base = state(&app).settings.read().unwrap().install_dir.clone();
    let docker_app = docker::desktop_apps()
        .into_iter()
        .next()
        .map(|(n, _)| n.to_string());
    let can_install_docker = docker::installer().is_some();
    let script = script::host_check_script(&script_dir(&base));
    let cap = match exec::run_capture(&script, Duration::from_secs(40)).await {
        Ok(c) => c,
        Err(e) => {
            return Ok(HostCheck {
                error: Some(e.to_string()),
                ..Default::default()
            })
        }
    };
    if cap.code != 0 {
        return Ok(HostCheck {
            error: Some(format!("Vérification impossible : {}", cap.stderr.trim())),
            ..Default::default()
        });
    }
    let r = exec::results(&cap.stdout);
    let get = |k: &str| r.get(k).cloned().unwrap_or_default();
    Ok(HostCheck {
        reachable: true,
        os: get("os"),
        git: get("git"),
        docker: get("docker"),
        compose: get("compose"),
        daemon: get("daemon") == "ok",
        docker_app,
        can_install_docker,
        mem_total: get("memtotal").parse().unwrap_or(0),
        disk_free: get("diskfree")
            .parse::<u64>()
            .map(|kb| kb * 1024)
            .unwrap_or(0),
        engine: get("engine"),
        error: None,
    })
}

/// Starts Docker's desktop app and waits for it, then checks again.
#[tauri::command]
pub async fn docker_start(app: AppHandle) -> Result<HostCheck> {
    docker::start().await?;
    check_host(app).await
}

/// Downloads and installs Docker Desktop (the user confirms in the system's
/// administrator dialog), then starts it. Progress: `docker-setup` events.
#[tauri::command]
pub async fn docker_install(app: AppHandle) -> Result<HostCheck> {
    docker::install(&app).await?;
    check_host(app).await
}

#[tauri::command]
pub fn docker_install_cancel() {
    docker::cancel_install();
}

fn find_app(catalog: &Catalog, id: &str) -> Result<AppDef> {
    catalog
        .app(id)
        .cloned()
        .ok_or_else(|| Error::msg(format!("Application inconnue : {id}")))
}

fn target_def(app: &AppDef) -> Result<TargetDef> {
    app.targets
        .local
        .clone()
        .ok_or_else(|| Error::msg(format!("{} ne s'installe pas sur un poste.", app.name)))
}

#[tauri::command]
pub fn default_install_dir(app: AppHandle, app_id: String) -> Result<String> {
    let base = state(&app).settings.read().unwrap().install_dir.clone();
    let sep = if cfg!(windows) { "\\" } else { "/" };
    Ok(format!(
        "{}{sep}{app_id}",
        base.trim_end_matches(['/', '\\'])
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallRequest {
    pub app_id: String,
    pub dir: String,
    pub values: BTreeMap<String, String>,
}

/// Validates form values against the catalog fields.
fn validate_values(def: &AppDef, t: &TargetDef, values: &BTreeMap<String, String>) -> Result<()> {
    let email_re = regex_lite::Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$").unwrap();
    for f in &t.fields {
        if !f.is_active(values) {
            continue;
        }
        let v = values.get(&f.key).map(|s| s.trim()).unwrap_or("");
        let v = if v.is_empty() {
            f.default.as_deref().unwrap_or("")
        } else {
            v
        };
        if v.contains(['\n', '\r']) {
            bail!("« {} » doit tenir sur une ligne.", f.label);
        }
        if def.dotenv_quote == crate::catalog::DotenvQuote::Single && v.contains('\'') {
            bail!("« {} » ne doit pas contenir d'apostrophe.", f.label);
        }
        if v.is_empty() {
            if f.required {
                bail!("Le champ « {} » est obligatoire.", f.label);
            }
            continue;
        }
        if let Some(min) = f.min_length {
            if v.chars().count() < min {
                bail!("« {} » doit faire au moins {min} caractères.", f.label);
            }
        }
        match f.kind {
            FieldKind::Email if !email_re.is_match(v) => {
                bail!("« {} » : adresse e-mail invalide.", f.label)
            }
            FieldKind::Url if !(v.starts_with("https://") || v.starts_with("http://")) => {
                bail!("« {} » doit commencer par https://", f.label)
            }
            FieldKind::Number if v.parse::<u32>().is_err() => {
                bail!("« {} » doit être un nombre.", f.label)
            }
            FieldKind::Select if !f.options.iter().any(|o| o.value == v) => {
                bail!("Choix invalide pour « {} ».", f.label)
            }
            _ => {}
        }
    }
    Ok(())
}

fn render_url(template: &str, t: &TargetDef, values: &BTreeMap<String, String>) -> String {
    let mut url = template.to_string();
    for f in &t.fields {
        let v = values
            .get(&f.key)
            .filter(|v| !v.is_empty())
            .or(f.default.as_ref())
            .cloned()
            .unwrap_or_default();
        url = url.replace(&format!("{{{{{}}}}}", f.key), &v);
    }
    url.trim_end_matches('/').to_string()
}

/// Windows paths are passed to Git Bash with forward slashes.
fn script_dir(dir: &str) -> String {
    if cfg!(windows) {
        dir.replace('\\', "/")
    } else {
        dir.to_string()
    }
}

#[tauri::command]
pub fn install_app(app: AppHandle, req: InstallRequest) -> Result<String> {
    let st = state(&app);
    let def = find_app(&st.catalog.read().unwrap().catalog, &req.app_id)?;
    if def.coming_soon {
        bail!("{} sera bientôt disponible.", def.name);
    }
    let tdef = target_def(&def)?;
    let dir = req.dir.trim().trim_end_matches(['/', '\\']).to_string();
    check_dir(&dir)?;
    let values: BTreeMap<String, String> = req
        .values
        .into_iter()
        .map(|(k, v)| (k, v.trim().to_string()))
        .collect();
    validate_values(&def, &tdef, &values)?;

    let existing_id = {
        let store = st.store.read().unwrap();
        if let Some(i) = store
            .installations
            .iter()
            .find(|i| i.app_id == def.id && i.state != InstallState::Failed)
        {
            bail!("{} est déjà installé sur ce poste ({}).", def.name, i.dir);
        }
        if store
            .installations
            .iter()
            .any(|i| i.dir == dir && i.app_id != def.id)
        {
            bail!("Ce dossier est déjà utilisé par une autre application.");
        }
        // A failed install of the same app is retried in place.
        store
            .installations
            .iter()
            .find(|i| i.app_id == def.id && i.state == InstallState::Failed)
            .map(|i| i.id.clone())
    };

    // Secrets are kept only for the duration of the script.
    let stored: BTreeMap<String, String> = tdef
        .fields
        .iter()
        .filter(|f| !f.kind.is_secret())
        .filter_map(|f| values.get(&f.key).map(|v| (f.key.clone(), v.clone())))
        .collect();
    let url = render_url(&tdef.url, &tdef, &values);
    let id = existing_id
        .unwrap_or_else(|| format!("inst-{}", &uuid::Uuid::new_v4().simple().to_string()[..10]));
    {
        let mut store = st.store.write().unwrap();
        store.installations.retain(|i| i.id != id);
        store.installations.push(Installation {
            id: id.clone(),
            app_id: def.id.clone(),
            target: "local".into(),
            dir: dir.clone(),
            values: stored,
            version: None,
            commit: None,
            url: Some(url),
            state: InstallState::Installing,
            installed_at: now(),
            updated_at: None,
            auto_update: false,
            notified_version: None,
        });
    }
    st.save_store()?;

    let git_ref = latest_ref(&app, &def);
    let token = st.token();
    let sdir = script_dir(&dir);
    let phases = script::build_phases(&Plan {
        app: &def,
        target: &tdef,
        action: Action::Install,
        dir: &sdir,
        values: &values,
        git_ref: (&git_ref.0, &git_ref.1),
        token: token.as_deref(),
        windows: cfg!(windows),
    });
    let op = ops::spawn(
        &app,
        OpRequest {
            installation_id: id,
            app_id: def.id.clone(),
            action: Action::Install,
            dir: PathBuf::from(&dir),
            source: code_source(&def, &git_ref, token),
            phases,
            steps: script::step_labels(&tdef, Action::Install),
        },
    );
    let _ = app.emit("overview-changed", ());
    op
}

fn code_source(def: &AppDef, git_ref: &(String, String), token: Option<String>) -> Source {
    Source {
        url: def.repo.clone_url(),
        ref_kind: git_ref.0.clone(),
        ref_name: git_ref.1.clone(),
        token,
    }
}

/// Tag of the latest release when the app publishes tags, else its branch.
fn latest_ref(app: &AppHandle, def: &AppDef) -> (String, String) {
    let st = state(app);
    let latest = st.latest.read().unwrap();
    match latest.get(&def.id).and_then(|l| l.latest.as_ref()) {
        Some(l) if l.ref_kind == "tag" => ("tag".into(), l.ref_name.clone()),
        _ => ("branch".into(), def.repo.branch.clone()),
    }
}

pub fn start_action(app: &AppHandle, installation_id: &str, action: Action) -> Result<String> {
    let st = state(app);
    let inst = st
        .store
        .read()
        .unwrap()
        .installation(installation_id)
        .cloned()
        .ok_or_else(|| Error::msg("Installation introuvable."))?;
    let def = find_app(&st.catalog.read().unwrap().catalog, &inst.app_id)?;
    let tdef = target_def(&def)?;
    if action == Action::Update && inst.state != InstallState::Ready {
        bail!("Terminez d'abord l'installation de {}.", def.name);
    }
    let git_ref = latest_ref(app, &def);
    let token = st.token();
    let sdir = script_dir(&inst.dir);
    let phases = script::build_phases(&Plan {
        app: &def,
        target: &tdef,
        action,
        dir: &sdir,
        values: &inst.values,
        git_ref: (&git_ref.0, &git_ref.1),
        token: token.as_deref(),
        windows: cfg!(windows),
    });
    ops::spawn(
        app,
        OpRequest {
            installation_id: inst.id.clone(),
            app_id: def.id.clone(),
            action,
            dir: PathBuf::from(&inst.dir),
            source: code_source(&def, &git_ref, token),
            phases,
            steps: script::step_labels(&tdef, action),
        },
    )
}

#[tauri::command]
pub fn run_action(app: AppHandle, installation_id: String, action: String) -> Result<String> {
    let action = match action.as_str() {
        "update" => Action::Update,
        "start" => Action::Start,
        "stop" => Action::Stop,
        "restart" => Action::Restart,
        "remove" => Action::Remove,
        _ => bail!("Action inconnue."),
    };
    let id = start_action(&app, &installation_id, action)?;
    let _ = app.emit("overview-changed", ());
    Ok(id)
}

/// Removes an installation from the launcher without touching the machine.
#[tauri::command]
pub fn forget_installation(app: AppHandle, installation_id: String) -> Result<()> {
    let st = state(&app);
    if st.ops.running_for(&installation_id).is_some() {
        bail!("Une opération est en cours : attendez sa fin ou annulez-la.");
    }
    st.store
        .write()
        .unwrap()
        .installations
        .retain(|i| i.id != installation_id);
    st.save_store()?;
    let _ = app.emit("overview-changed", ());
    Ok(())
}

#[tauri::command]
pub fn set_auto_update(app: AppHandle, installation_id: String, enabled: bool) -> Result<()> {
    let st = state(&app);
    {
        let mut store = st.store.write().unwrap();
        let inst = store
            .installation_mut(&installation_id)
            .ok_or_else(|| Error::msg("Installation introuvable."))?;
        inst.auto_update = enabled;
    }
    st.save_store()?;
    let _ = app.emit("overview-changed", ());
    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub name: String,
    pub state: String,
    pub health: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    /// `running`, `partial`, `stopped`, `absent` or `unknown`.
    pub state: String,
    pub services: Vec<ServiceStatus>,
    pub error: Option<String>,
}

/// `docker compose ps --format json` prints either a JSON array or one object per line.
pub fn parse_ps(out: &str) -> Vec<ServiceStatus> {
    #[derive(Deserialize)]
    struct Ps {
        #[serde(rename = "Service", default)]
        service: String,
        #[serde(rename = "State", default)]
        state: String,
        #[serde(rename = "Health", default)]
        health: String,
    }
    let to_status = |p: Ps| ServiceStatus {
        name: p.service,
        state: p.state,
        health: p.health,
    };
    let trimmed = out.trim();
    if trimmed.starts_with('[') {
        return serde_json::from_str::<Vec<Ps>>(trimmed)
            .map(|v| v.into_iter().map(to_status).collect())
            .unwrap_or_default();
    }
    trimmed
        .lines()
        .filter(|l| l.trim_start().starts_with('{'))
        .filter_map(|l| serde_json::from_str::<Ps>(l).ok())
        .map(to_status)
        .collect()
}

/// One-shot helpers (migrations…) legitimately exit, so an exited service
/// next to running ones is not a problem; restarting or unhealthy ones are.
pub fn summarize(services: &[ServiceStatus]) -> String {
    if services.is_empty() {
        return "absent".into();
    }
    if !services.iter().any(|s| s.state == "running") {
        return "stopped".into();
    }
    let troubled = services.iter().any(|s| {
        matches!(s.state.as_str(), "restarting" | "dead" | "paused") || s.health == "unhealthy"
    });
    if troubled { "partial" } else { "running" }.into()
}

#[tauri::command]
pub async fn installation_status(app: AppHandle, installation_id: String) -> Result<RuntimeStatus> {
    let (inst, workdir) = installation_with_workdir(&app, &installation_id)?;
    let script = script::status_script(&script_dir(&inst.dir), workdir.as_deref(), cfg!(windows));
    let cap = match exec::run_capture(&script, Duration::from_secs(30)).await {
        Ok(c) => c,
        Err(e) => {
            return Ok(RuntimeStatus {
                state: "unknown".into(),
                services: vec![],
                error: Some(e.to_string()),
            })
        }
    };
    if cap.code != 0 {
        let error = match cap.code {
            3 => "Dossier d'installation introuvable.".into(),
            _ => cap
                .stderr
                .lines()
                .last()
                .unwrap_or("Docker ne répond pas.")
                .to_string(),
        };
        return Ok(RuntimeStatus {
            state: "unknown".into(),
            services: vec![],
            error: Some(error),
        });
    }
    let services = parse_ps(&cap.stdout);
    Ok(RuntimeStatus {
        state: summarize(&services),
        services,
        error: None,
    })
}

#[tauri::command]
pub async fn installation_logs(app: AppHandle, installation_id: String) -> Result<String> {
    let (inst, workdir) = installation_with_workdir(&app, &installation_id)?;
    let script = script::logs_script(&script_dir(&inst.dir), workdir.as_deref(), cfg!(windows));
    let cap = exec::run_capture(&script, Duration::from_secs(45)).await?;
    if cap.code != 0 && cap.stdout.trim().is_empty() {
        bail!(
            "Journaux indisponibles : {}",
            cap.stderr.lines().last().unwrap_or("erreur inconnue")
        );
    }
    Ok(cap.stdout)
}

fn installation_with_workdir(app: &AppHandle, id: &str) -> Result<(Installation, Option<String>)> {
    let st = state(app);
    let inst = st
        .store
        .read()
        .unwrap()
        .installation(id)
        .cloned()
        .ok_or_else(|| Error::msg("Installation introuvable."))?;
    let catalog = st.catalog.read().unwrap();
    let workdir = catalog
        .catalog
        .app(&inst.app_id)
        .and_then(|a| a.targets.local.as_ref())
        .and_then(|t| t.workdir.clone());
    Ok((inst, workdir))
}

#[tauri::command]
pub fn get_op_log(app: AppHandle, op_id: String) -> Vec<LogLine> {
    state(&app).ops.log(&op_id)
}

#[tauri::command]
pub fn cancel_op(app: AppHandle, op_id: String) -> bool {
    state(&app).ops.cancel(&op_id)
}

#[tauri::command]
pub fn generate_secret(kind: String) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    match kind.as_str() {
        "hex32" => (0..32)
            .map(|_| format!("{:02x}", rng.gen::<u8>()))
            .collect(),
        _ => {
            // Readable password: no ambiguous characters, no quotes or shell specials.
            const CHARS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789";
            let groups: Vec<String> = (0..4)
                .map(|_| {
                    (0..5)
                        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
                        .collect()
                })
                .collect();
            groups.join("-")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compose_ps_both_formats() {
        let lines = "{\"Service\":\"app\",\"State\":\"running\",\"Health\":\"healthy\"}\n{\"Service\":\"db\",\"State\":\"running\",\"Health\":\"\"}\n";
        let v = parse_ps(lines);
        assert_eq!(v.len(), 2);
        assert_eq!(summarize(&v), "running");
        let arr = "[{\"Service\":\"app\",\"State\":\"exited\"}]";
        let v = parse_ps(arr);
        assert_eq!(v[0].state, "exited");
        assert_eq!(summarize(&v), "stopped");
        assert_eq!(summarize(&[]), "absent");
    }

    #[test]
    fn renders_urls() {
        let catalog = Catalog::parse(crate::catalog::EMBEDDED).unwrap();
        let mia = catalog.app("mia").unwrap();
        let t = mia.targets.local.as_ref().unwrap();
        let v = BTreeMap::new();
        assert_eq!(render_url(&t.url, t, &v), "http://localhost:8080");
    }

    #[test]
    fn validates_form_values() {
        let catalog = Catalog::parse(crate::catalog::EMBEDDED).unwrap();
        let mia = catalog.app("mia").unwrap();
        let t = mia.targets.local.as_ref().unwrap();
        let mut v = BTreeMap::new();
        assert!(validate_values(mia, t, &v).is_err());
        v.insert("MIA_ADMIN_EMAIL".into(), "admin@entreprise.fr".into());
        v.insert("MIA_ADMIN_PASSWORD".into(), "court".into());
        assert!(validate_values(mia, t, &v).is_err());
        v.insert("MIA_ADMIN_PASSWORD".into(), "assez-long-1".into());
        assert!(validate_values(mia, t, &v).is_ok());
        // Conditional fields are only required when active.
        let t: TargetDef = serde_json::from_str(
            r#"{"url": "http://localhost", "install": [], "update": [], "fields": [
                {"key": "MODE", "label": "Mode", "kind": "select", "required": true, "default": "a",
                 "options": [{"value": "a", "label": "A"}, {"value": "b", "label": "B"}]},
                {"key": "NET", "label": "Réseau", "kind": "text", "required": true,
                 "when": {"key": "MODE", "in": ["b"]}}]}"#,
        )
        .unwrap();
        let mut v = BTreeMap::new();
        assert!(validate_values(mia, &t, &v).is_ok());
        v.insert("MODE".into(), "b".into());
        assert!(validate_values(mia, &t, &v).is_err());
        v.insert("NET".into(), "web".into());
        assert!(validate_values(mia, &t, &v).is_ok());
        v.insert("MODE".into(), "c".into());
        assert!(validate_values(mia, &t, &v).is_err());
    }

    #[test]
    fn generates_passwords() {
        let p = generate_secret("password".into());
        assert_eq!(p.len(), 23);
        assert!(!p.contains(['\'', '"', '$']));
        assert_eq!(generate_secret("hex32".into()).len(), 64);
    }
}
