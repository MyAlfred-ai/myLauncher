// Demo backend used when the interface runs in a plain browser (npm run dev),
// so the design can be reviewed without the desktop app.
import catalogJson from "../../catalog/apps.json";
import type { Catalog, Installation, LogLine, OpInfo, Overview, Settings } from "./types";

type Listener = (payload: unknown) => void;
const listeners = new Map<string, Set<Listener>>();

function emit(event: string, payload: unknown) {
  listeners.get(event)?.forEach((cb) => cb(payload));
}

export function mockListen<T>(event: string, cb: (payload: T) => void): () => void {
  if (!listeners.has(event)) listeners.set(event, new Set());
  const set = listeners.get(event)!;
  set.add(cb as Listener);
  return () => set.delete(cb as Listener);
}

const ago = (h: number) => new Date(Date.now() - h * 3600_000).toISOString();

const catalog = catalogJson as unknown as Catalog;
let settings: Settings = {
  theme: "system",
  installDir: "/Users/demo/MyAlfred",
  autoCheck: true,
  checkIntervalHours: 6,
  notify: true,
  catalogUrl: "https://api.github.com/repos/MyAlfred-ai/myLauncher/contents/catalog/apps.json?ref=main",
};
let installations: Installation[] = [
  {
    id: "inst-mia",
    appId: "mia",
    dir: "/Users/demo/MyAlfred/mia",
    values: { MIA_PORT: "8080", MIA_ADMIN_EMAIL: "axel@myalfred.ai" },
    version: "1.5.0.2",
    commit: "a1b2c3d",
    url: "http://localhost:8080",
    state: "ready",
    installedAt: ago(240),
    updatedAt: ago(72),
    autoUpdate: false,
    updateAvailable: true,
    busyOp: null,
  },
  {
    id: "inst-brain",
    appId: "mybrain",
    dir: "/Users/demo/MyAlfred/mybrain",
    values: {},
    version: "9f8e7d6",
    commit: "9f8e7d6c",
    url: "http://localhost:5173",
    state: "ready",
    installedAt: ago(500),
    updatedAt: ago(20),
    autoUpdate: true,
    updateAvailable: false,
    busyOp: null,
  },
];
// Containers' state of the demo installations, changed by start/stop.
const running: Record<string, boolean> = { "inst-mia": true, "inst-brain": false };
const ops: OpInfo[] = [];
const logs = new Map<string, LogLine[]>();
let seq = 0;
let hasToken = true;

const latest = () => ({
  mia: { latest: { version: "1.5.0.4", refKind: "branch", refName: "main", commit: "f00ba44", date: ago(5), summary: "feat: recherche web Mistral" }, error: null, checkedAt: ago(0.2) },
  mybrief: { latest: { version: "1.3.0.90", refKind: "branch", refName: "main", commit: "2c68389", date: ago(30), summary: "fix: export PDF" }, error: null, checkedAt: ago(0.2) },
  mytrix: { latest: { version: "4d5e6f7", refKind: "branch", refName: "main", commit: "4d5e6f7a", date: ago(50), summary: "feat: agent en écriture" }, error: null, checkedAt: ago(0.2) },
  mybrain: { latest: { version: "9f8e7d6", refKind: "branch", refName: "main", commit: "9f8e7d6c", date: ago(20), summary: "feat: tableaux de bord" }, error: null, checkedAt: ago(0.2) },
});

function overview(): Overview {
  return {
    catalog,
    catalogSource: "remote",
    catalogFetchedAt: ago(0.2),
    latest: latest() as Overview["latest"],
    installations: installations.map((i) => ({
      ...i,
      busyOp: ops.find((o) => o.installationId === i.id && o.status === "running")?.id ?? null,
    })),
    ops: [...ops].sort((a, b) => b.startedAt.localeCompare(a.startedAt)),
    settings,
    hasToken,
    lastCheck: ago(0.2),
    platform: "macos",
    launcherVersion: "0.1.0",
  };
}

