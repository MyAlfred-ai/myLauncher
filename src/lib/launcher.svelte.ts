import { api, on } from "./api";
import type { AppDef, Installation, LogLine, OpInfo, Overview } from "./types";

export type View =
  | { name: "library" }
  | { name: "app"; id: string; tab?: "installs" | "about" | "news" }
  | { name: "servers" }
  | { name: "activity" }
  | { name: "op"; id: string }
  | { name: "settings" };

export interface Toast {
  id: number;
  kind: "info" | "success" | "error";
  text: string;
}

const MAX_LOG_LINES = 4000;

/** Global state of the interface. */
class Launcher {
  overview = $state<Overview | null>(null);
  loadError = $state<string | null>(null);
  view = $state<View>({ name: "library" });
  ops = $state<Record<string, OpInfo>>({});
  logs = $state.raw<Record<string, LogLine[]>>({});
  toasts = $state<Toast[]>([]);
  checking = $state(false);
  /** App whose install dialog is open. */
  installing = $state<{ appId: string; target?: string } | null>(null);

  #toastId = 0;
  #refreshing: Promise<void> | null = null;
  #again = false;

  get apps(): AppDef[] {
    return this.overview?.catalog.apps ?? [];
  }

  get updateCount(): number {
    return this.overview?.installations.filter((i) => i.updateAvailable).length ?? 0;
  }

  get runningOps(): OpInfo[] {
    return Object.values(this.ops).filter((o) => o.status === "running");
  }

  app(id: string): AppDef | undefined {
    return this.apps.find((a) => a.id === id);
  }

  installsFor(appId: string): Installation[] {
    return this.overview?.installations.filter((i) => i.appId === appId) ?? [];
  }

  latest(appId: string) {
    return this.overview?.latest[appId];
  }

  async init() {
    await this.refresh();
    await on("overview-changed", () => this.refresh());
    await on<OpInfo>("op-state", (op) => {
      this.ops[op.id] = op;
    });
    await on<LogLine[]>("op-log", (lines) => this.appendLogs(lines));
  }

  /** Coalesces bursts of refresh requests. */
  refresh(): Promise<void> {
    if (this.#refreshing) {
      this.#again = true;
      return this.#refreshing;
    }
    this.#refreshing = (async () => {
      do {
        this.#again = false;
        try {
          const o = await api.overview();
          this.overview = o;
          for (const op of o.ops) this.ops[op.id] = op;
          this.loadError = null;
          applyTheme(o.settings.theme);
        } catch (e) {
          this.loadError = String(e);
        }
      } while (this.#again);
      this.#refreshing = null;
    })();
    return this.#refreshing;
  }

  appendLogs(lines: LogLine[]) {
    if (!lines.length) return;
    const next = { ...this.logs };
    for (const l of lines) {
      const arr = next[l.opId] ? [...next[l.opId]] : [];
      // A snapshot may already contain lines that also arrive as events.
      if (arr.length && l.seq <= arr[arr.length - 1].seq) continue;
      arr.push(l);
      next[l.opId] = arr.length > MAX_LOG_LINES ? arr.slice(-MAX_LOG_LINES) : arr;
    }
    this.logs = next;
  }

  async loadLog(opId: string) {
    try {
      const snapshot = await api.opLog(opId);
      const last = snapshot.length ? snapshot[snapshot.length - 1].seq : 0;
      const live = (this.logs[opId] ?? []).filter((l) => l.seq > last);
      this.logs = { ...this.logs, [opId]: [...snapshot, ...live] };
    } catch {
      /* log may be gone after a restart */
    }
  }

  async checkUpdates() {
    if (this.checking) return;
    this.checking = true;
    try {
      this.overview = await api.checkUpdates();
      const n = this.updateCount;
      this.toast(n ? `${n} mise${n > 1 ? "s" : ""} à jour disponible${n > 1 ? "s" : ""}` : "Tout est à jour", n ? "info" : "success");
    } catch (e) {
      this.toast(String(e), "error");
    } finally {
      this.checking = false;
    }
  }

  /** Runs an action and follows it on the operation screen. */
  async run(installationId: string, action: "update" | "start" | "stop" | "restart" | "remove", follow = true) {
    try {
      const id = await api.runAction(installationId, action);
      await this.refresh();
      if (follow) this.go({ name: "op", id });
      return id;
    } catch (e) {
      this.toast(String(e), "error");
      return null;
    }
  }

  async updateAll() {
    const targets = this.overview?.installations.filter((i) => i.updateAvailable && !i.busyOp) ?? [];
    for (const i of targets) await this.run(i.id, "update", false);
    if (targets.length) this.toast(`${targets.length} mise${targets.length > 1 ? "s" : ""} à jour lancée${targets.length > 1 ? "s" : ""}`, "info");
    this.go({ name: "activity" });
  }

  go(view: View) {
    this.view = view;
    document.querySelector("main")?.scrollTo({ top: 0 });
  }

  toast(text: string, kind: Toast["kind"] = "info") {
    const id = ++this.#toastId;
    this.toasts.push({ id, kind, text: text.replace(/^Error:\s*/, "") });
    setTimeout(() => this.dismiss(id), kind === "error" ? 7000 : 3500);
  }

  dismiss(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }
}

export function applyTheme(theme: string) {
  const root = document.documentElement;
  if (theme === "light" || theme === "dark") root.dataset.theme = theme;
  else delete root.dataset.theme;
}

export const launcher = new Launcher();

const icons = import.meta.glob<string>("../assets/apps/*.png", { eager: true, import: "default" });

export function appIcon(icon: string): string {
  return icons[`../assets/apps/${icon}`] ?? "";
}
