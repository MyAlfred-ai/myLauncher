import type { FieldDef, Installation, LatestState, RuntimeStatus } from "./types";

const rtf = new Intl.RelativeTimeFormat("fr", { numeric: "auto" });

/** "il y a 3 heures", "hier"… */
export function relativeTime(iso: string | null | undefined, now = Date.now()): string {
  if (!iso) return "jamais";
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "";
  const s = Math.round((t - now) / 1000);
  const abs = Math.abs(s);
  if (abs < 45) return "à l'instant";
  if (abs < 3600) return rtf.format(Math.round(s / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(s / 3600), "hour");
  if (abs < 86400 * 30) return rtf.format(Math.round(s / 86400), "day");
  return new Date(t).toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });
}

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "";
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return "";
  return new Date(t).toLocaleString("fr-FR", { dateStyle: "medium", timeStyle: "short" });
}

/** Commit hashes are shown short; numeric versions with a "v". */
export function displayVersion(v: string | null | undefined): string {
  if (!v) return "—";
  const t = v.trim();
  // A version has dots (1.5.0.4) or is short (v3); anything else is a commit hash.
  const numeric = /^v?(\d+(\.\d+)+([-+].*)?|\d{1,4})$/.test(t);
  if (numeric) return t.startsWith("v") ? t : `v${t}`;
  return t.slice(0, 7);
}

export function isActive(field: FieldDef, values: Record<string, string>): boolean {
  if (!field.when) return true;
  return field.when.in.includes(values[field.when.key] ?? "");
}

export function initialValues(fields: FieldDef[], previous: Record<string, string> = {}): Record<string, string> {
  const v: Record<string, string> = {};
  for (const f of fields) v[f.key] = previous[f.key] ?? f.default ?? "";
  return v;
}

const emailRe = /^[^@\s]+@[^@\s]+\.[^@\s]+$/;

/** Same rules as the backend, to show errors before submitting. */
export function validateField(f: FieldDef, values: Record<string, string>): string | null {
  if (!isActive(f, values)) return null;
  // String(): an input bound to a number would otherwise crash the whole form.
  const raw = String(values[f.key] ?? "").trim();
  const v = raw || f.default || "";
  if (!v) return f.required ? "Obligatoire" : null;
  if (/[\r\n]/.test(v)) return "Une seule ligne";
  if (f.minLength && [...v].length < f.minLength) return `Au moins ${f.minLength} caractères`;
  if (f.kind === "email" && !emailRe.test(v)) return "Adresse e-mail invalide";
  if (f.kind === "url" && !/^https?:\/\//.test(v)) return "Doit commencer par https://";
  if (f.kind === "number" && !/^\d+$/.test(v)) return "Nombre attendu";
  return null;
}

export type AppStatus = "available" | "installing" | "installed" | "update" | "failed";

export function appStatus(installs: Installation[]): AppStatus {
  if (installs.some((i) => i.busyOp || i.state === "installing")) return "installing";
  if (installs.some((i) => i.updateAvailable)) return "update";
  if (installs.some((i) => i.state === "ready")) return "installed";
  if (installs.some((i) => i.state === "failed")) return "failed";
  return "available";
}

export function latestLabel(l: LatestState | undefined): string {
  if (!l) return "Version inconnue";
  if (l.error) return "Version indisponible";
  return displayVersion(l.latest?.version);
}

/** Label and colour of the containers' state, the same on every screen. */
export function runtimeInfo(s: RuntimeStatus | null | undefined): { cls: string; text: string } {
  switch (s?.state) {
    case "running":
      return { cls: "success", text: "En marche" };
    case "partial":
      return { cls: "warning", text: "Démarrage partiel" };
    case "stopped":
      return { cls: "", text: "Arrêtée" };
    case "absent":
      return { cls: "", text: "Conteneurs absents" };
    case "unknown":
      return { cls: "warning", text: "État inconnu" };
    default:
      return { cls: "", text: "—" };
  }
}

/** True when the next useful action is "Démarrer" rather than "Arrêter". */
export function isStopped(s: RuntimeStatus | null | undefined): boolean {
  return s?.state === "stopped" || s?.state === "absent";
}

/** "localhost:8081", "mia.entreprise.fr": where the application answers. */
export function addressLabel(url: string): string {
  try {
    const u = new URL(url);
    return u.host + u.pathname.replace(/\/$/, "");
  } catch {
    return url;
  }
}

export const actionLabels: Record<string, string> = {
  install: "Installation",
  update: "Mise à jour",
  start: "Démarrage",
  stop: "Arrêt",
  restart: "Redémarrage",
  remove: "Désinstallation",
};