function simulate(installationId: string, appId: string, action: OpInfo["action"]) {
  const app = catalog.apps.find((a) => a.id === appId)!;
  const tdef = app.targets.local!;
  const steps = ["Vérification des prérequis", ...(action === "update" ? tdef.update : action === "install" ? tdef.install : [{ label: action === "stop" ? "Arrêt" : action === "restart" ? "Redémarrage" : "Démarrage", run: "" }]).map((s) => s.label)];
  const op: OpInfo = {
    id: `op-${Math.random().toString(36).slice(2, 9)}`,
    installationId,
    appId,
    action,
    status: "running",
    steps,
    currentStep: 0,
    error: null,
    warnings: [],
    startedAt: new Date().toISOString(),
    endedAt: null,
    logFile: null,
  };
  ops.push(op);
  logs.set(op.id, []);
  emit("op-state", { ...op });
  emit("overview-changed", null);
  const sample = [
    "docker compose version v2.39.4",
    "Clonage de https://github.com/MyAlfred-ai/…",
    "remote: Enumerating objects: 4120, done.",
    "Receiving objects: 100% (4120/4120), 21.3 MiB | 18.2 MiB/s, done.",
    "Code à jour : 2c68389",
    "#7 [app 3/9] RUN composer install --no-dev --optimize-autoloader",
    "#7 DONE 12.4s",
    "Container mia-db-1  Healthy",
    "Container mia-app-1  Started",
  ];
  let i = 0;
  const timer = setInterval(() => {
    const step = Math.min(steps.length - 1, Math.floor(i / 3));
    if (step !== op.currentStep || i === 0) {
      op.currentStep = step;
      const line: LogLine = { opId: op.id, seq: ++seq, kind: "step", text: steps[step] };
      logs.get(op.id)!.push(line);
      emit("op-log", [line]);
      emit("op-state", { ...op });
    }
    const line: LogLine = { opId: op.id, seq: ++seq, kind: "out", text: sample[i % sample.length] };
    logs.get(op.id)!.push(line);
    emit("op-log", [line]);
    i++;
    if (i > steps.length * 3) {
      clearInterval(timer);
      op.status = "success";
      op.currentStep = steps.length;
      op.endedAt = new Date().toISOString();
      const target = installations.find((x) => x.id === installationId);
      if (target) {
        if (action === "remove") installations = installations.filter((x) => x.id !== installationId);
        else {
          target.state = "ready";
          running[installationId] = action !== "stop";
          if (action === "update" || action === "install") {
            target.version = latest()[appId as "mia"].latest.version;
            target.updateAvailable = false;
          }
        }
      }
      emit("op-state", { ...op });
      emit("overview-changed", null);
    }
  }, 450);
  return op.id;
}

export async function mockInvoke<T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> {
  await new Promise((r) => setTimeout(r, 120));
  const a = args as Record<string, any>;
  switch (cmd) {
    case "get_overview":
    case "check_updates":
      return overview() as T;
    case "get_changelog":
      return `## Nouveautés\n\n### Recherche web native\nChaque fournisseur utilise désormais son propre outil de recherche.\n\n### Assistants\n- Appel d'un assistant avec **@nom** depuis la saisie\n- Jusqu'à 3 bases de connaissances par conversation\n\n### Corrections\n- Export PDF des conversations longues\n` as T;
    case "save_settings":
      settings = a.settings;
      return settings as T;
    case "github_status":
      return { present: hasToken, login: hasToken ? "axel-myalfred" : null, error: null } as T;
    case "set_github_token":
      hasToken = true;
      return { present: true, login: "axel-myalfred", error: null } as T;
    case "clear_github_token":
      hasToken = false;
      return undefined as T;
    case "check_host":
      return {
        reachable: true,
        os: "Darwin arm64",
        git: "git version 2.50.1",
        docker: "Docker version 28.5.1",
        compose: "2.39.4",
        daemon: true,
        error: null,
      } as T;
    case "default_install_dir":
      return `${settings.installDir}/${a.appId}` as T;
    case "install_app": {
      const req = a.req;
      const id = `inst-${Math.random().toString(36).slice(2, 8)}`;
      installations.push({
        id,
        appId: req.appId,
        dir: req.dir,
        values: {},
        version: null,
        state: "installing",
        installedAt: new Date().toISOString(),
        autoUpdate: false,
        updateAvailable: false,
        busyOp: null,
        url: "http://localhost:3000",
      });
      return simulate(id, req.appId, "install") as T;
    }
    case "run_action": {
      const inst = installations.find((i) => i.id === a.installationId)!;
      return simulate(inst.id, inst.appId, a.action) as T;
    }
    case "forget_installation":
      installations = installations.filter((i) => i.id !== a.installationId);
      emit("overview-changed", null);
      return undefined as T;
    case "set_auto_update": {
      const inst = installations.find((i) => i.id === a.installationId);
      if (inst) inst.autoUpdate = a.enabled;
      emit("overview-changed", null);
      return undefined as T;
    }
    case "installation_status": {
      const on = running[a.installationId] ?? true;
      const state = on ? "running" : "exited";
      return {
        state: on ? "running" : "stopped",
        services: [
          { name: "app", state, health: on ? "healthy" : "" },
          { name: "db", state, health: on ? "healthy" : "" },
          { name: "worker", state, health: "" },
        ],
        error: null,
      } as T;
    }
    case "installation_logs":
      return "app-1  | [2026-10-18 10:02:11] Listening on :80\nworker-1  | job queue ready\n" as T;
    case "get_op_log":
      return (logs.get(a.opId) ?? []) as T;
    case "cancel_op":
      return true as T;
    case "generate_secret":
      return "Kp7mx-Qa2Wd-Rt9Ny-Hb3Lz" as T;
    default:
      throw new Error(`mock: commande inconnue ${cmd}`);
  }
}
