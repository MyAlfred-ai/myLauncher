//! Builds the bash script that performs an operation. The same script runs
//! locally (`bash -s`) and on a server (`ssh … bash -s`): it is sent on stdin,
//! so secrets never appear in a command line.
//!
//! Output protocol, parsed by `ops.rs`:
//!   `::ml::step::<label>`   a new step starts
//!   `::ml::warn::<text>`    a non-blocking warning
//!   `::ml::error::<text>`   a readable failure reason
//!   `::ml::result::k=v`     a value reported back (version, commit…)

use std::collections::BTreeMap;

use base64::Engine;

use crate::catalog::{AppDef, DotenvQuote, Step, TargetDef, VersionSource};

const PRELUDE: &str = r#"set -Eeuo pipefail
umask 022
export DOCKER_CLI_HINTS=false
ML_STEP=""
ml_step() { ML_STEP="$*"; printf '::ml::step::%s\n' "$*"; }
ml_info() { printf '%s\n' "$*"; }
ml_warn() { printf '::ml::warn::%s\n' "$*"; }
ml_fail() { printf '::ml::error::%s\n' "$*" >&2; exit 1; }
trap 'rc=$?; printf "::ml::error::Échec pendant « %s » (code %s).\n" "$ML_STEP" "$rc" >&2' ERR
ml_b64d() { printf '%s' "$1" | base64 -d 2>/dev/null || printf '%s' "$1" | base64 -D; }
ml_rand_hex() { openssl rand -hex "$1" 2>/dev/null || od -An -N"$1" -tx1 /dev/urandom | tr -d ' \n'; }
ml_rand_b64() { { openssl rand -base64 "$1" 2>/dev/null || head -c "$1" /dev/urandom | base64; } | tr -d '\n'; }
ml_compose() { docker compose "$@"; }
ml_require() { command -v "$1" >/dev/null 2>&1 || ml_fail "$2"; }
# The launcher fetches the code itself: git is optional on the machine. When
# it is missing, the few read-only calls of the apps' scripts are answered
# from what the launcher knows (exported, so child bash scripts see it).
if ! command -v git >/dev/null 2>&1; then
  git() {
    case "$*" in
      "rev-parse HEAD") printf '%s\n' "$ML_COMMIT";;
      "rev-parse --short HEAD"|"rev-parse --short=7 HEAD") printf '%s\n' "${ML_COMMIT:0:7}";;
      *) printf 'git est absent de ce poste : « git %s » ignoré.\n' "$*" >&2; return 1;;
    esac
  }
  export -f git
fi
ml_check_prereqs() {
  ml_require docker "Docker est introuvable. Installez Docker Desktop (poste) ou Docker Engine (serveur)."
  docker compose version >/dev/null 2>&1 || ml_fail "Docker Compose v2 est introuvable (commande « docker compose »)."
  docker info >/dev/null 2>&1 || ml_fail "Docker ne répond pas. Démarrez Docker Desktop (ou le service docker), puis réessayez."
}
# Sets KEY=VALUE in a dotenv file, in place, keeping the other lines.
ml_dotenv() {
  local key="$1" val="$2" file="${3:-.env}" line out found=0 re
  case "$val" in *$'\n'*) ml_fail "Valeur sur plusieurs lignes refusée pour $key.";; esac
  case "$ML_DOTENV_QUOTE" in
    single)
      case "$val" in *"'"*) ml_fail "Apostrophe refusée dans la valeur de $key.";; esac
      out="$key='$val'";;
    *) out="$key=$val";;
  esac
  [ -f "$file" ] || { : > "$file"; chmod 600 "$file" 2>/dev/null || true; }
  re="^[[:space:]]*(export[[:space:]]+)?${key}[[:space:]]*="
  {
    while IFS= read -r line || [ -n "$line" ]; do
      if [[ $line =~ $re ]]; then
        [ "$found" = 1 ] || printf '%s\n' "$out"
        found=1
      else
        printf '%s\n' "$line"
      fi
    done < "$file"
    [ "$found" = 1 ] || printf '%s\n' "$out"
  } > "$file.ml-tmp"
  cat "$file.ml-tmp" > "$file"
  rm -f "$file.ml-tmp"
}
ml_dotenv_get() {
  local file="${2:-.env}"
  [ -f "$file" ] || return 0
  sed -n -E "s/^[[:space:]]*(export[[:space:]]+)?$1[[:space:]]*=[[:space:]]*//p" "$file" | tail -n 1 | sed -E "s/^'(.*)'\$/\\1/; s/^\"(.*)\"\$/\\1/"
}
# Sets a value only when it is missing or a placeholder: generated secrets are never replaced.
ml_dotenv_default() {
  local cur
  cur="$(ml_dotenv_get "$1" "${3:-.env}")"
  case "$cur" in
    ""|CHANGE_ME|CHANGEME|changeme|change-me*) ml_dotenv "$1" "$2" "${3:-.env}";;
  esac
}
ml_wait() {
  local timeout="$1" start=$SECONDS; shift
  until "$@" >/dev/null 2>&1; do
    [ $((SECONDS - start)) -lt "$timeout" ] || return 1
    sleep 3
  done
}
ml_http_ok() {
  if command -v curl >/dev/null 2>&1; then curl -fsSk -o /dev/null --max-time 5 "$1"
  else wget -q -O /dev/null --no-check-certificate -T 5 "$1"; fi
}
ml_container_http() {
  ml_compose exec -T "$1" sh -c 'curl -fsS -o /dev/null --max-time 5 "$0" 2>/dev/null || wget -q -O /dev/null -T 5 "$0"' "$2"
}
ml_ghcr_login() {
  [ -n "${ML_GH_TOKEN:-}" ] || return 1
  printf '%s' "$ML_GH_TOKEN" | docker login ghcr.io -u x-access-token --password-stdin >/dev/null
}
ml_pull() {
  ml_compose pull "$@" && return 0
  ml_info "Registre d'images refusé : connexion à ghcr.io avec le token GitHub…"
  ml_ghcr_login || ml_fail "Images inaccessibles. Ajoutez un token GitHub (droit read:packages) dans les réglages du launcher."
  ml_compose pull "$@"
}
"#;

