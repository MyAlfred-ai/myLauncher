<script lang="ts">
  import { launcher } from "../launcher.svelte";
  import { relativeTime } from "../format";
  import AppCard from "../components/AppCard.svelte";
  import ServerOffer from "../components/ServerOffer.svelte";
  import Icon from "../components/Icon.svelte";

  const o = $derived(launcher.overview!);
  const updates = $derived(launcher.updateCount);
  const accessErrors = $derived(Object.values(o.latest).filter((l) => l.error).length);
  const installed = $derived(new Set(o.installations.filter((i) => i.state === "ready").map((i) => i.appId)).size);
</script>

<div class="page">
  <header class="hero">
    <div>
      <p class="eyebrow">Bienvenue</p>
      <h1>Vos applications <span class="grad">MyAlfred</span></h1>
      <p class="sub muted">
        {installed} sur {o.catalog.apps.length} installée{installed > 1 ? "s" : ""} ·
        vérification {relativeTime(o.lastCheck)}
      </p>
    </div>
    <button class="btn" onclick={() => launcher.checkUpdates()} disabled={launcher.checking}>
      <Icon name="refresh" size={16} class={launcher.checking ? "spin" : ""} />
      {launcher.checking ? "Vérification…" : "Rechercher des mises à jour"}
    </button>
  </header>

  {#if updates}
    <div class="banner">
      <div class="banner-icon"><Icon name="sparkles" size={20} /></div>
      <div class="banner-text">
        <strong>{updates} mise{updates > 1 ? "s" : ""} à jour disponible{updates > 1 ? "s" : ""}</strong>
        <span>Les nouvelles versions s'installent en un clic ; une sauvegarde est faite avant quand l'application le prévoit.</span>
      </div>
      <button class="btn" onclick={() => launcher.updateAll()}>Tout mettre à jour</button>
    </div>
  {/if}

  {#if accessErrors && !o.hasToken}
    <div class="callout warning">
      <Icon name="key" size={18} />
      <div>
        <strong>Versions indisponibles pour l'instant.</strong> GitHub ne répond pas (poste hors ligne ou limite de requêtes
        atteinte) : réessayez plus tard, ou ajoutez un token GitHub pour lever la limite.
        <button class="link" onclick={() => launcher.go({ name: "settings" })}>Ajouter un token</button>
      </div>
    </div>
  {/if}

  <section class="grid" aria-label="Applications">
    {#each o.catalog.apps as app, i (app.id)}
      <AppCard {app} index={i} />
    {/each}
  </section>

  <ServerOffer />
</div>

<style>
  .page {
    max-width: 1240px;
    margin: 0 auto;
    padding: 28px 32px 48px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  .hero {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  .eyebrow {
    font-size: 13px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin-bottom: 4px;
  }
  h1 {
    font-size: clamp(26px, 3.4vw, 34px);
    font-weight: 700;
    letter-spacing: -0.03em;
  }
  .grad {
    background: var(--brand-gradient);
    -webkit-background-clip: text;
    background-clip: text;
    color: transparent;
  }
  .sub {
    margin-top: 6px;
    font-size: 14px;
  }
  .banner {
    position: relative;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 18px;
    border-radius: var(--radius-lg);
    background: var(--brand-gradient);
    color: #fff;
    box-shadow: 0 16px 40px -18px rgb(214 38 153 / 70%);
    overflow: hidden;
  }
  .banner::after {
    content: "";
    position: absolute;
    inset: 0;
    background: radial-gradient(60% 120% at 90% 0%, rgb(255 255 255 / 22%), transparent 60%);
    pointer-events: none;
  }
  .banner-icon {
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    border-radius: 12px;
    background: rgb(255 255 255 / 20%);
    flex: none;
  }
  .banner-text {
    display: flex;
    flex-direction: column;
    flex: 1;
    font-size: 13.5px;
  }
  .banner-text strong {
    font-size: 15.5px;
  }
  .banner-text span {
    opacity: 0.92;
  }
  .banner .btn {
    position: relative;
    z-index: 1;
    background: #fff;
    color: #3b1a73;
    border: none;
  }
  .callout strong {
    color: var(--text);
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
  /* Four apps: 1, 2×2 or 4 columns, never an orphan card. */
  .page {
    container-type: inline-size;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 18px;
  }
  @container (min-width: 520px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  @container (min-width: 1040px) {
    .grid {
      grid-template-columns: repeat(4, minmax(0, 1fr));
    }
  }
  @media (max-width: 760px) {
    .page {
      padding: 20px 16px 32px;
      gap: 18px;
    }
    .hero .btn {
      width: 100%;
    }
    .banner {
      flex-wrap: wrap;
    }
    .banner .btn {
      width: 100%;
    }
  }
</style>
