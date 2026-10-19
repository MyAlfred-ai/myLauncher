<script lang="ts">
  import { untrack } from "svelte";
  import { appIcon, launcher } from "../launcher.svelte";
  import { actionLabels, addressLabel, appStatus, displayVersion, isStopped, runtimeInfo } from "../format";
  import { openUrl } from "../api";
  import type { AppDef } from "../types";
  import Icon from "./Icon.svelte";

  let { app, index = 0 }: { app: AppDef; index?: number } = $props();

  const installs = $derived(launcher.installsFor(app.id));
  const status = $derived(appStatus(installs));
  const latest = $derived(launcher.latest(app.id));
  const toUpdate = $derived(installs.filter((i) => i.updateAvailable && !i.busyOp));
  // The installation driven from the card.
  const primary = $derived(installs.find((i) => i.state === "ready"));
  const rt = $derived(primary ? launcher.runtime[primary.id] : undefined);
  const op = $derived(primary?.busyOp ? launcher.ops[primary.busyOp] : undefined);
  const stopped = $derived(isStopped(rt?.status));
  const led = $derived.by(() => {
    if (op) return "work";
    switch (rt?.status?.state) {
      case "running":
        return "on";
      case "partial":
      case "unknown":
        return "warn";
      default:
        return "off";
    }
  });

  $effect(() => {
    const id = primary?.id;
    if (id) untrack(() => launcher.refreshStatus(id));
  });

  // Read the containers again once an operation on this installation ends.
  let wasBusy = false;
  $effect(() => {
    const id = primary?.id;
    const busy = !!primary?.busyOp;
    if (id && wasBusy && !busy) untrack(() => launcher.refreshStatus(id, true));
    wasBusy = busy;
  });

  const statusInfo = $derived.by(() => {
    switch (status) {
      case "installing":
        return { cls: "brand", text: "En cours…" };
      case "update":
        return { cls: "warning", text: "Mise à jour disponible" };
      case "installed":
        if (installs.length > 1) return { cls: "success", text: `${installs.length} installations` };
        return rt?.status ? runtimeInfo(rt.status) : { cls: "success", text: "Installé" };
      case "failed":
        return { cls: "danger", text: "Installation échouée" };
      default:
        return app.comingSoon ? { cls: "brand", text: "Bientôt disponible" } : null;
    }
  });

  function open() {
    launcher.go({ name: "app", id: app.id });
  }

  function connect(e: MouseEvent) {
    e.stopPropagation();
    if (primary?.url) openUrl(primary.url);
  }

  function toggle(e: MouseEvent) {
    e.stopPropagation();
    if (primary) launcher.run(primary.id, stopped ? "start" : "stop", false);
  }

  async function update(e: MouseEvent) {
    e.stopPropagation();
    if (toUpdate.length === 1) await launcher.run(toUpdate[0].id, "update");
    else launcher.go({ name: "app", id: app.id, tab: "installs" });
  }
</script>

