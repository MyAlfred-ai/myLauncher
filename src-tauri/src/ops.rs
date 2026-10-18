//! Long-running operations (install, update, start…): one at a time per
//! installation, streamed to the UI and written to a log file.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;

use crate::error::{bail, Result};
use crate::exec::{self, Stream, Target};
use crate::script::Action;
use crate::state::{now, state};
use crate::store::InstallState;

const MAX_LINES: usize = 5000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpInfo {
    pub id: String,
    pub installation_id: String,
    pub app_id: String,
    pub action: String,
    pub target_label: String,
    /// `running`, `success`, `failed` or `cancelled`.
    pub status: String,
    pub steps: Vec<String>,
    pub current_step: usize,
    pub error: Option<String>,
    pub warnings: Vec<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub log_file: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub op_id: String,
    /// Position in the operation's output, to merge snapshots and live events.
    pub seq: u64,
    /// `out`, `err`, `step`, `warn` or `error`.
    pub kind: String,
    pub text: String,
}

struct Entry {
    info: OpInfo,
    log: Vec<LogLine>,
    cancel: Arc<Notify>,
}

#[derive(Default)]
pub struct OpManager {
    inner: Mutex<HashMap<String, Entry>>,
}

impl OpManager {
    pub fn list(&self) -> Vec<OpInfo> {
        let mut v: Vec<OpInfo> = self
            .inner
            .lock()
            .unwrap()
            .values()
            .map(|e| e.info.clone())
            .collect();
        v.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        v
    }

    pub fn log(&self, op_id: &str) -> Vec<LogLine> {
        self.inner
            .lock()
            .unwrap()
            .get(op_id)
            .map(|e| e.log.clone())
            .unwrap_or_default()
    }

    pub fn running_for(&self, installation_id: &str) -> Option<String> {
        self.inner
            .lock()
            .unwrap()
            .values()
            .find(|e| e.info.installation_id == installation_id && e.info.status == "running")
            .map(|e| e.info.id.clone())
    }

    pub fn cancel(&self, op_id: &str) -> bool {
        match self.inner.lock().unwrap().get(op_id) {
            Some(e) if e.info.status == "running" => {
                e.cancel.notify_one();
                true
            }
            _ => false,
        }
    }

    fn update<F: FnOnce(&mut Entry)>(&self, op_id: &str, f: F) -> Option<OpInfo> {
        let mut map = self.inner.lock().unwrap();
        let e = map.get_mut(op_id)?;
        f(e);
        Some(e.info.clone())
    }

    /// Keeps the 30 most recent finished operations.
    fn prune(&self) {
        let mut map = self.inner.lock().unwrap();
        let mut finished: Vec<(String, String)> = map
            .values()
            .filter(|e| e.info.status != "running")
            .map(|e| (e.info.started_at.clone(), e.info.id.clone()))
            .collect();
        if finished.len() > 30 {
            finished.sort();
            for (_, id) in finished.iter().take(finished.len() - 30) {
                map.remove(id);
            }
        }
    }
}

pub struct OpRequest {
    pub installation_id: String,
    pub app_id: String,
    pub action: Action,
    pub target: Target,
    pub target_label: String,
    pub script: String,
    pub steps: Vec<String>,
}

pub fn action_name(a: Action) -> &'static str {
    match a {
        Action::Install => "install",
        Action::Update => "update",
        Action::Start => "start",
        Action::Stop => "stop",
        Action::Restart => "restart",
        Action::Remove => "remove",
    }
}

pub fn spawn(app: &AppHandle, req: OpRequest) -> Result<String> {
    let st = state(app);
    if st.ops.running_for(&req.installation_id).is_some() {
        bail!("Une opération est déjà en cours pour cette installation.");
    }
    let id = uuid::Uuid::new_v4().to_string();
    let started = now();
    let log_path = {
        let dir = st.log_dir.join("operations");
        let _ = std::fs::create_dir_all(&dir);
        let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
        dir.join(format!(
            "{stamp}-{}-{}.log",
            req.app_id,
            action_name(req.action)
        ))
    };
    let cancel = Arc::new(Notify::new());
    let info = OpInfo {
        id: id.clone(),
        installation_id: req.installation_id.clone(),
        app_id: req.app_id.clone(),
        action: action_name(req.action).into(),
        target_label: req.target_label.clone(),
        status: "running".into(),
        steps: req.steps.clone(),
        current_step: 0,
        error: None,
        warnings: vec![],
        started_at: started,
        ended_at: None,
        log_file: Some(log_path.to_string_lossy().into_owned()),
    };
    st.ops.inner.lock().unwrap().insert(
        id.clone(),
        Entry {
            info: info.clone(),
            log: Vec::new(),
            cancel: cancel.clone(),
        },
    );
    st.ops.prune();
    let _ = app.emit("op-state", &info);

    let app = app.clone();
    let op_id = id.clone();
    tauri::async_runtime::spawn(async move {
        run(app, op_id, req, cancel, log_path).await;
    });
    Ok(id)
}

