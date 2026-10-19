<script lang="ts">
  import { onMount } from "svelte";
  import { launcher, applyTheme } from "../launcher.svelte";
  import { api, isTauri, openUrl, pickDir } from "../api";
  import { formatDate, relativeTime } from "../format";
  import type { Settings, TokenStatus } from "../types";
  import Icon, { type IconName } from "../components/Icon.svelte";

  const o = $derived(launcher.overview!);
  let s = $state<Settings>({ ...launcher.overview!.settings });
  let token = $state("");
  let tokenStatus = $state<TokenStatus | null>(null);
  let savingToken = $state(false);
  let tokenError = $state<string | null>(null);
  const lu = $derived(o.launcherUpdate);

  onMount(async () => {
    tokenStatus = await api.githubStatus().catch(() => null);
  });

  async function save(patch: Partial<Settings>) {
    const next = { ...s, ...patch };
    try {
      s = await api.saveSettings(next);
      applyTheme(s.theme);
    } catch (e) {
      launcher.toast(String(e), "error");
    }
  }

  async function saveToken(e: Event) {
    e.preventDefault();
    savingToken = true;
    tokenError = null;
    try {
      tokenStatus = await api.setGithubToken(token);
      token = "";
      launcher.toast(`Connecté à GitHub (${tokenStatus.login})`, "success");
      launcher.checkUpdates();
    } catch (err) {
      tokenError = String(err).replace(/^Error:\s*/, "");
    } finally {
      savingToken = false;
    }
  }

  async function clearToken() {
    await api.clearGithubToken();
    tokenStatus = { present: false, login: null, error: null };
  }

  async function chooseInstallDir() {
    const d = await pickDir("Dossier des applications");
    if (d) save({ installDir: d });
  }

  const tokenUrl =
    "https://github.com/settings/tokens/new?scopes=repo,read:packages&description=MyAlfred%20Launcher";
  const themes: { id: Settings["theme"]; label: string; icon: IconName }[] = [
    { id: "system", label: "Système", icon: "monitor" },
    { id: "light", label: "Clair", icon: "sun" },
    { id: "dark", label: "Sombre", icon: "moon" },
  ];
  const sourceLabel = { embedded: "intégré au launcher", cache: "téléchargé (copie locale)", remote: "à jour depuis GitHub" };
</script>

