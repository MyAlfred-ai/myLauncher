//! Catalog refresh, update detection, notifications and automatic updates.

use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;

use crate::catalog::{Catalog, VersionSource};
use crate::error::Result;
use crate::github::Gh;
use crate::ops::OpInfo;
use crate::script::Action;
use crate::state::{now, state, LatestState};
use crate::store::InstallState;
use crate::{commands, version};

/// Fetches the remote catalog; keeps the current one if anything is wrong.
pub async fn refresh_catalog(app: &AppHandle) -> Result<()> {
    let st = state(app);
    let url = st.settings.read().unwrap().catalog_url.clone();
    if url.trim().is_empty() {
        return Ok(());
    }
    let gh = Gh::new(st.token());
    let text = gh.fetch_text(&url).await?;
    let remote = Catalog::parse(&text)?;
    let mut cur = st.catalog.write().unwrap();
    if remote.updated >= cur.catalog.updated {
        let _ = std::fs::write(st.catalog_cache_path(), &text);
        cur.catalog = remote;
        cur.source = "remote".into();
    }
    cur.fetched_at = Some(now());
    Ok(())
}

/// Checks every app of the catalog, then notifies and applies automatic updates.
pub async fn check_all(app: &AppHandle) {
    let st = state(app);
    if let Err(e) = refresh_catalog(app).await {
        log::info!("catalogue distant indisponible : {e}");
    }
    let apps = st.catalog.read().unwrap().catalog.apps.clone();
    let token = st.token();

    let mut set = tokio::task::JoinSet::new();
    for a in apps {
        let token = token.clone();
        set.spawn(async move {
            let gh = Gh::new(token);
            let r = gh.latest(&a).await;
            (a.id.clone(), r)
        });
    }
    while let Some(Ok((id, r))) = set.join_next().await {
        let entry = match r {
            Ok(l) => LatestState {
                latest: Some(l),
                error: None,
                checked_at: now(),
            },
            Err(e) => LatestState {
                latest: None,
                error: Some(e.to_string()),
                checked_at: now(),
            },
        };
        st.latest.write().unwrap().insert(id, entry);
    }
    *st.last_check.write().unwrap() = Some(now());
    let _ = app.emit("overview-changed", ());
    handle_new_updates(app);
}

fn handle_new_updates(app: &AppHandle) {
    let st = state(app);
    let notify = st.settings.read().unwrap().notify;
    let mut to_notify: Vec<String> = Vec::new();
    let mut to_apply: Vec<String> = Vec::new();
    {
        let latest = st.latest.read().unwrap();
        let catalog = st.catalog.read().unwrap();
        let mut store = st.store.write().unwrap();
        for inst in store
            .installations
            .iter_mut()
            .filter(|i| i.state == InstallState::Ready)
        {
            let Some(l) = latest.get(&inst.app_id).and_then(|s| s.latest.as_ref()) else {
                continue;
            };
            let by_commit = catalog
                .catalog
                .app(&inst.app_id)
                .is_some_and(|a| a.version.source == VersionSource::Commit);
            if !version::is_update(
                &l.version,
                l.commit.as_deref(),
                inst.version.as_deref(),
                inst.commit.as_deref(),
                by_commit,
            ) {
                continue;
            }
            if inst.auto_update {
                to_apply.push(inst.id.clone());
            } else if inst.notified_version.as_deref() != Some(l.version.as_str()) {
                inst.notified_version = Some(l.version.clone());
                let name = catalog
                    .catalog
                    .app(&inst.app_id)
                    .map(|a| a.name.clone())
                    .unwrap_or_default();
                to_notify.push(format!("{name} {} est disponible.", l.version));
            }
        }
    }
    let _ = st.save_store();
    if notify && !to_notify.is_empty() {
        let title = if to_notify.len() == 1 {
            "Mise à jour disponible"
        } else {
            "Mises à jour disponibles"
        };
        let _ = app
            .notification()
            .builder()
            .title(title)
            .body(to_notify.join("\n"))
            .show();
    }
    for id in to_apply {
        if st.ops.running_for(&id).is_none() {
            if let Err(e) = commands::start_action(app, &id, Action::Update) {
                log::warn!("mise à jour automatique impossible : {e}");
            }
        }
    }
}

/// Notifies the end of long operations.
pub fn notify_op(app: &AppHandle, op: &OpInfo) {
    let st = state(app);
    if !st.settings.read().unwrap().notify || !matches!(op.action.as_str(), "install" | "update") {
        return;
    }
    let name = st
        .catalog
        .read()
        .unwrap()
        .catalog
        .app(&op.app_id)
        .map(|a| a.name.clone())
        .unwrap_or_else(|| op.app_id.clone());
    let (title, body) = match (op.action.as_str(), op.status.as_str()) {
        ("install", "success") => (
            format!("{name} est installé"),
            format!("Prêt sur {}.", op.target_label),
        ),
        ("update", "success") => (
            format!("{name} est à jour"),
            format!("Mise à jour terminée sur {}.", op.target_label),
        ),
        (_, "cancelled") => return,
        _ => (
            format!("Échec : {name}"),
            op.error
                .clone()
                .unwrap_or_else(|| "Consultez le journal dans le launcher.".into()),
        ),
    };
    let _ = app.notification().builder().title(title).body(body).show();
}

pub fn spawn_background(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        loop {
            let (auto, hours) = {
                let st = state(&app);
                let s = st.settings.read().unwrap();
                (s.auto_check, s.check_interval_hours.clamp(1, 168))
            };
            if auto {
                check_all(&app).await;
            }
            tokio::time::sleep(Duration::from_secs(u64::from(hours) * 3600)).await;
        }
    });
}
