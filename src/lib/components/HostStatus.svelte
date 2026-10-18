<script lang="ts">
  import type { HostCheck } from "../types";
  import Icon from "./Icon.svelte";
  import { openUrl } from "../api";

  let { check, loading, local }: { check: HostCheck | null; loading: boolean; local: boolean } = $props();

  const items = $derived(
    check?.reachable
      ? [
          { ok: !!check.docker, label: check.docker ? check.docker.replace(/^Docker version /, "Docker ").replace(/, build.*$/, "") : "Docker absent" },
          { ok: !!check.compose, label: check.compose ? `Compose ${check.compose}` : "Compose v2 absent" },
          { ok: check.daemon, label: check.daemon ? "Docker démarré" : "Docker arrêté" },
          { ok: !!check.git, label: check.git ? check.git.replace(/^git version /, "Git ").replace(/ \(.*\)$/, "") : "Git absent" },
        ]
      : [],
  );
  const allOk = $derived(items.length > 0 && items.every((i) => i.ok));
</script>

<div class="host" class:ok={allOk} class:bad={check && !allOk}>
  {#if loading}
    <span class="muted"><Icon name="loader" size={15} class="spin" /> Vérification de la machine…</span>
  {:else if check && !check.reachable}
    <div class="row">
      <Icon name="x-circle" size={17} />
      <span>{check.error}</span>
    </div>
  {:else if check}
    <div class="chips">
      {#each items as it}
        <span class="chip" class:ko={!it.ok}><Icon name={it.ok ? "check" : "x"} size={13} stroke={2.6} /> {it.label}</span>
      {/each}
      {#if check.os}<span class="chip neutral">{check.os}</span>{/if}
    </div>
    {#if !allOk}
      <p class="hint">
        {#if local}
          {#if !check.docker}
            Installez Docker Desktop, lancez-le, puis vérifiez à nouveau.
            <button class="link" onclick={() => openUrl("https://www.docker.com/products/docker-desktop/")}>Télécharger Docker Desktop</button>
          {:else if !check.daemon}
            Lancez Docker Desktop et attendez qu'il soit prêt, puis vérifiez à nouveau.
          {:else if !check.git}
            Installez Git (sous macOS : <span class="mono">xcode-select --install</span>).
          {/if}
        {:else}
          Installez Docker Engine et le plugin Compose sur le serveur, et ajoutez l'utilisateur au groupe « docker ».
        {/if}
      </p>
    {/if}
  {/if}
</div>

<style>
  .host {
    padding: 12px 14px;
    border-radius: 14px;
    background: var(--surface-2);
    font-size: 13.5px;
  }
  .host.bad {
    background: var(--warning-soft);
  }
  .row {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    color: var(--danger);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 26px;
    padding: 0 10px;
    border-radius: 99px;
    background: var(--success-soft);
    color: var(--success);
    font-size: 12.5px;
    font-weight: 500;
  }
  .chip.ko {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .chip.neutral {
    background: var(--surface);
    color: var(--muted);
  }
  .hint {
    margin-top: 10px;
    color: var(--text-2);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--primary);
    font-weight: 600;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
</style>