/// What the script must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Install,
    Update,
    Start,
    Stop,
    Restart,
    Remove,
}

/// Where the code lives and which version to fetch.
pub struct Plan<'a> {
    pub app: &'a AppDef,
    pub target: &'a TargetDef,
    pub action: Action,
    /// Install directory; may start with `~/` (expanded on the target).
    pub dir: &'a str,
    /// Form values (secrets included for install), keyed by field key.
    pub values: &'a BTreeMap<String, String>,
    /// `("tag", "v1.2.0")` or `("branch", "main")`.
    pub git_ref: (&'a str, &'a str),
    pub token: Option<&'a str>,
    /// Running under Git Bash on Windows.
    pub windows: bool,
}

pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// `~/x` → `"$HOME"/'x'`, otherwise single-quoted.
fn shell_path(p: &str) -> String {
    if p == "~" {
        "\"$HOME\"".into()
    } else if let Some(rest) = p.strip_prefix("~/") {
        format!("\"$HOME\"/{}", sh_quote(rest))
    } else {
        sh_quote(p)
    }
}

fn b64(s: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(s)
}

/// Replaced by the operation runner with the commit checked out at the time
/// each script phase starts (it changes after a sync).
pub const COMMIT_SLOT: &str = "@@ML_COMMIT@@";

/// An operation is a sequence of bash scripts, with the code fetched by the
/// launcher itself in between (`@sync` steps of the catalog).
#[derive(Debug, Clone)]
pub enum Phase {
    Script(String),
    Sync { label: String },
}

fn wrap(plan: &Plan, body: &str) -> String {
    let mut s = String::with_capacity(16 * 1024);
    // The whole script is one compound command: bash reads it entirely before
    // running it, so commands inside can never swallow the rest of stdin.
    s.push_str("{\n");
    s.push_str(PRELUDE);
    s.push_str(&context_block(plan));
    s.push_str(body);
    s.push_str("} < /dev/null\n");
    s
}

pub fn build_phases(plan: &Plan) -> Vec<Phase> {
    let steps = steps_for(plan.target, plan.action);
    let mut phases = Vec::new();
    let mut body = String::from("ml_step 'Vérification des prérequis'\nml_check_prereqs\n");
    if !matches!(plan.action, Action::Install) {
        body.push_str(
            "[ -d \"$ML_WORK\" ] || ml_fail \"Dossier d'installation introuvable : $ML_WORK\"\n",
        );
    }
    for step in &steps {
        if step.is_sync() {
            phases.push(Phase::Script(wrap(plan, &body)));
            phases.push(Phase::Sync {
                label: step.label.clone(),
            });
            body.clear();
        } else {
            body.push_str(&format!("ml_step {}\n", sh_quote(&step.label)));
            body.push_str("cd \"$ML_WORK\"\n");
            body.push_str(step.run.trim_end());
            body.push('\n');
        }
    }
    body.push_str("ml_report\n");
    phases.push(Phase::Script(wrap(plan, &body)));
    phases
}

