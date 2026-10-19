<script lang="ts">
  import { appIcon, launcher } from "../launcher.svelte";
  import { api, isTauri, pickDir } from "../api";
  import { initialValues, isActive, validateField } from "../format";
  import type { HostCheck, TargetDef } from "../types";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import HostStatus from "./HostStatus.svelte";
  import ServerOffer from "./ServerOffer.svelte";
  import ServiceNeeds from "./ServiceNeeds.svelte";

  let { appId }: { appId: string } = $props();

  const app = $derived(launcher.app(appId)!);
  const installs = $derived(launcher.installsFor(appId));

  let step = $state<1 | 2>(1);
  let check = $state<HostCheck | null>(null);
  let checking = $state(false);
  let dir = $state("");
  let values = $state<Record<string, string>>({});
  let reveal = $state<Record<string, boolean>>({});
  let touched = $state(false);
  let submitting = $state(false);
  let error = $state<string | null>(null);

  const tdef: TargetDef | null = $derived(app.targets.local ?? null);
  const fields = $derived((tdef?.fields ?? []).filter((f) => isActive(f, values)));
  const errors = $derived(Object.fromEntries(fields.map((f) => [f.key, validateField(f, values)])));
  const hasErrors = $derived(Object.values(errors).some(Boolean));
  const ready = $derived(!!check?.reachable && !!check.docker && !!check.compose && check.daemon && !!check.git);
  const alreadyThere = $derived(installs.some((i) => i.state !== "failed"));

  function close() {
    launcher.installing = null;
  }

  async function runCheck() {
    checking = true;
    check = null;
    try {
      check = await api.checkHost();
    } catch (e) {
      check = { reachable: false, os: "", git: "", docker: "", compose: "", daemon: false, error: String(e) };
    } finally {
      checking = false;
    }
  }

  $effect(() => {
    if (step === 1) runCheck();
  });

  async function next() {
    if (!tdef) return;
    values = initialValues(tdef.fields, installs[0]?.values ?? {});
    dir = installs[0]?.dir ?? (await api.defaultInstallDir(appId));
    touched = false;
    error = null;
    step = 2;
  }

  async function generate(key: string, kind: string) {
    values[key] = await api.generateSecret(kind);
    reveal[key] = true;
  }

  async function chooseDir() {
    const d = await pickDir("Dossier d'installation");
    if (d) dir = `${d.replace(/[\\/]$/, "")}${launcher.overview?.platform === "windows" ? "\\" : "/"}${appId}`;
  }

  async function submit(e: Event) {
    e.preventDefault();
    touched = true;
    if (hasErrors) return;
    submitting = true;
    error = null;
    try {
      const payload: Record<string, string> = {};
      for (const f of tdef?.fields ?? []) if (isActive(f, values)) payload[f.key] = values[f.key] ?? "";
      const opId = await api.install({ appId, dir, values: payload });
      close();
      await launcher.refresh();
      launcher.go({ name: "op", id: opId });
    } catch (err) {
      error = String(err).replace(/^Error:\s*/, "");
    } finally {
      submitting = false;
    }
  }
</script>

<Modal
  title={step === 1 ? `Installer ${app.name}` : "Configuration"}
  subtitle="Sur ce poste"
  width={640}
  onclose={close}