async fn run(
    app: AppHandle,
    op_id: String,
    req: OpRequest,
    cancel: Arc<Notify>,
    log_path: std::path::PathBuf,
) {
    let pending: Arc<Mutex<Vec<LogLine>>> = Arc::default();
    let mut file = File::create(&log_path).ok().map(BufWriter::new);
    if let Some(f) = file.as_mut() {
        let _ = writeln!(
            f,
            "# {} {} — {} — {}",
            action_name(req.action),
            req.app_id,
            req.target_label,
            chrono::Local::now().to_rfc2822()
        );
    }

    // Flush log lines to the UI in batches: builds can print thousands of lines.
    let flusher = {
        let app = app.clone();
        let pending = pending.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                let batch: Vec<LogLine> = std::mem::take(&mut *pending.lock().unwrap());
                if !batch.is_empty() {
                    let _ = app.emit("op-log", &batch);
                }
            }
        })
    };

    let mut results: BTreeMap<String, String> = BTreeMap::new();
    let mut last_error: Option<String> = None;
    let mut stderr_tail: Vec<String> = Vec::new();
    let mut seq: u64 = 0;
    let st = state(&app);

    let outcome = exec::run_streaming(&req.target, &req.script, cancel, |stream, text| {
        let (kind, shown) = if let Some(step) = text.strip_prefix("::ml::step::") {
            let step = step.to_string();
            if let Some(info) = st.ops.update(&op_id, |e| {
                if let Some(i) = e.info.steps.iter().position(|s| *s == step) {
                    e.info.current_step = i;
                }
            }) {
                let _ = app.emit("op-state", &info);
            }
            ("step", step)
        } else if let Some(kv) = text.strip_prefix("::ml::result::") {
            if let Some((k, v)) = kv.split_once('=') {
                results.insert(k.to_string(), v.trim().to_string());
            }
            return;
        } else if let Some(w) = text.strip_prefix("::ml::warn::") {
            let w = w.to_string();
            st.ops.update(&op_id, |e| e.info.warnings.push(w.clone()));
            ("warn", w)
        } else if let Some(e) = text.strip_prefix("::ml::error::") {
            last_error = Some(e.to_string());
            ("error", e.to_string())
        } else {
            if stream == Stream::Err && !text.trim().is_empty() {
                stderr_tail.push(text.clone());
                if stderr_tail.len() > 5 {
                    stderr_tail.remove(0);
                }
            }
            (if stream == Stream::Err { "err" } else { "out" }, text)
        };
        if let Some(f) = file.as_mut() {
            let _ = writeln!(f, "{shown}");
        }
        seq += 1;
        let line = LogLine {
            op_id: op_id.clone(),
            seq,
            kind: kind.into(),
            text: shown,
        };
        st.ops.update(&op_id, |e| {
            if e.log.len() >= MAX_LINES {
                e.log.drain(..MAX_LINES / 10);
            }
            e.log.push(line.clone());
        });
        pending.lock().unwrap().push(line);
    })
    .await;

    tokio::time::sleep(Duration::from_millis(250)).await;
    flusher.abort();
    let rest: Vec<LogLine> = std::mem::take(&mut *pending.lock().unwrap());
    if !rest.is_empty() {
        let _ = app.emit("op-log", &rest);
    }
    if let Some(mut f) = file {
        let _ = f.flush();
    }

    let (status, error) = match outcome {
        Ok(Some(0)) => ("success", None),
        Ok(Some(code)) => {
            let msg = last_error.unwrap_or_else(|| match &req.target {
                Target::Ssh(_) if code == 255 => exec::ssh_error(&stderr_tail.join("\n")),
                _ => format!("L'opération a échoué (code {code}). Consultez le journal."),
            });
            ("failed", Some(msg))
        }
        Ok(None) => ("cancelled", Some("Opération annulée.".to_string())),
        Err(e) => ("failed", Some(e.to_string())),
    };

    let info = st.ops.update(&op_id, |e| {
        e.info.status = status.into();
        e.info.error = error.clone();
        e.info.ended_at = Some(now());
        if status == "success" {
            e.info.current_step = e.info.steps.len();
        }
    });
    apply_outcome(&app, &req, status, &results);
    if let Some(info) = info {
        let _ = app.emit("op-state", &info);
        crate::updates::notify_op(&app, &info);
    }
    let _ = app.emit("overview-changed", ());
}

/// Records what an operation changed in the installation registry.
fn apply_outcome(
    app: &AppHandle,
    req: &OpRequest,
    status: &str,
    results: &BTreeMap<String, String>,
) {
    let st = state(app);
    {
        let mut store = st.store.write().unwrap();
        let ok = status == "success";
        if req.action == Action::Remove && ok {
            store.installations.retain(|i| i.id != req.installation_id);
        } else if let Some(inst) = store.installation_mut(&req.installation_id) {
            if let Some(v) = results.get("version").filter(|v| !v.is_empty()) {
                inst.version = Some(v.clone());
            }
            if let Some(c) = results.get("commit").filter(|c| !c.is_empty()) {
                inst.commit = Some(c.clone());
            }
            match (req.action, ok) {
                (Action::Install, true) => inst.state = InstallState::Ready,
                (Action::Install, false) => inst.state = InstallState::Failed,
                (Action::Update, true) => {
                    inst.updated_at = Some(now());
                    inst.notified_version = None;
                }
                _ => {}
            }
        }
    }
    if let Err(e) = st.save_store() {
        log::error!("sauvegarde de l'état impossible : {e}");
    }
}