fn context_block(plan: &Plan) -> String {
    let mut s = String::new();
    let app = plan.app;
    let workdir = plan.target.workdir.as_deref().unwrap_or("");
    s.push_str(&format!("ML_DIR={}\n", shell_path(plan.dir)));
    if workdir.is_empty() {
        s.push_str("ML_WORK=\"$ML_DIR\"\n");
    } else {
        s.push_str(&format!("ML_WORK=\"$ML_DIR\"/{}\n", sh_quote(workdir)));
    }
    s.push_str(&format!("ML_APP={}\n", sh_quote(&app.id)));
    s.push_str(&format!(
        "ML_REPO_URL={}\n",
        sh_quote(&app.repo.clone_url())
    ));
    s.push_str(&format!("ML_REF_KIND={}\n", sh_quote(plan.git_ref.0)));
    s.push_str(&format!("ML_REF={}\n", sh_quote(plan.git_ref.1)));
    let quote = match app.dotenv_quote {
        DotenvQuote::Single => "single",
        DotenvQuote::None => "none",
    };
    s.push_str(&format!("ML_DOTENV_QUOTE={quote}\n"));
    if plan.windows {
        // Compose files are listed with ':' in the apps' .env files.
        s.push_str("export COMPOSE_PATH_SEPARATOR=:\nexport MSYS_NO_PATHCONV=1\n");
    }
    // Only used to log in to ghcr.io when images are private.
    match plan.token {
        Some(t) if !t.is_empty() => s.push_str(&format!(
            "ML_GH_TOKEN=\"$(ml_b64d {})\"\n",
            sh_quote(&b64(t))
        )),
        _ => s.push_str("ML_GH_TOKEN=\"\"\n"),
    }
    s.push_str(&format!("ML_COMMIT={COMMIT_SLOT}\nexport ML_COMMIT\n"));
    // Every field of the target is defined (empty when not given), so steps
    // can use them under `set -u`. They are shell variables, not exported:
    // a step exports what its own tools need.
    for f in &plan.target.fields {
        let v = plan
            .values
            .get(&f.key)
            .cloned()
            .or_else(|| f.default.clone())
            .unwrap_or_default();
        s.push_str(&format!(
            "{}=\"$(ml_b64d {})\"\n",
            f.key,
            sh_quote(&b64(&v))
        ));
    }
    s.push_str(&report_fn(app));
    s
}

fn report_fn(app: &AppDef) -> String {
    let detect = match app.version.source {
        VersionSource::File => {
            let path = app.version.path.as_deref().unwrap_or("VERSION");
            match &app.version.json_key {
                Some(k) => format!(
                    "sed -n -E 's/.*\"{k}\"[[:space:]]*:[[:space:]]*\"([^\"]*)\".*/\\1/p' {} | head -n 1",
                    sh_quote(path)
                ),
                None => format!("tr -d '[:space:]' < {}", sh_quote(path)),
            }
        }
        VersionSource::Tags | VersionSource::Releases => {
            "if [ \"$ML_REF_KIND\" = tag ]; then printf '%s' \"$ML_REF\"; else printf '%s' \"${ML_COMMIT:0:7}\"; fi".into()
        }
        VersionSource::Commit => "printf '%s' \"${ML_COMMIT:0:7}\"".into(),
    };
    format!(
        "ml_report() {{\n  [ -n \"$ML_COMMIT\" ] || return 0\n  cd \"$ML_DIR\" 2>/dev/null || return 0\n  printf '::ml::result::commit=%s\\n' \"$ML_COMMIT\"\n  printf '::ml::result::version=%s\\n' \"$({detect} 2>/dev/null || true)\"\n}}\n"
    )
}

fn default_steps(action: Action) -> Vec<Step> {
    let step = |label: &str, run: &str| Step {
        label: label.into(),
        run: run.into(),
    };
    // An explicit timeout: Docker Desktop can give containers 1 s before
    // SIGKILL, which cuts a database off in the middle of its shutdown.
    match action {
        Action::Start => vec![step("Démarrage", "ml_compose up -d")],
        Action::Stop => vec![step("Arrêt", "ml_compose stop --timeout 60")],
        Action::Remove => vec![step(
            "Arrêt et suppression des conteneurs (les données sont conservées)",
            "ml_compose down --remove-orphans --timeout 60",
        )],
        _ => vec![],
    }
}

