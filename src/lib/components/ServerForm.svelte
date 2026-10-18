<script lang="ts">
  import { untrack } from "svelte";
  import { api, isTauri, pickFile } from "../api";
  import { launcher } from "../launcher.svelte";
  import type { Server } from "../types";
  import Icon from "./Icon.svelte";
  import Modal from "./Modal.svelte";

  let { server, onclose, onsaved }: { server?: Server; onclose: () => void; onsaved?: (s: Server) => void } = $props();

  // The form edits a copy taken when the dialog opens.
  let form = $state<Server>(
    untrack(() =>
      server
        ? { ...server }
        : { id: "", name: "", host: "", port: 22, user: "", identityFile: "", baseDir: "~/myalfred" },
    ),
  );
  let saving = $state(false);
  let error = $state<string | null>(null);

  async function pickKey() {
    const f = await pickFile("Clé privée SSH");
    if (f) form.identityFile = f;
  }

  async function save(e: Event) {
    e.preventDefault();
    saving = true;
    error = null;
    try {
      const saved = await api.saveServer({ ...form, port: Number(form.port) || 22 });
      launcher.toast(`Serveur « ${saved.name} » enregistré`, "success");
      onsaved?.(saved);
      onclose();
    } catch (err) {
      error = String(err).replace(/^Error:\s*/, "");
    } finally {
      saving = false;
    }
  }
</script>

<Modal title={server ? "Modifier le serveur" : "Ajouter un serveur"} subtitle="Connexion SSH par clé, comme dans votre terminal" width={560} {onclose}>
  <form id="server-form" class="form" onsubmit={save}>
    <div class="field">
      <label for="s-name">Nom</label>
      <input id="s-name" class="input" bind:value={form.name} placeholder="VPS de production" required />
    </div>
    <div class="grid2">
      <div class="field grow">
        <label for="s-host">Adresse</label>
        <input id="s-host" class="input mono" bind:value={form.host} placeholder="vps.entreprise.fr" required autocapitalize="off" spellcheck="false" />
      </div>
      <div class="field port">
        <label for="s-port">Port</label>
        <input id="s-port" class="input mono" type="number" min="1" max="65535" bind:value={form.port} />
      </div>
    </div>
    <div class="field">
      <label for="s-user">Utilisateur</label>
      <input id="s-user" class="input mono" bind:value={form.user} placeholder="ubuntu" required autocapitalize="off" spellcheck="false" />
      <span class="help">Il doit pouvoir utiliser Docker (groupe « docker »).</span>
    </div>
    <div class="field">
      <label for="s-key">Clé privée <span class="muted">(facultatif)</span></label>
      <div class="input-group">
        <input id="s-key" class="input mono" bind:value={form.identityFile} placeholder="Agent SSH ou ~/.ssh/config par défaut" spellcheck="false" />
        {#if isTauri}<button type="button" class="btn" onclick={pickKey}><Icon name="key" size={16} /> Choisir</button>{/if}
      </div>
    </div>
    <div class="field">
      <label for="s-dir">Dossier des applications</label>
      <input id="s-dir" class="input mono" bind:value={form.baseDir} spellcheck="false" />
      <span class="help">Chaque application est installée dans un sous-dossier.</span>
    </div>
    <div class="callout">
      <Icon name="shield" size={18} />
      <span>Le launcher utilise votre client <span class="mono">ssh</span> : aucun mot de passe n'est demandé ni stocké. À la première connexion, l'empreinte du serveur est mémorisée.</span>
    </div>
    {#if error}
      <div class="callout danger"><Icon name="alert" size={18} /><span>{error}</span></div>
    {/if}
  </form>
  {#snippet footer()}
    <button class="btn" type="button" onclick={onclose}>Annuler</button>
    <button class="btn primary" type="submit" form="server-form" disabled={saving}>
      {#if saving}<Icon name="loader" size={16} class="spin" />{/if} Enregistrer
    </button>
  {/snippet}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .grid2 {
    display: flex;
    gap: 12px;
  }
  .grow {
    flex: 1;
  }
  .port {
    width: 110px;
  }
</style>