<article class="app-card" style:--app={app.color} style:--i={index}>
  <button class="hit" onclick={open} aria-label="Voir {app.name}"></button>
  <div class="visual">
    <div class="halo"></div>
    <img src={appIcon(app.icon)} alt="" width="132" height="132" />
    {#if statusInfo}
      <span class="pill {statusInfo.cls} status">
        {#if status === "installing"}<Icon name="loader" size={12} class="spin" />{:else}<span class="dot"></span>{/if}
        {statusInfo.text}
      </span>
    {/if}
  </div>
  <div class="content">
    <div class="title-row">
      <h3>{app.name}</h3>
      {#if latest?.latest}
        <span class="version mono" title="Dernière version publiée">{displayVersion(latest.latest.version)}</span>
      {/if}
    </div>
    <p class="tagline">{app.tagline}</p>
    <div class="controls">
      {#if primary}
        <div class="endpoint">
          <span class="led {led}" title={runtimeInfo(rt?.status).text}></span>
          {#if primary.url && !stopped}
            <button class="address mono" title="Se connecter à {primary.url}" onclick={connect}>{addressLabel(primary.url)}</button>
          {:else if primary.url}
            <span class="address mono">{addressLabel(primary.url)}</span>
          {:else}
            <span class="address">Ce poste</span>
          {/if}
          {#if op}
            <span class="busy" title="{actionLabels[op.action] ?? 'Opération'} en cours">
              <Icon name="loader" size={13} class="spin" />
              <span class="busy-label">{actionLabels[op.action] ?? "Opération"}…</span>
            </span>
          {:else if !rt?.status}
            <span class="busy" title="Lecture de l'état…"><Icon name="loader" size={13} class="spin" /></span>
          {:else}
            <button
              class="btn sm power"
              class:start={stopped}
              class:halt={!stopped}
              aria-label="{stopped ? 'Démarrer' : 'Arrêter'} {app.name}"
              title="{stopped ? 'Démarrer' : 'Arrêter'} {app.name}"
              onclick={toggle}
            >
              <Icon name={stopped ? "play" : "stop"} size={11} />
              <span class="power-label">{stopped ? "Démarrer" : "Arrêter"}</span>
            </button>
          {/if}
        </div>
      {/if}
      <div class="actions" class:stacked={status === "update" && !!primary}>
        {#if status === "update"}
          <button class="btn brand sm" onclick={update}><Icon name="up" size={16} /> Mettre à jour</button>
        {/if}
        {#if primary}
          <button
            class="btn sm"
            class:primary={status !== "update"}
            disabled={!primary.url || stopped}
            title={stopped ? `${app.name} est arrêtée : démarrez-la d'abord` : (primary.url ?? "")}
            onclick={connect}><Icon name="external" size={15} /> Se connecter</button
          >
        {:else if status === "available" && !app.comingSoon}
          <button
            class="btn primary sm"
            onclick={(e) => {
              e.stopPropagation();
              launcher.installing = { appId: app.id };
            }}><Icon name="download" size={16} /> Installer</button
          >
        {/if}
        <span class="more">Détails <Icon name="chevron" size={15} /></span>
      </div>
    </div>
  </div>
</article>

<style>
  .app-card {
    container-type: inline-size;
    position: relative;
    display: flex;
    flex-direction: column;
    border-radius: var(--radius-xl);
    background: var(--surface);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-sm);
    overflow: hidden;
    transition:
      transform 0.2s ease,
      box-shadow 0.2s ease,
      border-color 0.2s;
    animation: rise 0.4s ease both;
    animation-delay: calc(var(--i) * 60ms);
  }
  .app-card:hover {
    transform: translateY(-3px);
    box-shadow: var(--shadow);
    border-color: color-mix(in srgb, var(--app) 35%, var(--border));
  }
  .hit {
    position: absolute;
    inset: 0;
    border: none;
    background: none;
    cursor: pointer;
    z-index: 1;
    border-radius: inherit;
  }
  .visual {
    position: relative;
    height: 168px;
    display: grid;
    place-items: center;
    background:
      radial-gradient(120% 90% at 50% 100%, color-mix(in srgb, var(--app) 30%, transparent), transparent 70%),
      linear-gradient(180deg, color-mix(in srgb, var(--app) 10%, var(--surface)), color-mix(in srgb, var(--app) 4%, var(--surface)));
    overflow: hidden;
  }
  .halo {
    position: absolute;
    width: 180px;
    height: 180px;
    border-radius: 50%;
    background: color-mix(in srgb, var(--app) 26%, transparent);
    filter: blur(40px);
  }
  .visual img {
    position: relative;
    width: 132px;
    height: 132px;
    object-fit: contain;
    filter: drop-shadow(0 14px 18px rgb(30 10 60 / 25%));
    transition: transform 0.35s cubic-bezier(0.2, 0.8, 0.2, 1.2);
  }
  .app-card:hover .visual img {
    transform: translateY(-4px) scale(1.04) rotate(-1.5deg);
  }
  .status {
    position: absolute;
    top: 12px;
    right: 12px;
    box-shadow: var(--shadow-sm);
  }
  .content {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 16px 18px 18px;
    flex: 1;
  }
  .title-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
  }
  h3 {
    font-size: 19px;
  }
  .version {
    font-size: 12px;
    color: var(--muted);
  }
  .tagline {
    color: var(--text-2);
    font-size: 14px;
    min-height: 2.9em;
  }
  /* Pinned to the bottom: cards of a row line up whatever their text. */
  .controls {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin-top: auto;
    padding-top: 8px;
  }
  .endpoint {
    position: relative;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 9px;
    min-height: 40px;
    padding: 5px 5px 5px 12px;
    border-radius: 12px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .led {
    --ring: var(--success);
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border-strong);
    flex: none;
  }
  .led.on {
    background: var(--success);
    animation: beat 2.4s ease-out infinite;
  }
  .led.warn {
    background: var(--warning);
  }
  .led.work {
    --ring: var(--primary);
    background: var(--primary);
    animation: beat 1.2s ease-out infinite;
  }
  @keyframes beat {
    0% {
      box-shadow: 0 0 0 0 color-mix(in srgb, var(--ring) 50%, transparent);
    }
    70%,
    100% {
      box-shadow: 0 0 0 5px transparent;
    }
  }
  .address {
    flex: 1;
    min-width: 0;
    padding: 0;
    border: none;
    background: none;
    font-size: 12.5px;
    color: var(--text-2);
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  button.address {
    cursor: pointer;
  }
  button.address:hover {
    color: var(--primary);
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .power {
    height: 30px;
    padding: 0 11px;
    gap: 6px;
    font-size: 12.5px;
    border-radius: 9px;
    flex: none;
  }
  .power.halt {
    color: var(--text-2);
  }
  .busy {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 8px;
    font-size: 12.5px;
    color: var(--primary);
    flex: none;
  }
  .actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .actions .btn {
    position: relative;
    z-index: 2;
  }
  /* Narrow card: the address matters more than the button's words. */
  @container (max-width: 300px) {
    .power {
      width: 30px;
      padding: 0;
    }
    .power-label,
    .busy-label {
      display: none;
    }
    .busy {
      width: 30px;
      justify-content: center;
      padding: 0;
    }
  }
  /* Update and connect side by side do not fit a narrow card. */
  .actions.stacked .btn.brand {
    flex: 1 1 100%;
  }
  .more {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    font-size: 13px;
    color: var(--muted);
    transition: color 0.15s;
  }
  .app-card:hover .more {
    color: var(--primary);
  }
</style>