/// Steps of an action; install and update always fetch the code, even if a
/// catalog forgets it.
fn steps_for(t: &TargetDef, action: Action) -> Vec<Step> {
    let mut steps: Vec<Step> = match action {
        Action::Install => t.install.clone(),
        Action::Update => t.update.clone(),
        Action::Start => non_empty(&t.start).unwrap_or_else(|| default_steps(Action::Start)),
        Action::Stop => non_empty(&t.stop).unwrap_or_else(|| default_steps(Action::Stop)),
        Action::Restart => {
            let mut v = non_empty(&t.stop).unwrap_or_else(|| default_steps(Action::Stop));
            v.extend(non_empty(&t.start).unwrap_or_else(|| default_steps(Action::Start)));
            v
        }
        Action::Remove => default_steps(Action::Remove),
    };
    if matches!(action, Action::Install | Action::Update) && !steps.iter().any(Step::is_sync) {
        steps.insert(
            0,
            Step {
                label: "Récupération du code".into(),
                run: "@sync".into(),
            },
        );
    }
    steps
}

fn non_empty(v: &[Step]) -> Option<Vec<Step>> {
    (!v.is_empty()).then(|| v.to_vec())
}

/// Lists the step labels in order, for the progress display.
pub fn step_labels(target: &TargetDef, action: Action) -> Vec<String> {
    let mut labels = vec!["Vérification des prérequis".to_string()];
    labels.extend(steps_for(target, action).into_iter().map(|s| s.label));
    labels
}

/// Probes this machine: Docker, its memory, free disk space where apps are
/// installed (the nearest existing parent of `base`). Read-only.
pub fn host_check_script(base: &str) -> String {
    format!(
        r#"{{
r() {{ printf '::ml::result::%s=%s\n' "$1" "$2"; }}
r os "$(uname -sm 2>/dev/null || echo inconnu)"
if command -v git >/dev/null 2>&1; then r git "$(git --version 2>/dev/null)"; else r git ""; fi
if command -v docker >/dev/null 2>&1; then
  r docker "$(docker --version 2>/dev/null)"
  r compose "$(docker compose version --short 2>/dev/null || true)"
  if docker info >/dev/null 2>&1; then
    r daemon ok
    r memtotal "$(docker info --format '{{{{.MemTotal}}}}' 2>/dev/null)"
    r engine "$(docker info --format '{{{{.OperatingSystem}}}}' 2>/dev/null)"
  else r daemon ""; fi
else
  r docker ""; r compose ""; r daemon ""
fi
d={base}
while [ ! -d "$d" ]; do p="$(dirname "$d")"; [ "$p" = "$d" ] && break; d="$p"; done
r diskfree "$(df -Pk "$d" 2>/dev/null | awk 'NR==2 {{print $4}}')"
}} < /dev/null
"#,
        base = shell_path(base)
    )
}

/// Lists the containers of an installation, as JSON lines.
pub fn status_script(dir: &str, workdir: Option<&str>, windows: bool) -> String {
    let mut s = String::from("{\n");
    if windows {
        s.push_str("export COMPOSE_PATH_SEPARATOR=:\n");
    }
    s.push_str(&format!("ML_DIR={}\n", shell_path(dir)));
    match workdir {
        Some(w) if !w.is_empty() => {
            s.push_str(&format!("cd \"$ML_DIR\"/{} || exit 3\n", sh_quote(w)))
        }
        _ => s.push_str("cd \"$ML_DIR\" || exit 3\n"),
    }
    s.push_str("docker compose ps --all --format json\n} < /dev/null\n");
    s
}

