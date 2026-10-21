import type {
  HostCheck,
  LauncherUpdate,
  LogLine,
  OpInfo,
  Overview,
  RuntimeStatus,
  Settings,
  TokenStatus,
} from "./types";

/** True inside the desktop app; false in a plain browser (demo data). */
export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri) {
    const { mockInvoke } = await import("./mock");
    return mockInvoke<T>(cmd, args);
  }
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export async function on<T>(event: string, cb: (payload: T) => void): Promise<() => void> {
  if (!isTauri) {
    const { mockListen } = await import("./mock");
    return mockListen<T>(event, cb);
  }
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(event, (e) => cb(e.payload));
}

export const api = {
  overview: () => call<Overview>("get_overview"),
  checkUpdates: () => call<Overview>("check_updates"),
  changelog: (appId: string) => call<string>("get_changelog", { appId }),
  checkLauncherUpdate: () => call<LauncherUpdate>("check_launcher_update"),
  installLauncherUpdate: () => call<void>("install_launcher_update"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  githubStatus: () => call<TokenStatus>("github_status"),
  setGithubToken: (token: string) => call<TokenStatus>("set_github_token", { token }),
  clearGithubToken: () => call<void>("clear_github_token"),
  checkHost: () => call<HostCheck>("check_host"),
  dockerStart: () => call<HostCheck>("docker_start"),
  dockerInstall: () => call<HostCheck>("docker_install"),
  dockerInstallCancel: () => call<void>("docker_install_cancel"),
  defaultInstallDir: (appId: string) => call<string>("default_install_dir", { appId }),
  install: (req: { appId: string; dir: string; values: Record<string, string> }) =>
    call<string>("install_app", { req }),
  runAction: (installationId: string, action: "update" | "start" | "stop" | "restart" | "remove") =>
    call<string>("run_action", { installationId, action }),
  forget: (installationId: string) => call<void>("forget_installation", { installationId }),
  setAutoUpdate: (installationId: string, enabled: boolean) =>
    call<void>("set_auto_update", { installationId, enabled }),
  status: (installationId: string) => call<RuntimeStatus>("installation_status", { installationId }),
  logs: (installationId: string) => call<string>("installation_logs", { installationId }),
  opLog: (opId: string) => call<LogLine[]>("get_op_log", { opId }),
  cancelOp: (opId: string) => call<boolean>("cancel_op", { opId }),
  generateSecret: (kind: string) => call<string>("generate_secret", { kind }),
};

export async function openUrl(url: string) {
  if (!isTauri) {
    window.open(url, "_blank", "noopener");
    return;
  }
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } catch (e) {
    // Imported here: launcher.svelte imports this module.
    const { launcher } = await import("./launcher.svelte");
    launcher.toast(`Impossible d'ouvrir ${url} : ${e}`, "error");
  }
}

export async function revealDir(path: string) {
  if (!isTauri) return;
  const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
  await revealItemInDir(path);
}

export async function pickDir(title: string): Promise<string | null> {
  if (!isTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const r = await open({ title, multiple: false, directory: true });
  return typeof r === "string" ? r : null;
}

export type OpEvents = { state: (op: OpInfo) => void; log: (lines: LogLine[]) => void };