<div class="page">
  <header>
    <h1>Réglages</h1>
  </header>

  <section class="card block">
    <div class="block-head">
      <div class="b-icon"><Icon name="github" size={20} /></div>
      <div>
        <h2>Accès GitHub</h2>
        <p class="muted">Nécessaire tant que les dépôts MyAlfred sont privés. Le token reste dans le trousseau de votre système.</p>
      </div>
    </div>
    {#if tokenStatus?.present}
      <div class="token-ok" class:warn={!!tokenStatus.error}>
        <Icon name={tokenStatus.error ? "alert" : "check-circle"} size={18} />
        <span>
          {#if tokenStatus.error}{tokenStatus.error}{:else}Connecté en tant que <strong>{tokenStatus.login}</strong>{/if}
        </span>
        <button class="btn sm danger" onclick={clearToken}>Supprimer</button>
      </div>
    {/if}
    <form class="field" onsubmit={saveToken}>
      <label for="token">{tokenStatus?.present ? "Remplacer le token" : "Token d'accès personnel"}</label>
      <div class="input-group">
        <input id="token" class="input mono" type="password" bind:value={token} placeholder="ghp_… ou github_pat_…" autocomplete="off" spellcheck="false" />
        <button class="btn primary" disabled={!token.trim() || savingToken}>
          {#if savingToken}<Icon name="loader" size={16} class="spin" />{/if} Enregistrer
        </button>
      </div>
      {#if tokenError}<span class="err">{tokenError}</span>{/if}
      <span class="help">
        Droits requis : lecture des dépôts (<span class="mono">repo</span>) et des images (<span class="mono">read:packages</span>).
        <button type="button" class="link" onclick={() => openUrl(tokenUrl)}>Créer un token sur GitHub</button>
      </span>
    </form>
  </section>

  <section class="card block">
    <div class="block-head">
      <div class="b-icon"><Icon name="refresh" size={20} /></div>
      <div>
        <h2>Mises à jour</h2>
        <p class="muted">Dernière vérification {relativeTime(o.lastCheck)}.</p>
      </div>
    </div>
    <label class="row">
      <span><strong>Vérifier automatiquement</strong><span class="muted">Au démarrage puis à intervalle régulier</span></span>
      <input type="checkbox" class="switch" checked={s.autoCheck} onchange={(e) => save({ autoCheck: e.currentTarget.checked })} />
    </label>
    <label class="row">
      <span><strong>Fréquence</strong></span>
      <select class="input narrow" value={String(s.checkIntervalHours)} onchange={(e) => save({ checkIntervalHours: Number(e.currentTarget.value) })} disabled={!s.autoCheck}>
        <option value="1">Toutes les heures</option>
        <option value="3">Toutes les 3 heures</option>
        <option value="6">Toutes les 6 heures</option>
        <option value="12">Toutes les 12 heures</option>
        <option value="24">Une fois par jour</option>
      </select>
    </label>
    <label class="row">
      <span><strong>Notifications</strong><span class="muted">Nouvelle version, fin d'installation ou de mise à jour</span></span>
      <input type="checkbox" class="switch" checked={s.notify} onchange={(e) => save({ notify: e.currentTarget.checked })} />
    </label>
    <p class="help">Les mises à jour automatiques s'activent application par application, depuis le menu ⋯ de chaque installation.</p>
  </section>

  <section class="card block">
    <div class="block-head">
      <div class="b-icon"><Icon name="folder" size={20} /></div>
      <div>
        <h2>Installation sur ce poste</h2>
        <p class="muted">Dossier où chaque application reçoit son sous-dossier.</p>
      </div>
    </div>
    <div class="input-group">
      <input class="input mono" value={s.installDir} onchange={(e) => save({ installDir: e.currentTarget.value })} spellcheck="false" aria-label="Dossier des applications" />
      {#if isTauri}<button class="btn" onclick={chooseInstallDir}><Icon name="folder" size={16} /> Choisir</button>{/if}
    </div>
  </section>

  <section class="card block">
    <div class="block-head">
      <div class="b-icon"><Icon name="sun" size={20} /></div>
      <div><h2>Apparence</h2></div>
    </div>
    <div class="segmented" role="radiogroup" aria-label="Thème">
      {#each themes as t}
        <button role="radio" aria-checked={s.theme === t.id} class:active={s.theme === t.id} onclick={() => save({ theme: t.id })}>
          <Icon name={t.icon} size={16} /> {t.label}
        </button>
      {/each}
    </div>
  </section>

  <section class="card block">
    <div class="block-head">
      <div class="b-icon"><Icon name="info" size={20} /></div>
      <div>
        <h2>À propos</h2>
        <p class="muted">MyAlfred Launcher {o.launcherVersion} · {o.platform}</p>
      </div>
    </div>
    <div class="row">
      <span>
        <strong>Catalogue des applications</strong>
        <span class="muted">Version du {o.catalog.updated}, {sourceLabel[o.catalogSource]}{o.catalogFetchedAt ? ` (${formatDate(o.catalogFetchedAt)})` : ""}</span>
      </span>
    </div>
    <div class="row">
      <span>
        <strong>Mise à jour du launcher</strong>
        <span class="muted">
          {#if launcher.selfUpdate === "checking"}Recherche…
          {:else if launcher.selfUpdate === "installing"}Installation, le launcher va redémarrer…
          {:else if lu.version}Version {lu.version} disponible (vous avez la {o.launcherVersion}).
          {:else if lu.error}<span title={lu.error}>Vérification impossible pour l'instant.</span>
          {:else if lu.checkedAt}Le launcher est à jour (vérifié {relativeTime(lu.checkedAt)}).
          {:else}Vérifiée au démarrage puis à intervalle régulier ; les versions sont signées et contrôlées avant installation.{/if}
        </span>
      </span>
      {#if lu.version}
        <button
          class="btn brand sm"
          disabled={launcher.selfUpdate !== "idle" || launcher.runningOps.length > 0}
          title={launcher.runningOps.length ? "Une opération est en cours" : ""}
          onclick={() => launcher.installLauncher()}>Installer et redémarrer</button
        >
      {:else}
        <button class="btn sm" onclick={() => launcher.checkLauncher()} disabled={launcher.selfUpdate !== "idle"}>Rechercher</button>
      {/if}
    </div>
    <details>
      <summary class="muted">Avancé</summary>
      <div class="field" style="margin-top:10px">
        <label for="catalog">Adresse du catalogue</label>
        <input id="catalog" class="input mono" value={s.catalogUrl} onchange={(e) => save({ catalogUrl: e.currentTarget.value })} spellcheck="false" />
        <span class="help">Le token GitHub n'est transmis qu'aux adresses GitHub.</span>
      </div>
    </details>
  </section>
</div>

<style>
  .page {
    max-width: 820px;
    margin: 0 auto;
    padding: 28px 32px 48px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  h1 {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 20px 22px;
  }
  .block-head {
    display: flex;
    gap: 14px;
    align-items: flex-start;
  }
  .b-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 12px;
    background: var(--primary-soft);
    color: var(--primary);
    flex: none;
  }
  h2 {
    font-size: 16.5px;
  }
  .block-head p {
    font-size: 13.5px;
    margin-top: 2px;
  }
  .token-ok {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 12px;
    background: var(--success-soft);
    color: var(--success);
    font-size: 14px;
  }
  .token-ok.warn {
    background: var(--warning-soft);
    color: var(--warning);
  }
  .token-ok span {
    flex: 1;
    color: var(--text);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .row > span {
    display: flex;
    flex-direction: column;
    font-size: 14px;
  }
  .row > span .muted {
    font-size: 13px;
  }
  .narrow {
    width: auto;
    min-width: 200px;
  }
  .help,
  .err {
    font-size: 12.5px;
  }
  .help {
    color: var(--muted);
  }
  .err {
    color: var(--danger);
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
    font-size: inherit;
  }
  .segmented {
    display: inline-flex;
    align-self: flex-start;
    padding: 4px;
    border-radius: 14px;
    background: var(--surface-2);
    gap: 4px;
  }
  .segmented button {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 14px;
    border: none;
    border-radius: 10px;
    background: none;
    color: var(--text-2);
    font-weight: 500;
    cursor: pointer;
  }
  .segmented button.active {
    background: var(--bg-elevated);
    color: var(--text);
    box-shadow: var(--shadow-sm);
  }
  details summary {
    cursor: pointer;
    font-size: 13.5px;
  }
  @media (max-width: 760px) {
    .page {
      padding: 20px 16px 32px;
    }
    .row {
      flex-wrap: wrap;
    }
    .narrow {
      width: 100%;
    }
  }
</style>