/// Last lines of the containers' logs.
pub fn logs_script(dir: &str, workdir: Option<&str>, windows: bool) -> String {
    let mut s = status_script(dir, workdir, windows);
    s = s.replace(
        "docker compose ps --all --format json",
        "docker compose logs --no-color --tail 300 --timestamps",
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Catalog, EMBEDDED};

    #[test]
    fn quotes_shell_strings() {
        assert_eq!(sh_quote("abc"), "'abc'");
        assert_eq!(sh_quote("it's"), r"'it'\''s'");
        assert_eq!(shell_path("~/myalfred/mia"), "\"$HOME\"/'myalfred/mia'");
        assert_eq!(shell_path("/opt/x y"), "'/opt/x y'");
    }

    fn scripts(phases: &[Phase]) -> Vec<&str> {
        phases
            .iter()
            .filter_map(|p| match p {
                Phase::Script(s) => Some(s.as_str()),
                Phase::Sync { .. } => None,
            })
            .collect()
    }

    #[test]
    fn secrets_are_never_in_clear() {
        let catalog = Catalog::parse(EMBEDDED).unwrap();
        let app = catalog.app("mia").unwrap();
        let target = app.targets.local.as_ref().unwrap();
        let mut values = BTreeMap::new();
        values.insert("MIA_ADMIN_PASSWORD".to_string(), "S3cret!pass".to_string());
        let phases = build_phases(&Plan {
            app,
            target,
            action: Action::Install,
            dir: "~/MyAlfred/mia",
            values: &values,
            git_ref: ("branch", "main"),
            token: Some("ghp_secretToken"),
            windows: false,
        });
        assert!(phases.iter().any(|p| matches!(p, Phase::Sync { .. })));
        for script in scripts(&phases) {
            assert!(!script.contains("S3cret!pass"));
            assert!(!script.contains("ghp_secretToken"));
            assert!(script.starts_with("{\n"));
            assert!(script.trim_end().ends_with("} < /dev/null"));
            assert!(script.contains(COMMIT_SLOT));
            // Defaults are applied to fields left empty.
            assert!(script.contains(&format!("MIA_PORT=\"$(ml_b64d '{}')\"", b64("8080"))));
        }
    }

    fn bash() -> &'static str {
        // macOS ships bash 3.2: the oldest version the scripts must support.
        if std::path::Path::new("/bin/bash").exists() {
            "/bin/bash"
        } else {
            "bash"
        }
    }

    fn run_bash(script: &str, cwd: &std::path::Path) -> std::process::Output {
        use std::io::Write;
        let mut child = std::process::Command::new(bash())
            .arg("-s")
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }

    #[test]
    fn every_generated_script_is_valid_bash() {
        if cfg!(windows) {
            return;
        }
        let catalog = Catalog::parse(EMBEDDED).unwrap();
        let values = BTreeMap::new();
        for app in &catalog.apps {
            if let Some(target) = &app.targets.local {
                for action in [
                    Action::Install,
                    Action::Update,
                    Action::Start,
                    Action::Stop,
                    Action::Restart,
                    Action::Remove,
                ] {
                    let phases = build_phases(&Plan {
                        app,
                        target,
                        action,
                        dir: "~/MyAlfred/x",
                        values: &values,
                        git_ref: ("branch", "main"),
                        token: Some("tok"),
                        windows: false,
                    });
                    for script in scripts(&phases) {
                        let script = script.replace(COMMIT_SLOT, "'abc1234'");
                        let out = std::process::Command::new(bash())
                            .args(["-n", "-c", &script])
                            .output()
                            .unwrap();
                        assert!(
                            out.status.success(),
                            "{} {:?}: {}",
                            app.id,
                            action,
                            String::from_utf8_lossy(&out.stderr)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn dotenv_helpers_edit_in_place() {
        if cfg!(windows) {
            return;
        }
        let dir = std::env::temp_dir().join(format!("ml-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(".env"),
            "# comment\nA=1\nexport B=old\nMONGO_PASS=\nKEEP=x\n",
        )
        .unwrap();
        let script = format!(
            "{{\n{PRELUDE}\nML_DOTENV_QUOTE=none\nml_dotenv A 2\nml_dotenv B 'new value'\nml_dotenv C 3\n\
             ml_dotenv_default MONGO_PASS generated\nml_dotenv_default KEEP other\n\
             printf 'got=%s\\n' \"$(ml_dotenv_get B)\"\nml_dotenv X y sub.env\n\
             ML_DOTENV_QUOTE=single\nml_dotenv Q 'a b'\n}} < /dev/null\n"
        );
        let out = run_bash(&script, &dir);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let env = std::fs::read_to_string(dir.join(".env")).unwrap();
        assert_eq!(
            env,
            "# comment\nA=2\nB=new value\nMONGO_PASS=generated\nKEEP=x\nC=3\nQ='a b'\n"
        );
        assert!(String::from_utf8_lossy(&out.stdout).contains("got=new value"));
        assert_eq!(
            std::fs::read_to_string(dir.join("sub.env")).unwrap(),
            "X=y\n"
        );
        // A failing step reports its label.
        let out = run_bash(
            &format!(
                "{{\n{PRELUDE}\nml_step 'Étape test'\nfalse\necho unreachable\n}} < /dev/null\n"
            ),
            &dir,
        );
        assert!(!out.status.success());
        assert!(!String::from_utf8_lossy(&out.stdout).contains("unreachable"));
        assert!(String::from_utf8_lossy(&out.stderr)
            .contains("::ml::error::Échec pendant « Étape test »"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Runs MIA's "Configuration et construction" step with a stub install.sh.
    #[test]
    fn mia_install_step_uses_its_own_image_and_a_free_port() {
        if cfg!(windows) {
            return;
        }
        let catalog = Catalog::parse(EMBEDDED).unwrap();
        let app = catalog.app("mia").unwrap();
        let target = app.targets.local.as_ref().unwrap();
        let step = &target.install[1];
        let dir = std::env::temp_dir()
            .join(format!("ml-test-{}", uuid::Uuid::new_v4()))
            .join("Recette MIA");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("install.sh"),
            "printf '%s %s %s\\n' \"$APP_IMAGE\" \"$APP_BIND\" \"$MIA_PORT\" > seen\n",
        )
        .unwrap();
        let run = |port: u16| {
            let mut values = BTreeMap::new();
            values.insert("MIA_PORT".to_string(), port.to_string());
            let dir_s = dir.to_string_lossy().into_owned();
            let plan = Plan {
                app,
                target,
                action: Action::Install,
                dir: &dir_s,
                values: &values,
                git_ref: ("branch", "main"),
                token: None,
                windows: false,
            };
            let script = format!(
                "{{\n{PRELUDE}{}cd \"$ML_WORK\"\n{}\n}} < /dev/null\n",
                context_block(&plan),
                step.run
            );
            run_bash(&script, &dir)
        };

        // A port held by another program (even one lsof cannot see) is refused.
        let busy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = busy.local_addr().unwrap().port();
        let out = run(port);
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr)
            .contains(&format!("::ml::error::Le port {port} est déjà utilisé")));
        assert!(!dir.join("seen").exists());
        drop(busy);

        // A free port: install.sh builds an image named after the folder, bound to localhost.
        let out = run(port);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("seen")).unwrap(),
            format!("myalfred-recette-mia:latest 127.0.0.1 {port}\n")
        );

        // A reinstall keeps the image recorded in .env.
        std::fs::write(dir.join(".env"), "APP_IMAGE=mia-prod:latest\n").unwrap();
        let out = run(port);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(std::fs::read_to_string(dir.join("seen"))
            .unwrap()
            .starts_with("mia-prod:latest "));
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    /// Runs myTrix's "Configuration" and "Construction et démarrage" steps with
    /// a stub ./mytrix and a stub docker, so no container is ever touched.
    #[test]
    fn mytrix_install_steps_use_their_own_project_and_a_free_port() {
        if cfg!(windows) {
            return;
        }
        let catalog = Catalog::parse(EMBEDDED).unwrap();
        let app = catalog.app("mytrix").unwrap();
        let target = app.targets.local.as_ref().unwrap();
        let (config, build_step) = (&target.install[1], &target.install[2]);
        let dir = std::env::temp_dir()
            .join(format!("ml-test-{}", uuid::Uuid::new_v4()))
            .join("Recette myTrix");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mytrix"), "printf '%s\\n' \"$*\" > seen\n").unwrap();
        // `ps.out`: containers already carrying the project name; `port.out`:
        // what `docker compose port` answers when the app container is ours.
        let docker = r#"docker() {
  case "$1" in
    ps) printf '%s\n' "$*" > docker-ps; cat ps.out 2>/dev/null || true ;;
    compose) [ "$2" = port ] && cat port.out ;;
    *) return 1 ;;
  esac
}
"#;
        let run = || {
            let values = BTreeMap::new();
            let dir_s = dir.to_string_lossy().into_owned();
            let plan = Plan {
                app,
                target,
                action: Action::Install,
                dir: &dir_s,
                values: &values,
                git_ref: ("branch", "main"),
                token: None,
                windows: false,
            };
            let script = format!(
                "{{\n{PRELUDE}{}{docker}cd \"$ML_WORK\"\n{}\ncd \"$ML_WORK\"\n{}\n}} < /dev/null\n",
                context_block(&plan),
                config.run,
                build_step.run
            );
            run_bash(&script, &dir)
        };
        let busy = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = busy.local_addr().unwrap().port();
        std::fs::write(
            dir.join(".env.example"),
            format!("APP_PORT={port}\nPOSTGRES_USER=app\n"),
        )
        .unwrap();

        // Containers of another install with the same project name: nothing is written.
        std::fs::write(dir.join("ps.out"), "0123456789ab\n").unwrap();
        let out = run();
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr).contains(
            "::ml::error::Des conteneurs Docker portent déjà le nom « myalfred-recette-mytrix »"
        ));
        assert_eq!(
            std::fs::read_to_string(dir.join("docker-ps")).unwrap(),
            "ps -aq --filter label=com.docker.compose.project=myalfred-recette-mytrix\n"
        );
        assert!(!dir.join(".env").exists());
        std::fs::remove_file(dir.join("ps.out")).unwrap();

        // A port held by another program (the development myTrix…) is refused before
        // any build; the .env already names the install's own project and images.
        let out = run();
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr)
            .contains(&format!("::ml::error::Le port {port} est déjà utilisé")));
        assert!(!dir.join("seen").exists());
        let env = std::fs::read_to_string(dir.join(".env")).unwrap();
        assert!(
            env.contains("COMPOSE_PROJECT_NAME='myalfred-recette-mytrix'\n"),
            "{env}"
        );
        assert!(
            env.contains("APP_VERSION='myalfred-recette-mytrix'\n"),
            "{env}"
        );
        assert!(!env.contains("APP_VERSION='dev'"), "{env}");

        // The port is held by this install's own app (a reinstall): it goes on, and
        // its own containers do not block it.
        std::fs::write(dir.join("port.out"), format!("127.0.0.1:{port}\n")).unwrap();
        std::fs::write(dir.join("ps.out"), "0123456789ab\n").unwrap();
        let out = run();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("seen")).unwrap(),
            "installer\n"
        );
        drop(busy);

        // A free port.
        std::fs::remove_file(dir.join("port.out")).unwrap();
        std::fs::remove_file(dir.join("seen")).unwrap();
        let out = run();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("seen")).unwrap(),
            "installer\n"
        );
        let _ = std::fs::remove_dir_all(dir.parent().unwrap());
    }

    /// Runs MIA's "Sauvegarde" step with a stub docker: the public repository
    /// has no backup.sh, so the catalog carries the backup itself.
    #[test]
    fn mia_backup_step_dumps_archives_and_keeps_three() {
        if cfg!(windows) {
            return;
        }
        let catalog = Catalog::parse(EMBEDDED).unwrap();
        let app = catalog.app("mia").unwrap();
        let target = app.targets.local.as_ref().unwrap();
        let step = &target.update[0];
        assert_eq!(step.label, "Sauvegarde");
        let base = std::env::temp_dir().join(format!("ml-test-{}", uuid::Uuid::new_v4()));
        let dir = base.join("Recette MIA");
        let backups = base.join("home/backups/Recette MIA");
        std::fs::create_dir_all(&dir).unwrap();
        for old in [
            "20250101_000000",
            "20250102_000000",
            "20250103_000000",
            "20250104_000000",
            "notes",
        ] {
            std::fs::create_dir_all(backups.join(old)).unwrap();
        }
        // `log-refused`: the engine refuses `--log-driver none`.
        let docker = r#"docker() {
  case "$1" in
    compose) case "$2" in
        exec) cat dump.out ;;
        ps) printf 'cid-app\n' ;;
      esac ;;
    run) printf '%s\n' "$*" >> runs
      case " $* " in *" --log-driver none "*) [ ! -f log-refused ] || return 1 ;; esac
      printf 'archive' ;;
  esac
}
"#;
        let run = || {
            let values = BTreeMap::new();
            let dir_s = dir.to_string_lossy().into_owned();
            let plan = Plan {
                action: Action::Update,
                dir: &dir_s,
                ..plan_like(app, target, &values)
            };
            let script = format!(
                "{{\n{PRELUDE}{}HOME={}\n{docker}cd \"$ML_WORK\"\n{}\n}} < /dev/null\n",
                context_block(&plan).replace(COMMIT_SLOT, "0123456789abcdef"),
                sh_quote(&base.join("home").to_string_lossy()),
                step.run
            );
            run_bash(&script, &dir)
        };
        let listing = || {
            let mut l: Vec<String> = std::fs::read_dir(&backups)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            l.sort();
            l
        };

        // A truncated dump stops the update and leaves no half-written backup.
        std::fs::write(
            dir.join("dump.out"),
            "-- MySQL dump\nCREATE TABLE t (id int);\n",
        )
        .unwrap();
        let out = run();
        assert!(!out.status.success());
        assert!(String::from_utf8_lossy(&out.stderr)
            .contains("::ml::error::Sauvegarde de la base impossible"));
        assert_eq!(listing().len(), 5);

        // A complete dump: database, files and commit saved; three backups kept.
        std::fs::write(
            dir.join("dump.out"),
            "-- MySQL dump\nCREATE TABLE t (id int);\n-- Dump completed on 2026-10-26 10:00:00\n",
        )
        .unwrap();
        std::fs::write(dir.join("log-refused"), "").unwrap();
        let out = run();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let l = listing();
        assert_eq!(l.len(), 4, "{l:?}");
        assert_eq!(l[..2], ["20250103_000000", "20250104_000000"]);
        assert_eq!(l[3], "notes");
        let new = backups.join(&l[2]);
        assert_eq!(
            std::fs::read_to_string(new.join("git_sha.txt")).unwrap(),
            "0123456789abcdef\n"
        );
        assert_eq!(
            std::fs::read_to_string(new.join("data.tar.gz")).unwrap(),
            "archive"
        );
        let sql = std::process::Command::new("gzip")
            .arg("-dc")
            .arg(new.join("db.sql.gz"))
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&sql.stdout)
            .ends_with("-- Dump completed on 2026-10-26 10:00:00\n"));
        // The archive is retried with the default log driver, from the app container.
        let runs = std::fs::read_to_string(dir.join("runs")).unwrap();
        assert_eq!(runs.lines().count(), 2, "{runs}");
        assert!(runs
            .lines()
            .all(|r| r.contains("--volumes-from cid-app alpine:3.22")));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn default_stop_lets_databases_shut_down() {
        let catalog = Catalog::parse(EMBEDDED).unwrap();
        let app = catalog.app("mia").unwrap();
        let target = app.targets.local.as_ref().unwrap();
        let values = BTreeMap::new();
        for (action, cmd) in [
            (Action::Stop, "ml_compose stop --timeout 60\n"),
            (Action::Restart, "ml_compose stop --timeout 60\n"),
            (
                Action::Remove,
                "ml_compose down --remove-orphans --timeout 60\n",
            ),
        ] {
            let phases = build_phases(&Plan {
                app,
                target,
                action,
                dir: "~/MyAlfred/mia",
                values: &values,
                git_ref: ("branch", "main"),
                token: None,
                windows: false,
            });
            assert!(scripts(&phases).concat().contains(cmd), "{action:?}");
        }
    }

    /// MIA's update backs up with the old code, then fetches, then builds.
    #[test]
    fn update_is_split_around_the_code_fetch() {
        let catalog = Catalog::parse(EMBEDDED).unwrap();
        let app = catalog.app("mia").unwrap();
        let target = app.targets.local.as_ref().unwrap();
        let values = BTreeMap::new();
        let phases = build_phases(&Plan {
            app,
            target,
            action: Action::Update,
            dir: "/x/mia",
            values: &values,
            git_ref: ("branch", "main"),
            token: None,
            windows: false,
        });
        assert_eq!(phases.len(), 3);
        match (&phases[0], &phases[1], &phases[2]) {
            (Phase::Script(before), Phase::Sync { label }, Phase::Script(after)) => {
                assert!(before.contains("mysqldump") && !before.contains("ml_report\n"));
                assert_eq!(label, "Récupération de la nouvelle version");
                assert!(after.contains("ml_compose build") && after.contains("ml_report\n"));
            }
            other => panic!("phases inattendues : {other:?}"),
        }
        // Start / stop never fetch the code.
        let start = build_phases(&Plan {
            action: Action::Start,
            ..plan_like(app, target, &values)
        });
        assert_eq!(start.len(), 1);
    }

    fn plan_like<'a>(
        app: &'a AppDef,
        target: &'a TargetDef,
        values: &'a BTreeMap<String, String>,
    ) -> Plan<'a> {
        Plan {
            app,
            target,
            action: Action::Install,
            dir: "/x/app",
            values,
            git_ref: ("branch", "main"),
            token: None,
            windows: false,
        }
    }

    /// Without git on the machine, the apps' scripts still get the commit.
    #[test]
    fn git_is_answered_when_absent() {
        if cfg!(windows) {
            return;
        }
        let bin = std::env::temp_dir().join(format!("ml-nogit-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&bin).unwrap();
        std::os::unix::fs::symlink(bash(), bin.join("bash")).unwrap();
        let script = format!(
            "{{\n{PRELUDE}\nML_COMMIT=0123456789abcdef; export ML_COMMIT\n\
             git rev-parse HEAD\nbash -c 'git rev-parse --short HEAD'\n\
             git pull || echo refused\n}} < /dev/null\n"
        );
        let out = std::process::Command::new(bin.join("bash"))
            .args(["-c", &script])
            .env("PATH", &bin)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(stdout, "0123456789abcdef\n0123456\nrefused\n");
        let _ = std::fs::remove_dir_all(&bin);
    }

    #[test]
    fn host_check_reports_disk_space() {
        if cfg!(windows) {
            return;
        }
        let script = host_check_script("/nonexistent/deep/MyAlfred");
        let out = std::process::Command::new(bash())
            .args(["-c", &script])
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&out.stdout);
        let free = stdout
            .lines()
            .find_map(|l| l.strip_prefix("::ml::result::diskfree="))
            .expect("diskfree");
        assert!(free.parse::<u64>().unwrap() > 0, "{stdout}");
        assert!(script.contains("{{.MemTotal}}"));
    }

    #[test]
    fn labels_match_steps() {
        let catalog = Catalog::parse(EMBEDDED).unwrap();
        let app = catalog.app("mytrix").unwrap();
        let target = app.targets.local.as_ref().unwrap();
        let labels = step_labels(target, Action::Update);
        assert_eq!(labels.len(), 1 + target.update.len());
        let start = step_labels(target, Action::Start);
        assert_eq!(start.last().unwrap(), "Démarrage");
    }
}
