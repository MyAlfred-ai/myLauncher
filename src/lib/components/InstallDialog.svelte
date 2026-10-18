<script lang="ts">
  import { untrack } from "svelte";
  import { appIcon, launcher } from "../launcher.svelte";
  import { api, isTauri, pickDir } from "../api";
  import { initialValues, isActive, validateField } from "../format";
  import type { HostCheck, TargetDef } from "../types";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";
  import ServerForm from "./ServerForm.svelte";
  import HostStatus from "./HostStatus.svelte";

  let { appId, initialTarget }: { appId: string; initialTarget?: string } = $props();

  const app = $derived(launcher.app(appId)!);
  const servers = $derived(launcher.overview?.servers ?? []);
  const installs = $derived(launcher.installsFor(appId));

  let step = $state<1 | 2>(1);
  let target = $state<string>(
    untrack(() => {
      if (initialTarget) return initialTarget;
      const a = launcher.app(appId);
      const taken = new Set(launcher.installsFor(appId).filter((i) => i.state !== "failed").map((i) => i.target));
      const options = [
        ...(a?.targets.local ? ["local"] : []),
        ...(a?.targets.server ? (launcher.overview?.servers ?? []).map((s) => s.id) : []),
      ];
      return options.find((t) => !taken.has(t)) ?? "";
    }),
  );
  let check = $state<HostCheck | null>(null);
  let checking = $state(false);
  let addServer = $state(false);
  let dir = $state("");
  let values = $state<Record<string, string>>({});
  let reveal = $state<Record<string, boolean>>({});
  let touched = $state(false);
  let submitting = $state(false);
  let error = $state<string | null>(null);

  const tdef: TargetDef | null = $derived(
    target === "local" ? (app.targets.local ?? null) : target ? (app.targets.server ?? null) : null,
  );
  const fields = $derived((tdef?.fields ?? []).filter((f) => isActive(f, values)));
  const errors = $derived(Object.fromEntries(fields.map((f) => [f.key, validateField(f, values)])));
  const hasErrors = $derived(Object.values(errors).some(Boolean));
  const ready = $derived(!!check?.reachable && !!check.docker && !!check.compose && check.daemon && !!check.git);
  const alreadyThere = (t: string) => installs.some((i) => i.target === t && i.state !== "failed");

  function close() {
    launcher.installing = null;
  }

  async function runCheck() {
    if (!target) return;
    checking = true;
    check = null;
    try {
      check = await api.checkHost(target);
    } catch (e) {
      check = { reachable: false, os: "", git: "", docker: "", compose: "", daemon: false, error: String(e) };
    } finally {
      checking = false;
    }
  }

  $effect(() => {
    if (target && step === 1) runCheck();
  });

  async function next() {
    if (!tdef) return;
    const prev = installs.find((i) => i.target === target)?.values ?? {};
    values = initialValues(tdef.fields, prev);
    dir = installs.find((i) => i.target === target)?.dir ?? (await api.defaultInstallDir(appId, target));
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
      const opId = await api.install({ appId, target, dir, values: payload });
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
  subtitle={step === 1 ? "Où voulez-vous l'installer ?" : target === "local" ? "Sur ce poste" : servers.find((s) => s.id === target)?.name}
  width={640}
  onclose={close}
>
  {#snippet header()}
    <img class="mini" src={appIcon(app.icon)} alt="" width="44" height="44" style:--app={app.color} />
  {/snippet}

  {#if step === 1}
    <div class="targets" role="radiogroup" aria-label="Destination">
      <button
        role="radio"
        aria-checked={target === "local"}
        class="target"
        class:selected={target === "local"}
        disabled={!app.targets.local}
        onclick={() => (target = "local")}
      >
        <span class="t-icon"><Icon name="laptop" size={22} /></span>
        <span class="t-text">
          <strong>Ce poste</strong>
          <span class="muted">{app.targets.local ? "Pour essayer ou travailler seul" : "Non disponible pour cette application"}</span>
        </span>
        {#if app.targets.local?.experimental}<span class="pill warning">Expérimental</span>{/if}
        {#if alreadyThere("local")}<span class="pill success">Déjà installé</span>{/if}
      </button>
      {#each servers as s (s.id)}
        <button
          role="radio"
          aria-checked={target === s.id}
          class="target"
          class:selected={target === s.id}
          disabled={!app.targets.server}
          onclick={() => (target = s.id)}
        >
          <span class="t-icon"><Icon name="server" size={22} /></span>
          <span class="t-text">
            <strong>{s.name}</strong>
            <span class="muted mono">{s.user}@{s.host}</span>
          </span>
          {#if app.targets.server?.experimental}<span class="pill warning">Expérimental</span>{/if}
          {#if alreadyThere(s.id)}<span class="pill success">Déjà installé</span>{/if}
        </button>
      {/each}
      <button class="target add" onclick={() => (addServer = true)} disabled={!app.targets.server}>
        <span class="t-icon"><Icon name="plus" size={22} /></span>
        <span class="t-text">
          <strong>Ajouter un serveur</strong>
          <span class="muted">Pour toute l'équipe, avec un nom de domaine</span>
        </span>
      </button>
    </div>

    {#if target}
      <div class="check">
        <div class="check-head">
          <span class="label">Machine</span>
          <button class="btn ghost sm" onclick={runCheck} disabled={checking}><Icon name="refresh" size={14} /> Vérifier à nouveau</button>
        </div>
        <HostStatus {check} loading={checking} local={target === "local"} />
      </div>
    {/if}

    {#if target && alreadyThere(target)}
      <div class="callout warning">
        <Icon name="info" size={18} />
        <span>{app.name} est déjà installé ici. Utilisez « Mettre à jour » depuis sa fiche.</span>
      </div>
    {/if}
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
              type={f.kind === "email" ? "email" : f.kind === "number" ? "number" : f.kind === "url" ? "url" : "text"}
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
            {#if target === "local" && isTauri}<button type="button" class="btn" onclick={chooseDir}><Icon name="folder" size={16} /> Choisir</button>{/if}
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
      <button class="btn primary" disabled={!tdef || !ready || alreadyThere(target)} onclick={next}>
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

{#if addServer}
  <ServerForm onclose={() => (addServer = false)} onsaved={(s) => (target = s.id)} />
{/if}

<style>
  .mini {
    width: 44px;
    height: 44px;
    object-fit: contain;
    border-radius: 12px;
    background: color-mix(in srgb, var(--app) 14%, transparent);
    padding: 3px;
  }
  .targets {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .target {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    border-radius: 16px;
    border: 1.5px solid var(--border);
    background: var(--surface);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 0.15s,
      background 0.15s,
      box-shadow 0.15s;
  }
  .target:hover:not(:disabled) {
    border-color: var(--border-strong);
  }
  .target.selected {
    border-color: var(--primary);
    background: color-mix(in srgb, var(--primary) 5%, var(--surface));
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--primary) 12%, transparent);
  }
  .target:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .target.add {
    border-style: dashed;
    background: transparent;
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
