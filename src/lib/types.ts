// Mirrors the Rust types serialized by src-tauri (camelCase).

export type FieldKind = "text" | "email" | "url" | "number" | "secret" | "password" | "select";

export interface FieldDef {
  key: string;
  label: string;
  kind: FieldKind;
  required?: boolean;
  default?: string | null;
  placeholder?: string | null;
  help?: string | null;
  generate?: string | null;
  options?: { value: string; label: string }[];
  minLength?: number | null;
  when?: { key: string; in: string[] } | null;
}

export interface Step {
  label: string;
  run: string;
}

export interface TargetDef {
  notes: string[];
  experimental: boolean;
  workdir?: string | null;
  url: string;
  duration?: string | null;
  fields: FieldDef[];
  install: Step[];
  update: Step[];
  start: Step[];
  stop: Step[];
}

export interface AppDef {
  id: string;
  name: string;
  tagline: string;
  description: string;
  icon: string;
  color: string;
  features: string[];
  repo: { owner: string; name: string; branch: string };
  version: { source: "releases" | "tags" | "file" | "commit"; path?: string | null };
  changelog?: { source: "file" | "releases" | "commits"; path?: string | null } | null;
  requirements: string[];
  targets: { local?: TargetDef | null; server?: TargetDef | null };
}

export interface Catalog {
  schema: number;
  updated: string;
  apps: AppDef[];
}

export interface Latest {
  version: string;
  refKind: "tag" | "branch";
  refName: string;
  commit?: string | null;
  date?: string | null;
  summary?: string | null;
}

export interface LatestState {
  latest: Latest | null;
  error: string | null;
  checkedAt: string;
}

export type InstallState = "installing" | "ready" | "failed";

export interface Installation {
  id: string;
  appId: string;
  target: string;
  dir: string;
  values: Record<string, string>;
  version?: string | null;
  commit?: string | null;
  url?: string | null;
  state: InstallState;
  installedAt: string;
  updatedAt?: string | null;
  autoUpdate: boolean;
  targetLabel: string;
  updateAvailable: boolean;
  busyOp: string | null;
}

export interface Server {
  id: string;
  name: string;
  host: string;
  port: number;
  user: string;
  identityFile?: string | null;
  baseDir: string;
}

export interface Settings {
  theme: "system" | "light" | "dark";
  installDir: string;
  autoCheck: boolean;
  checkIntervalHours: number;
  notify: boolean;
  catalogUrl: string;
}

export type OpStatus = "running" | "success" | "failed" | "cancelled";
export type OpAction = "install" | "update" | "start" | "stop" | "restart" | "remove";

export interface OpInfo {
  id: string;
  installationId: string;
  appId: string;
  action: OpAction;
  targetLabel: string;
  status: OpStatus;
  steps: string[];
  currentStep: number;
  error: string | null;
  warnings: string[];
  startedAt: string;
  endedAt: string | null;
  logFile: string | null;
}

export interface LogLine {
  opId: string;
  seq: number;
  kind: "out" | "err" | "step" | "warn" | "error";
  text: string;
}

export interface Overview {
  catalog: Catalog;
  catalogSource: "embedded" | "cache" | "remote";
  catalogFetchedAt: string | null;
  latest: Record<string, LatestState>;
  installations: Installation[];
  servers: Server[];
  ops: OpInfo[];
  settings: Settings;
  hasToken: boolean;
  lastCheck: string | null;
  platform: string;
  launcherVersion: string;
}

export interface TokenStatus {
  present: boolean;
  login: string | null;
  error: string | null;
}

export interface HostCheck {
  reachable: boolean;
  os: string;
  git: string;
  docker: string;
  compose: string;
  daemon: boolean;
  error: string | null;
}

export interface ServiceStatus {
  name: string;
  state: string;
  health: string;
}

export interface RuntimeStatus {
  state: "running" | "partial" | "stopped" | "absent" | "unknown";
  services: ServiceStatus[];
  error: string | null;
}
