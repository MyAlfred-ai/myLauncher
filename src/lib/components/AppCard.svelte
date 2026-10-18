<script lang="ts">
  import { appIcon, launcher } from "../launcher.svelte";
  import { appStatus, displayVersion } from "../format";
  import { openUrl } from "../api";
  import type { AppDef } from "../types";
  import Icon from "./Icon.svelte";

  let { app, index = 0 }: { app: AppDef; index?: number } = $props();

  const installs = $derived(launcher.installsFor(app.id));
  const status = $derived(appStatus(installs));
  const latest = $derived(launcher.latest(app.id));
  const ready = $derived(installs.find((i) => i.state === "ready" && i.url));
  const toUpdate = $derived(installs.filter((i) => i.updateAvailable && !i.busyOp));

  const statusInfo = $derived.by(() => {
    switch (status) {
      case "installing":
        return { cls: "brand", text: "En cours…" };
      case "update":
        return { cls: "warning", text: "Mise à jour disponible" };
      case "installed":
        return { cls: "success", text: installs.length > 1 ? `${installs.length} installations` : "Installé" };
      case "failed":
        return { cls: "danger", text: "Installation échouée" };
      default:
        return null;
    }
  });

  function open() {
    launcher.go({ name: "app", id: app.id });
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
    <div class="actions">
      {#if status === "update"}
        <button class="btn brand sm" onclick={update}><Icon name="up" size={16} /> Mettre à jour</button>
      {/if}
      {#if ready}
        <button
          class="btn sm"
          class:primary={status !== "update"}
          onclick={(e) => {
            e.stopPropagation();
            openUrl(ready.url!);
          }}><Icon name="external" size={15} /> Ouvrir</button
        >
      {:else if status === "available"}
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
</article>

<style>
  .app-card {
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
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
  }
  .actions .btn {
    position: relative;
    z-index: 2;
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
