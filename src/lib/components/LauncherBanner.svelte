<script lang="ts">
  import { launcher } from "../launcher.svelte";
  import { openUrl } from "../api";
  import Icon from "./Icon.svelte";
  import logo from "../../assets/myalfred-picto.svg";

  // A newer launcher, shown on every screen until installed or put off.
  const o = $derived(launcher.overview);
  const version = $derived(o?.launcherUpdate.version ?? null);
  const shown = $derived(!!version && launcher.launcherLater !== version);
  // Restarting would cut an install or an update short.
  const busy = $derived(launcher.runningOps.length > 0);
  const installing = $derived(launcher.selfUpdate === "installing");
</script>

{#if shown && o}
  <div class="wrap">
    <div class="bar" role="status">
      <span class="logo"><img src={logo} alt="" width="34" height="17" /></span>
      <div class="text">
        <strong>MyAlfred Launcher {version} est disponible</strong>
        <span>
          {#if installing}
            Téléchargement et installation… le launcher va redémarrer.
          {:else if busy}
            Une opération est en cours : vous pourrez installer dès qu'elle sera terminée.
          {:else}
            Vous utilisez la {o.launcherVersion}. Quelques secondes suffisent, puis le launcher redémarre.
          {/if}
        </span>
      </div>
      <div class="actions">
        <button class="link" onclick={() => openUrl(`https://github.com/MyAlfred-ai/myLauncher/releases/tag/v${version}`)}>Nouveautés</button>
        <button class="btn ghost sm" disabled={installing} onclick={() => (launcher.launcherLater = version)}>Plus tard</button>
        <button class="btn brand sm" disabled={busy || installing} onclick={() => launcher.installLauncher()}>
          <Icon name={installing ? "loader" : "download"} size={15} class={installing ? "spin" : ""} />
          Installer et redémarrer
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .wrap {
    max-width: 1240px;
    margin: 0 auto;
    padding: 20px 32px 0;
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px 12px 12px;
    border-radius: 16px;
    border: 1.5px solid transparent;
    background:
      linear-gradient(var(--surface), var(--surface)) padding-box,
      var(--brand-gradient) border-box;
    box-shadow: var(--shadow-sm);
    animation: rise 0.35s ease both;
  }
  .logo {
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    border-radius: 12px;
    background: var(--primary-soft);
    flex: none;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
    font-size: 13px;
    color: var(--text-2);
  }
  .text strong {
    font-size: 14.5px;
    color: var(--text);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: none;
  }
  .link {
    border: none;
    background: none;
    padding: 0 6px;
    color: var(--primary);
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  @media (max-width: 760px) {
    .wrap {
      padding: 16px 16px 0;
    }
    .bar {
      flex-wrap: wrap;
    }
    .actions {
      width: 100%;
      flex-wrap: wrap;
    }
    .actions .btn.brand {
      flex: 1;
    }
  }
</style>
