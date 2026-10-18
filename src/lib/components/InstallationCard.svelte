<script lang="ts">
  import { onMount } from "svelte";
  import { launcher } from "../launcher.svelte";
  import { api, isTauri, openUrl, revealDir } from "../api";
  import { displayVersion, relativeTime } from "../format";
  import type { Installation, RuntimeStatus } from "../types";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  let { inst }: { inst: Installation } = $props();

  let status = $state<RuntimeStatus | null>(null);
  let loadingStatus = $state(false);
  let menuOpen = $state(false);
  let confirm = $state<null | "remove" | "forget">(null);
  let logs = $state<string | null>(null);

  const latest = $derived(launcher.latest(inst.appId)?.latest);
  const op = $derived(inst.busyOp ? launcher.ops[inst.busyOp] : undefined);
  const app = $derived(launcher.app(inst.appId));
  const progress = $derived(op ? Math.round((op.currentStep / Math.max(1, op.steps.length)) * 100) : 0);

  async function refreshStatus() {
    if (inst.state !== "ready") return;
    loadingStatus = true;
    try {
      status = await api.status(inst.id);
    } catch (e) {
      status = { state: "unknown", services: [], error: String(e) };
    } finally {
      loadingStatus = false;
    }
  }

  onMount(refreshStatus);

  // Refresh the container state once an operation on this install ends.
  let wasBusy = false;
  $effect(() => {
    const busy = !!inst.busyOp;
    if (wasBusy && !busy) refreshStatus();
    wasBusy = busy;
  });

  const stateInfo = $derived.by(() => {
    if (inst.state === "installing" && !op) return { cls: "danger", text: "Installation interrompue" };
    if (inst.state === "failed") return { cls: "danger", text: "Installation échouée" };
    if (op) return { cls: "brand", text: "En cours" };
    if (loadingStatus && !status) return { cls: "", text: "…" };
    switch (status?.state) {
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
  });

  async function showLogs() {
    menuOpen = false;
    logs = "Chargement…";
    try {
      logs = (await api.logs(inst.id)) || "Aucun journal.";
    } catch (e) {
      logs = String(e);
    }
  }

  async function toggleAuto() {
    try {
      await api.setAutoUpdate(inst.id, !inst.autoUpdate);
    } catch (e) {
      launcher.toast(String(e), "error");
    }
  }

  async function doConfirm() {
    const what = confirm;
    confirm = null;
    if (what === "remove") await launcher.run(inst.id, "remove");
    else if (what === "forget") {
      try {
        await api.forget(inst.id);
        launcher.toast("Retirée du launcher", "success");
      } catch (e) {
        launcher.toast(String(e), "error");
      }
    }
  }

  function retry() {
    launcher.installing = { appId: inst.appId, target: inst.target };
  }
</script>

<svelte:window onclick={() => (menuOpen = false)} />

<div class="inst card" class:busy={!!op}>
  <div class="row top">
    <div class="where">
      <div class="where-icon"><Icon name={inst.target === "local" ? "laptop" : "server"} size={20} /></div>
      <div class="where-text">
        <strong>{inst.targetLabel}</strong>
        <span class="muted mono truncate selectable" title={inst.dir}>{inst.dir}</span>
      </div>
    </div>
    <span class="pill {stateInfo.cls}">
      {#if op}<Icon name="loader" size={12} class="spin" />{:else}<span class="dot"></span>{/if}
      {stateInfo.text}
    </span>
  </div>

  {#if op}
    <button class="progress" onclick={() => launcher.go({ name: "op", id: op.id })}>
      <div class="bar"><div style:width="{progress}%"></div></div>
      <span class="truncate">{op.steps[op.currentStep] ?? "Préparation"}</span>
      <span class="muted">Voir le détail</span>
    </button>
  {:else if inst.state === "ready"}
    <div class="row version">
      <span class="mono">{displayVersion(inst.version)}</span>
      {#if inst.updateAvailable && latest}
        <Icon name="chevron" size={14} />
        <span class="pill warning">{displayVersion(latest.version)} disponible</span>
      {:else}
        <span class="muted">· à jour {relativeTime(inst.updatedAt ?? inst.installedAt)}</span>
      {/if}
    </div>
    {#if status?.error}
      <p class="status-err">{status.error}</p>
    {/if}
  {:else}
    <div class="callout danger">
      <Icon name="alert" size={18} />
      <span>L'installation ne s'est pas terminée. Corrigez la cause indiquée dans l'activité, puis relancez-la : le code déjà téléchargé est réutilisé.</span>
    </div>
  {/if}

  <div class="row actions">
    {#if inst.state === "ready"}
      {#if inst.updateAvailable}
        <button class="btn brand sm" disabled={!!op} onclick={() => launcher.run(inst.id, "update")}>
          <Icon name="up" size={15} /> Mettre à jour
        </button>
      {/if}
      {#if inst.url}
        <button class="btn sm" class:primary={!inst.updateAvailable} onclick={() => openUrl(inst.url!)}>
          <Icon name="external" size={15} /> Ouvrir
        </button>
      {/if}
      {#if status?.state === "stopped" || status?.state === "absent"}
        <button class="btn sm" disabled={!!op} onclick={() => launcher.run(inst.id, "start", false)}>
          <Icon name="play" size={13} /> Démarrer
        </button>
      {:else}
        <button class="btn sm" disabled={!!op} onclick={() => launcher.run(inst.id, "stop", false)}>
          <Icon name="stop" size={14} /> Arrêter
        </button>
      {/if}
    {:else}
      <button class="btn primary sm" disabled={!!op} onclick={retry}><Icon name="refresh" size={15} /> Reprendre l'installation</button>
    {/if}

    <div class="menu-wrap">
      <button
        class="btn sm icon ghost"
        aria-label="Plus d'actions"
        aria-expanded={menuOpen}
        onclick={(e) => {
          e.stopPropagation();
          menuOpen = !menuOpen;
        }}><Icon name="more" /></button
      >
      {#if menuOpen}
        <div class="menu" role="menu">
          {#if inst.state === "ready"}
            <button role="menuitem" disabled={!!op} onclick={() => launcher.run(inst.id, "restart", false)}><Icon name="restart" size={16} /> Redémarrer</button>
            <button role="menuitem" onclick={showLogs}><Icon name="terminal" size={16} /> Journaux de l'application</button>
            <button role="menuitem" onclick={refreshStatus}><Icon name="refresh" size={16} /> Actualiser l'état</button>
          {/if}
          {#if inst.target === "local" && isTauri}
            <button role="menuitem" onclick={() => revealDir(inst.dir)}><Icon name="folder" size={16} /> Ouvrir le dossier</button>
          {/if}
          {#if inst.state === "ready"}
            <label class="menu-toggle">
              <Icon name="sparkles" size={16} /> Mises à jour automatiques
              <input type="checkbox" class="switch" checked={inst.autoUpdate} onchange={toggleAuto} />
            </label>
          {/if}
          <hr />
          {#if inst.state === "ready"}
            <button role="menuitem" class="danger" disabled={!!op} onclick={() => (confirm = "remove")}><Icon name="trash" size={16} /> Désinstaller…</button>
          {/if}
          <button role="menuitem" class="danger" disabled={!!op} onclick={() => (confirm = "forget")}><Icon name="x" size={16} /> Retirer du launcher…</button>
        </div>
      {/if}
    </div>
  </div>
</div>

{#if confirm}
  <Modal
    title={confirm === "remove" ? `Désinstaller ${app?.name} ?` : `Retirer ${app?.name} du launcher ?`}
    subtitle={inst.targetLabel}
    width={480}
    onclose={() => (confirm = null)}
  >
    {#if confirm === "remove"}
      <p>Les conteneurs de l'application sont arrêtés et supprimés.</p>
      <div class="callout success" style="margin-top:12px">
        <Icon name="shield" size={18} />
        <span>Vos données (bases, volumes Docker, fichiers .env) et le dossier <span class="mono">{inst.dir}</span> sont <strong>conservés</strong> : une réinstallation dans le même dossier les retrouve.</span>
      </div>
    {:else}
      <p>Le launcher oublie cette installation. Rien n'est modifié sur {inst.target === "local" ? "ce poste" : "le serveur"} : l'application continue de tourner si elle était démarrée.</p>
    {/if}
    {#snippet footer()}
      <button class="btn" onclick={() => (confirm = null)}>Annuler</button>
      <button class="btn danger solid" onclick={doConfirm}>{confirm === "remove" ? "Désinstaller" : "Retirer"}</button>
    {/snippet}
  </Modal>
{/if}

{#if logs !== null}
  <Modal title="Journaux de {app?.name}" subtitle="300 dernières lignes par service" width={900} onclose={() => (logs = null)}>
    <pre class="console">{logs}</pre>
    {#snippet footer()}
      <button class="btn" onclick={() => navigator.clipboard.writeText(logs ?? "")}><Icon name="copy" size={15} /> Copier</button>
      <button class="btn primary" onclick={() => (logs = null)}>Fermer</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .inst {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 16px 18px;
  }
  .inst.busy {
    border-color: color-mix(in srgb, var(--primary) 40%, var(--border));
  }
  .row {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .top {
    justify-content: space-between;
  }
  .where {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .where-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 12px;
    background: var(--surface-2);
    color: var(--text-2);
    flex: none;
  }
  .where-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .where-text span {
    font-size: 12.5px;
  }
  .version {
    font-size: 14px;
    flex-wrap: wrap;
  }
  .status-err {
    font-size: 12.5px;
    color: var(--warning);
  }
  .progress {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 6px 12px;
    padding: 0;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    font-size: 13.5px;
  }
  .progress .bar {
    grid-column: 1 / -1;
    height: 6px;
    border-radius: 99px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .progress .bar div {
    height: 100%;
    background: var(--brand-gradient);
    border-radius: inherit;
    transition: width 0.4s ease;
  }
  .actions {
    flex-wrap: wrap;
  }
  .menu-wrap {
    position: relative;
    margin-left: auto;
  }
  .menu {
    position: absolute;
    right: 0;
    bottom: calc(100% + 6px);
    z-index: 20;
    min-width: 250px;
    padding: 6px;
    border-radius: 14px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
    display: flex;
    flex-direction: column;
  }
  .menu button,
  .menu-toggle {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 10px;
    border: none;
    border-radius: 9px;
    background: none;
    font-size: 13.5px;
    text-align: left;
    cursor: pointer;
    color: var(--text);
  }
  .menu button:hover:not(:disabled),
  .menu-toggle:hover {
    background: var(--surface-2);
  }
  .menu button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .menu .danger {
    color: var(--danger);
  }
  .menu-toggle .switch {
    margin-left: auto;
    transform: scale(0.8);
  }
  .menu hr {
    border: none;
    border-top: 1px solid var(--border);
    margin: 4px 0;
  }
  .console {
    margin: 0;
    max-height: 60vh;
    overflow: auto;
    padding: 14px;
    border-radius: 12px;
    background: var(--console-bg);
    color: var(--console-text);
    font: 12px/1.55 var(--mono);
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
