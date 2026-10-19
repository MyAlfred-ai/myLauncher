<script lang="ts">
  import { untrack } from "svelte";
  import { appIcon, launcher } from "../launcher.svelte";
  import { api } from "../api";
  import { displayVersion, relativeTime } from "../format";
  import Icon from "../components/Icon.svelte";
  import InstallationCard from "../components/InstallationCard.svelte";
  import Markdown from "../components/Markdown.svelte";
  import ServerOffer from "../components/ServerOffer.svelte";

  let { id, tab: initialTab }: { id: string; tab?: "installs" | "about" | "news" } = $props();

  const app = $derived(launcher.app(id));
  const installs = $derived(launcher.installsFor(id));
  const latest = $derived(launcher.latest(id));
  let tab = $state<"installs" | "about" | "news">(
    untrack(() => initialTab ?? (launcher.installsFor(id).length ? "installs" : "about")),
  );

  let changelog = $state<string | null>(null);
  let changelogError = $state<string | null>(null);

  $effect(() => {
    if (tab === "news" && changelog === null && !changelogError) {
      api
        .changelog(id)
        .then((c) => (changelog = c))
        .catch((e) => (changelogError = String(e)));
    }
  });

  const installed = $derived(installs.some((i) => i.state !== "failed"));
</script>

