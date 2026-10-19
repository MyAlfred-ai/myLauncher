<script lang="ts">
  import { onMount } from "svelte";
  import { launcher } from "./lib/launcher.svelte";
  import Icon, { type IconName } from "./lib/components/Icon.svelte";
  import Toasts from "./lib/components/Toasts.svelte";
  import InstallDialog from "./lib/components/InstallDialog.svelte";
  import Library from "./lib/views/Library.svelte";
  import AppDetail from "./lib/views/AppDetail.svelte";
  import Contact from "./lib/views/Contact.svelte";
  import Activity from "./lib/views/Activity.svelte";
  import OpView from "./lib/views/OpView.svelte";
  import SettingsView from "./lib/views/Settings.svelte";
  import logo from "./assets/myalfred-picto.svg";

  onMount(() => {
    launcher.init();
  });

  const nav: { id: "library" | "activity" | "settings" | "contact"; label: string; short?: string; icon: IconName }[] = [
    { id: "library", label: "Applications", icon: "apps" },
    { id: "activity", label: "Activité", icon: "activity" },
    { id: "settings", label: "Réglages", icon: "settings" },
    { id: "contact", label: "Infos & contact", short: "Contact", icon: "message" },
  ];

  const section = $derived.by(() => {
    const v = launcher.view.name;
    if (v === "app") return "library";
    if (v === "op") return "activity";
    return v;
  });
  const running = $derived(launcher.runningOps.length);
  const mac = $derived(launcher.overview?.platform === "macos");
</script>

<div class="shell" class:mac>
  {#if mac}
    <div class="drag" data-tauri-drag-region></div>
  {/if}

  <aside class="sidebar">
    <div class="brand" data-tauri-drag-region>
      <img src={logo} alt="" width="44" height="22" />
      <div>
        <div class="brand-name">MyAlfred</div>
        <div class="brand-sub">Launcher</div>
      </div>
    </div>

    <nav aria-label="Navigation principale">
      {#each nav as item (item.id)}
        <button
          class="nav-item"
          class:active={section === item.id}
          aria-current={section === item.id ? "page" : undefined}
          onclick={() => launcher.go({ name: item.id })}
        >
          <Icon name={item.icon} size={19} />
          <span class="nav-label">{item.label}</span>
          {#if item.short}<span class="nav-short">{item.short}</span>{/if}
          {#if item.id === "library" && launcher.updateCount}
            <span class="badge" title="Mises à jour disponibles">{launcher.updateCount}</span>
          {:else if item.id === "activity" && running}
            <span class="badge live" title="Opérations en cours"><Icon name="loader" size={12} class="spin" /> {running}</span>
          {/if}
        </button>
      {/each}
    </nav>

    <div class="side-foot">
      {#if launcher.overview}
        <span class="muted">Version {launcher.overview.launcherVersion}</span>
      {/if}
    </div>
  </aside>

  <main>
    {#if launcher.loadError && !launcher.overview}
      <div class="fatal">
        <Icon name="alert" size={28} />
        <h2>Le launcher n'a pas pu démarrer</h2>
        <p class="muted selectable">{launcher.loadError}</p>
        <button class="btn primary" onclick={() => launcher.refresh()}>Réessayer</button>
      </div>
    {:else if !launcher.overview}
      <div class="fatal"><Icon name="loader" size={28} class="spin" /></div>
    {:else if launcher.view.name === "library"}
      <Library />
    {:else if launcher.view.name === "app"}
      {#key launcher.view.id}
        <AppDetail id={launcher.view.id} tab={launcher.view.tab} />
      {/key}
    {:else if launcher.view.name === "contact"}
      {#key launcher.view.appId}
        <Contact appId={launcher.view.appId} />
      {/key}
    {:else if launcher.view.name === "activity"}
      <Activity />
    {:else if launcher.view.name === "op"}
      {#key launcher.view.id}
        <OpView id={launcher.view.id} />
      {/key}
    {:else if launcher.view.name === "settings"}
      <SettingsView />
    {/if}
  </main>
</div>

{#if launcher.installing}
  <InstallDialog appId={launcher.installing.appId} />
{/if}
<Toasts />

<style>
  .shell {
    display: grid;
    grid-template-columns: var(--sidebar-w) minmax(0, 1fr);
    height: 100%;
  }
  .shell.mac {
    --titlebar-h: 28px;
  }
  .drag {
    position: fixed;
    inset: 0 0 auto 0;
    height: 28px;
    z-index: 50;
  }

  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: calc(18px + var(--titlebar-h)) 14px 16px;
    background: color-mix(in srgb, var(--surface) 70%, var(--bg));
    border-right: 1px solid var(--border);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 4px 10px 20px;
  }
  .brand-name {
    font-weight: 700;
    font-size: 17px;
    letter-spacing: -0.02em;
    line-height: 1.1;
  }
  .brand-sub {
    font-size: 12px;
    font-weight: 500;
    background: var(--brand-gradient);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 42px;
    padding: 0 12px;
    border: none;
    border-radius: 12px;
    background: transparent;
    color: var(--text-2);
    font-weight: 500;
    font-size: 14.5px;
    cursor: pointer;
    text-align: left;
    transition:
      background 0.15s,
      color 0.15s;
  }
  .nav-item:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .nav-item.active {
    background: var(--primary-soft);
    color: var(--primary);
  }
  .nav-short {
    display: none;
  }
  .badge {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    min-width: 22px;
    height: 22px;
    padding: 0 7px;
    border-radius: 99px;
    background: var(--brand-gradient);
    color: #fff;
    font-size: 12px;
    font-weight: 600;
    justify-content: center;
  }
  .badge.live {
    background: var(--primary);
  }
  .side-foot {
    margin-top: auto;
    padding: 0 12px;
    font-size: 12px;
  }

  main {
    overflow-y: auto;
    overflow-x: hidden;
    padding-top: var(--titlebar-h);
    scroll-behavior: smooth;
  }

  .fatal {
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 14px;
    padding: 24px;
    text-align: center;
    color: var(--muted);
  }
  .fatal h2 {
    color: var(--text);
  }

  /* Narrow windows: the sidebar becomes a bottom tab bar. */
  @media (max-width: 760px) {
    .shell {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(0, 1fr) auto;
    }
    main {
      grid-row: 1;
    }
    .sidebar {
      grid-row: 2;
      flex-direction: row;
      padding: 6px 8px calc(6px + env(safe-area-inset-bottom));
      border-right: none;
      border-top: 1px solid var(--border);
      background: color-mix(in srgb, var(--surface) 92%, transparent);
      backdrop-filter: blur(16px);
    }
    .brand,
    .side-foot {
      display: none;
    }
    nav {
      flex-direction: row;
      flex: 1;
      justify-content: space-around;
    }
    .nav-item {
      position: relative;
      flex-direction: column;
      gap: 2px;
      height: 54px;
      flex: 1;
      justify-content: center;
      font-size: 11px;
      padding: 0 4px;
    }
    .nav-item:has(.nav-short) .nav-label {
      display: none;
    }
    .nav-short {
      display: inline;
    }
    .badge {
      position: absolute;
      top: 4px;
      left: calc(50% + 6px);
      margin: 0;
      min-width: 18px;
      height: 18px;
      font-size: 10.5px;
      padding: 0 5px;
    }
  }
</style>
