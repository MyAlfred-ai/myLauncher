//! Long-running operations (install, update, start…): one at a time per
//! installation, streamed to the UI and written to a log file.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;

use crate::error::{bail, Result};
use crate::exec::{self, Stream};
use crate::gitsync::{self, Source};
use crate::script::{sh_quote, Action, Phase, COMMIT_SLOT};
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
    /// Wakes a running script; the flag stops a code download.
    cancel: Arc<Notify>,
    cancelled: Arc<AtomicBool>,
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
                e.cancelled.store(true, Ordering::Relaxed);
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
    /// Install folder, where the code is fetched.
    pub dir: PathBuf,
    pub source: Source,
    pub phases: Vec<Phase>,
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
    let cancelled = Arc::new(AtomicBool::new(false));
    let info = OpInfo {
        id: id.clone(),
        installation_id: req.installation_id.clone(),
        app_id: req.app_id.clone(),
        action: action_name(req.action).into(),
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
            cancelled: cancelled.clone(),
        },
    );
    st.ops.prune();
    let _ = app.emit("op-state", &info);

    let app = app.clone();
    let op_id = id.clone();
    tauri::async_runtime::spawn(async move {
        run(app, op_id, req, cancel, cancelled, log_path).await;
    });
    Ok(id)
}

/// Collects an operation's output: log file, UI events, reported values.
struct Sink {
    app: AppHandle,
    op_id: String,
    file: Option<BufWriter<File>>,
    pending: Arc<Mutex<Vec<LogLine>>>,
    seq: u64,
    results: BTreeMap<String, String>,
    last_error: Option<String>,
}

impl Sink {
    fn step(&mut self, step: &str) {
        let st = state(&self.app);
        if let Some(info) = st.ops.update(&self.op_id, |e| {
            if let Some(i) = e.info.steps.iter().position(|s| s == step) {
                e.info.current_step = i;
            }
        }) {
            let _ = self.app.emit("op-state", &info);
        }
        self.push("step", step.to_string());
    }

    /// Interprets one line of script output (see the protocol in script.rs).
    fn script_line(&mut self, stream: Stream, text: String) {
        if let Some(step) = text.strip_prefix("::ml::step::") {
            let step = step.to_string();
            self.step(&step);
        } else if let Some(kv) = text.strip_prefix("::ml::result::") {
            if let Some((k, v)) = kv.split_once('=') {
                self.results.insert(k.to_string(), v.trim().to_string());
            }
        } else if let Some(w) = text.strip_prefix("::ml::warn::") {
            let w = w.to_string();
            state(&self.app)
                .ops
                .update(&self.op_id, |e| e.info.warnings.push(w.clone()));
            self.push("warn", w);
        } else if let Some(e) = text.strip_prefix("::ml::error::") {
            self.last_error = Some(e.to_string());
            self.push("error", e.to_string());
        } else {
            self.push(if stream == Stream::Err { "err" } else { "out" }, text);
        }
    }

    fn push(&mut self, kind: &str, text: String) {
        if let Some(f) = self.file.as_mut() {
            let _ = writeln!(f, "{text}");
        }
        self.seq += 1;
        let line = LogLine {
            op_id: self.op_id.clone(),
            seq: self.seq,
            kind: kind.into(),
            text,
        };
        state(&self.app).ops.update(&self.op_id, |e| {
            if e.log.len() >= MAX_LINES {
                e.log.drain(..MAX_LINES / 10);
            }
            e.log.push(line.clone());
        });
        self.pending.lock().unwrap().push(line);
    }
}

/// Fetches the code in a worker thread, streaming its progress messages.
async fn run_sync(
    sink: &mut Sink,
    dir: PathBuf,
    source: Source,
    cancelled: Arc<AtomicBool>,
) -> Result<String> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let mut job = tokio::task::spawn_blocking(move || {
        gitsync::sync(&dir, &source, &cancelled, |m| {
            let _ = tx.send(m);
        })
    });
    loop {
        tokio::select! {
            Some(m) = rx.recv() => sink.push("out", m),
            res = &mut job => {
                while let Ok(m) = rx.try_recv() {
                    sink.push("out", m);
                }
                return res.map_err(|e| crate::error::Error::msg(format!("Récupération du code interrompue : {e}")))?;
            }
        }
    }
}

async fn run(
    app: AppHandle,
    op_id: String,
    req: OpRequest,
    cancel: Arc<Notify>,
    cancelled: Arc<AtomicBool>,
    log_path: std::path::PathBuf,
) {
    let pending: Arc<Mutex<Vec<LogLine>>> = Arc::default();
    let mut file = File::create(&log_path).ok().map(BufWriter::new);
    if let Some(f) = file.as_mut() {
        let _ = writeln!(
            f,
            "# {} {} — {}",
            action_name(req.action),
            req.app_id,
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

    let mut sink = Sink {
        app: app.clone(),
        op_id: op_id.clone(),
        file,
        pending: pending.clone(),
        seq: 0,
        results: BTreeMap::new(),
        last_error: None,
    };

    // Docker Desktop is often simply not started: start it instead of failing.
    crate::docker::ensure_running(|m| sink.push("out", m)).await;

    // Commit checked out now; updated after each fetch of the code.
    let dir = req.dir.clone();
    let mut commit = tokio::task::spawn_blocking(move || gitsync::head_commit(&dir))
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let mut outcome: Result<Option<i32>> = Ok(Some(0));
    for phase in &req.phases {
        if cancelled.load(Ordering::Relaxed) {
            outcome = Ok(None);
            break;
        }
        outcome = match phase {
            Phase::Script(script) => {
                let script = script.replace(COMMIT_SLOT, &sh_quote(&commit));
                exec::run_streaming(&script, cancel.clone(), |stream, text| {
                    sink.script_line(stream, text)
                })
                .await
            }
            Phase::Sync { label } => {
                sink.step(label);
                match run_sync(
                    &mut sink,
                    req.dir.clone(),
                    req.source.clone(),
                    cancelled.clone(),
                )
                .await
                {
                    Ok(c) => {
                        commit = c;
                        sink.results.insert("commit".into(), commit.clone());
                        Ok(Some(0))
                    }
                    Err(_) if cancelled.load(Ordering::Relaxed) => Ok(None),
                    Err(e) => {
                        sink.last_error = Some(e.to_string());
                        sink.push("error", e.to_string());
                        Ok(Some(1))
                    }
                }
            }
        };
        if !matches!(outcome, Ok(Some(0))) {
            break;
        }
    }

    tokio::time::sleep(Duration::from_millis(250)).await;
    flusher.abort();
    let rest: Vec<LogLine> = std::mem::take(&mut *pending.lock().unwrap());
    if !rest.is_empty() {
        let _ = app.emit("op-log", &rest);
    }
    if let Some(mut f) = sink.file.take() {
        let _ = f.flush();
    }

    let (status, error) = match outcome {
        Ok(Some(0)) => ("success", None),
        Ok(Some(code)) => {
            let msg = sink.last_error.clone().unwrap_or_else(|| {
                format!("L'opération a échoué (code {code}). Consultez le journal.")
            });
            ("failed", Some(msg))
        }
        Ok(None) => ("cancelled", Some("Opération annulée.".to_string())),
        Err(e) => ("failed", Some(e.to_string())),
    };

    let st = state(&app);
    let info = st.ops.update(&op_id, |e| {
        e.info.status = status.into();
        e.info.error = error.clone();
        e.info.ended_at = Some(now());
        if status == "success" {
            e.info.current_step = e.info.steps.len();
        }
    });
    apply_outcome(&app, &req, status, &sink.results);
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