{#if app}
  <div class="page" style:--app={app.color}>
    <button class="back btn ghost sm" onclick={() => launcher.go({ name: "library" })}>
      <Icon name="back" size={16} /> Applications
    </button>

    <header class="hero">
      <div class="hero-bg"></div>
      <img src={appIcon(app.icon)} alt="" width="168" height="168" />
      <div class="hero-text">
        <h1>{app.name}</h1>
        <p class="tagline">{app.tagline}</p>
        <div class="meta">
          {#if latest?.latest}
            <span class="pill brand"><Icon name="package" size={13} /> {displayVersion(latest.latest.version)}</span>
            {#if latest.latest.date}<span class="muted">publiée {relativeTime(latest.latest.date)}</span>{/if}
          {:else if latest?.error}
            <span class="pill warning" title={latest.error}><Icon name="alert" size={13} /> Version indisponible</span>
          {/if}
        </div>
        <div class="hero-actions">
          {#if installed}
            <span class="pill success"><Icon name="laptop" size={13} /> Installé sur ce poste</span>
          {:else}
            <button class="btn primary" onclick={() => (launcher.installing = { appId: app.id })}>
              <Icon name="download" size={17} /> Installer
            </button>
            <span class="muted small">Sur ce poste, pour essayer ou travailler seul</span>
          {/if}
        </div>
      </div>
    </header>

    <ServerOffer appId={app.id} />

    {#if latest?.error}
      <div class="callout warning">
        <Icon name="alert" size={18} />
        <span class="selectable">{latest.error}</span>
      </div>
    {/if}

    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={tab === "installs"} class:active={tab === "installs"} onclick={() => (tab = "installs")}>
        Installations {#if installs.length}<span class="count">{installs.length}</span>{/if}
      </button>
      <button role="tab" aria-selected={tab === "about"} class:active={tab === "about"} onclick={() => (tab = "about")}>Présentation</button>
      <button role="tab" aria-selected={tab === "news"} class:active={tab === "news"} onclick={() => (tab = "news")}>Nouveautés</button>
    </div>

    <section class="tab-body">
      {#if tab === "installs"}
        {#if installs.length}
          <div class="installs">
            {#each installs as inst (inst.id)}
              <InstallationCard {inst} />
            {/each}
          </div>
        {:else}
          <div class="empty card">
            <Icon name="download" size={28} />
            <h3>{app.name} n'est pas encore installé</h3>
            <p class="muted">Installez-le sur ce poste pour l'essayer ou travailler seul.</p>
            <button class="btn primary" onclick={() => (launcher.installing = { appId: app.id })}>Installer {app.name}</button>
          </div>
        {/if}
      {:else if tab === "about"}
        <div class="about">
          <div class="card about-main">
            <p class="desc">{app.description}</p>
            {#if app.features.length}
              <ul class="features">
                {#each app.features as f}
                  <li><span class="tick"><Icon name="check" size={14} stroke={2.5} /></span>{f}</li>
                {/each}
              </ul>
            {/if}
          </div>
          <aside class="card about-side">
            <h3>Prérequis</h3>
            <ul class="reqs">
              {#each app.requirements as r}
                <li><Icon name="check-circle" size={16} /> {r}</li>
              {/each}
            </ul>
            <h3>Version</h3>
            <dl>
              <dt>Dernière</dt>
              <dd class="mono">{displayVersion(latest?.latest?.version)}</dd>
              {#if latest?.latest?.summary}
                <dt>Dernier changement</dt>
                <dd>{latest.latest.summary}</dd>
              {/if}
              <dt>Source</dt>
              <dd class="mono">{app.repo.owner}/{app.repo.name}</dd>
            </dl>
          </aside>
        </div>
      {:else}
        <div class="card news">
          {#if changelog !== null}
            <Markdown source={changelog} />
          {:else if changelogError}
            <div class="callout warning"><Icon name="alert" size={18} /><span>{changelogError}</span></div>
          {:else}
            <p class="muted"><Icon name="loader" size={16} class="spin" /> Chargement des nouveautés…</p>
          {/if}
        </div>
      {/if}
    </section>
  </div>
{/if}

<style>
  .page {
    max-width: 1100px;
    margin: 0 auto;
    padding: 18px 32px 48px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  .back {
    align-self: flex-start;
    margin-left: -8px;
    color: var(--text-2);
  }
  .hero {
    position: relative;
    display: flex;
    align-items: center;
    gap: 28px;
    padding: 26px 30px;
    border-radius: var(--radius-xl);
    border: 1px solid color-mix(in srgb, var(--app) 22%, var(--border));
    overflow: hidden;
  }
  .hero-bg {
    position: absolute;
    inset: 0;
    background:
      radial-gradient(60% 140% at 0% 50%, color-mix(in srgb, var(--app) 30%, transparent), transparent 70%),
      linear-gradient(120deg, color-mix(in srgb, var(--app) 12%, var(--surface)), var(--surface) 70%);
    z-index: 0;
  }
  .hero img {
    position: relative;
    width: 168px;
    height: 168px;
    object-fit: contain;
    filter: drop-shadow(0 18px 24px rgb(30 10 60 / 28%));
    flex: none;
    animation: rise 0.5s ease both;
  }
  .hero-text {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  h1 {
    font-size: clamp(30px, 4vw, 40px);
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  .tagline {
    font-size: 17px;
    color: var(--text-2);
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    flex-wrap: wrap;
  }
  .hero-actions {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-top: 8px;
    flex-wrap: wrap;
  }
  .small {
    font-size: 13px;
  }
  .tabs {
    display: flex;
    gap: 4px;
    box-shadow: inset 0 -1px 0 var(--border);
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }
  .tabs button {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 42px;
    padding: 0 14px;
    border: none;
    background: none;
    color: var(--muted);
    font-weight: 500;
    cursor: pointer;
    white-space: nowrap;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs button.active {
    color: var(--text);
  }
  .tabs button.active::after {
    content: "";
    position: absolute;
    left: 10px;
    right: 10px;
    bottom: 0;
    height: 3px;
    border-radius: 3px 3px 0 0;
    background: var(--brand-gradient);
  }
  .count {
    font-size: 11.5px;
    padding: 1px 7px;
    border-radius: 99px;
    background: var(--surface-3);
  }
  .installs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(360px, 1fr));
    gap: 16px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 40px 24px;
    text-align: center;
    color: var(--muted);
  }
  .empty h3 {
    color: var(--text);
  }
  .about {
    display: grid;
    grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr);
    gap: 16px;
  }
  .about-main,
  .about-side,
  .news {
    padding: 22px 24px;
  }
  .desc {
    font-size: 15.5px;
    color: var(--text-2);
    line-height: 1.65;
  }
  .features {
    list-style: none;
    padding: 0;
    margin: 18px 0 0;
    display: grid;
    gap: 10px;
  }
  .features li {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .tick {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: 8px;
    background: color-mix(in srgb, var(--app) 16%, transparent);
    color: var(--app);
    flex: none;
  }
  .about-side h3 {
    font-size: 14px;
    margin-bottom: 10px;
  }
  .about-side h3:not(:first-child) {
    margin-top: 20px;
  }
  .reqs {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 8px;
    font-size: 14px;
    color: var(--text-2);
  }
  .reqs li {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .reqs :global(svg) {
    color: var(--success);
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 14px;
    font-size: 13.5px;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .news {
    max-height: none;
  }
  @media (max-width: 860px) {
    .about {
      grid-template-columns: 1fr;
    }
    .installs {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 760px) {
    .page {
      padding: 12px 16px 32px;
    }
    .hero {
      flex-direction: column;
      text-align: center;
      padding: 22px 18px;
      gap: 12px;
    }
    .hero img {
      width: 128px;
      height: 128px;
    }
    .meta,
    .hero-actions {
      justify-content: center;
    }
  }
</style>