>
  {#snippet header()}
    <img class="mini" src={appIcon(app.icon)} alt="" width="44" height="44" style:--app={app.color} />
  {/snippet}

  {#if step === 1}
    <div class="here">
      <span class="t-icon"><Icon name="laptop" size={22} /></span>
      <span class="t-text">
        <strong>Ce poste</strong>
        <span class="muted">Pour essayer {app.name} ou travailler seul</span>
      </span>
      {#if tdef?.experimental}<span class="pill warning">Expérimental</span>{/if}
      {#if alreadyThere}<span class="pill success">Déjà installé</span>{/if}
    </div>

    <div class="check">
      <div class="check-head">
        <span class="label">Vérification du poste</span>
        <button class="btn ghost sm" onclick={runCheck} disabled={checking}><Icon name="refresh" size={14} /> Vérifier à nouveau</button>
      </div>
      <HostStatus {check} loading={checking} />
    </div>

    {#if !tdef}
      <div class="callout warning">
        <Icon name="info" size={18} />
        <span>{app.name} ne s'installe pas sur un poste.</span>
      </div>
    {:else if alreadyThere}
      <div class="callout warning">
        <Icon name="info" size={18} />
        <span>{app.name} est déjà installé ici. Utilisez « Mettre à jour » depuis sa fiche.</span>
      </div>
    {/if}

    {#if tdef && !alreadyThere && app.services?.length}
      <div class="prepare">
        <span class="label">À prévoir</span>
        <ServiceNeeds services={app.services} compact />
      </div>
    {/if}

    <div class="team">
      <ServerOffer {appId} onnavigate={close} />
    </div>
  {:else if tdef}
    <form id="install-form" class="form" onsubmit={submit} novalidate>
      {#if tdef.experimental}
        <div class="callout warning"><Icon name="alert" size={18} /><span><strong>Expérimental.</strong> Ce mode d'installation est récent : signalez-nous tout souci.</span></div>
      {/if}
      {#if tdef.notes.length}
        <div class="callout">
          <Icon name="info" size={18} />
          <ul class="notes">
            {#each tdef.notes as note}<li>{note}</li>{/each}
          </ul>
        </div>
      {/if}

      {#each fields as f (f.key)}
        <div class="field">
          <label for="f-{f.key}">{f.label}{#if !f.required}<span class="muted">&nbsp;(facultatif)</span>{/if}</label>
          {#if f.kind === "select"}
            <select id="f-{f.key}" class="input" bind:value={values[f.key]}>
              {#each f.options ?? [] as o}
                <option value={o.value}>{o.label}</option>
              {/each}
            </select>
          {:else if f.kind === "secret" || f.kind === "password"}
            <div class="input-group">
              <input
                id="f-{f.key}"
                class="input mono"
                class:invalid={touched && errors[f.key]}
                type={reveal[f.key] ? "text" : "password"}
                bind:value={values[f.key]}
                placeholder={f.placeholder ?? ""}
                autocomplete="off"
                spellcheck="false"
              />
              <button type="button" class="btn icon" aria-label={reveal[f.key] ? "Masquer" : "Afficher"} onclick={() => (reveal[f.key] = !reveal[f.key])}>
                <Icon name={reveal[f.key] ? "eye-off" : "eye"} size={17} />
              </button>
              {#if f.generate}
                <button type="button" class="btn" onclick={() => generate(f.key, f.generate!)}><Icon name="wand" size={16} /> Générer</button>
              {/if}
            </div>
          {:else}
            <input
              id="f-{f.key}"
              class="input"
              class:mono={f.kind === "number" || f.kind === "url"}
              class:invalid={touched && errors[f.key]}
              type={f.kind === "email" ? "email" : f.kind === "url" ? "url" : "text"}
              inputmode={f.kind === "number" ? "numeric" : undefined}
              bind:value={values[f.key]}
              placeholder={f.placeholder ?? f.default ?? ""}
              autocapitalize="off"
              spellcheck="false"
            />
          {/if}
          {#if touched && errors[f.key]}
            <span class="err">{errors[f.key]}</span>
          {:else if f.help}
            <span class="help">{f.help}</span>
          {/if}
        </div>
      {/each}

      <details class="advanced">
        <summary>Dossier d'installation</summary>
        <div class="field">
          <div class="input-group">
            <input class="input mono" bind:value={dir} spellcheck="false" aria-label="Dossier d'installation" />
            {#if isTauri}<button type="button" class="btn" onclick={chooseDir}><Icon name="folder" size={16} /> Choisir</button>{/if}
          </div>
          <span class="help">Code, configuration (.env) et sauvegardes de l'application. Les données Docker sont conservées en cas de désinstallation.</span>
        </div>
      </details>

      {#if tdef.duration}
        <p class="duration muted"><Icon name="clock" size={15} /> Durée estimée : {tdef.duration}. Vous pouvez fermer cette fenêtre, l'installation continue.</p>
      {/if}
      {#if error}
        <div class="callout danger"><Icon name="alert" size={18} /><span class="selectable">{error}</span></div>
      {/if}
    </form>
  {/if}

  {#snippet footer()}
    {#if step === 1}
      <button class="btn" onclick={close}>Annuler</button>
      <button class="btn primary" disabled={!tdef || !ready || alreadyThere} onclick={next}>
        Continuer <Icon name="chevron" size={16} />
      </button>
    {:else}
      <button class="btn" onclick={() => (step = 1)}><Icon name="back" size={16} /> Retour</button>
      <button class="btn brand" type="submit" form="install-form" disabled={submitting}>
        {#if submitting}<Icon name="loader" size={16} class="spin" />{:else}<Icon name="download" size={16} />{/if}
        Installer {app.name}
      </button>
    {/if}
  {/snippet}
</Modal>


<style>
  .mini {
    width: 44px;
    height: 44px;
    object-fit: contain;
    border-radius: 12px;
    background: color-mix(in srgb, var(--app) 14%, transparent);
    padding: 3px;
  }
  .here {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    border-radius: 16px;
    border: 1.5px solid var(--primary);
    background: color-mix(in srgb, var(--primary) 5%, var(--surface));
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 12%, transparent);
  }
  .t-icon {
    display: grid;
    place-items: center;
    width: 44px;
    height: 44px;
    border-radius: 13px;
    background: var(--surface-2);
    color: var(--primary);
    flex: none;
  }
  .t-text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    font-size: 13px;
  }
  .t-text strong {
    font-size: 15px;
  }
  .check {
    margin-top: 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .check-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .check + .callout {
    margin-top: 12px;
  }
  .prepare {
    margin-top: 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .team {
    margin-top: 18px;
  }
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .notes {
    margin: 0;
    padding-left: 16px;
    display: grid;
    gap: 4px;
  }
  .advanced summary {
    cursor: pointer;
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text-2);
    margin-bottom: 10px;
  }
  .duration {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 13px;
  }
</style>
