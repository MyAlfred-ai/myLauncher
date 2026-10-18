<script lang="ts">
  import { launcher } from "../launcher.svelte";
  import { api } from "../api";
  import type { HostCheck, Server } from "../types";
  import Icon from "../components/Icon.svelte";
  import ServerForm from "../components/ServerForm.svelte";
  import HostStatus from "../components/HostStatus.svelte";
  import Modal from "../components/Modal.svelte";

  const servers = $derived(launcher.overview?.servers ?? []);
  let editing = $state<Server | null | undefined>(undefined);
  let checks = $state<Record<string, HostCheck | null>>({});
  let checking = $state<Record<string, boolean>>({});
  let removing = $state<Server | null>(null);

  const appsOn = (id: string) =>
    (launcher.overview?.installations ?? []).filter((i) => i.target === id).map((i) => launcher.app(i.appId)?.name ?? i.appId);

  async function test(s: Server) {
    checking[s.id] = true;
    try {
      checks[s.id] = await api.checkHost(s.id);
    } catch (e) {
      checks[s.id] = { reachable: false, os: "", git: "", docker: "", compose: "", daemon: false, error: String(e) };
    } finally {
      checking[s.id] = false;
    }
  }

  async function remove() {
    const s = removing!;
    removing = null;
    try {
      await api.deleteServer(s.id);
      launcher.toast(`Serveur « ${s.name} » supprimé`, "success");
    } catch (e) {
      launcher.toast(String(e), "error");
    }
  }
</script>

<div class="page">
  <header>
    <div>
      <h1>Serveurs</h1>
      <p class="muted">Installez les applications sur vos serveurs Linux via SSH, pour toute l'équipe.</p>
    </div>
    <button class="btn primary" onclick={() => (editing = null)}><Icon name="plus" size={17} /> Ajouter un serveur</button>
  </header>

  {#if servers.length}
    <div class="grid">
      {#each servers as s (s.id)}
        {@const apps = appsOn(s.id)}
        <article class="card srv">
          <div class="srv-head">
            <div class="srv-icon"><Icon name="server" size={22} /></div>
            <div class="srv-text">
              <strong>{s.name}</strong>
              <span class="muted mono selectable">{s.user}@{s.host}{s.port !== 22 ? `:${s.port}` : ""}</span>
            </div>
          </div>
          <div class="srv-meta">
            <span class="pill"><Icon name="folder" size={13} /> <span class="mono">{s.baseDir}</span></span>
            {#if s.identityFile}<span class="pill"><Icon name="key" size={13} /> Clé dédiée</span>{/if}
            {#each apps as a}<span class="pill brand">{a}</span>{/each}
          </div>
          {#if checks[s.id] !== undefined || checking[s.id]}
            <HostStatus check={checks[s.id] ?? null} loading={!!checking[s.id]} local={false} />
          {/if}
          <div class="srv-actions">
            <button class="btn sm" onclick={() => test(s)} disabled={checking[s.id]}><Icon name="plug" size={15} /> Tester la connexion</button>
            <button class="btn sm ghost" onclick={() => (editing = s)}>Modifier</button>
            <button class="btn sm ghost danger icon" aria-label="Supprimer" onclick={() => (removing = s)}><Icon name="trash" size={16} /></button>
          </div>
        </article>
      {/each}
    </div>
  {:else}
    <div class="empty card">
      <div class="empty-icon"><Icon name="server" size={30} /></div>
      <h3>Aucun serveur</h3>
      <p class="muted">Ajoutez un serveur Linux accessible en SSH (clé) avec Docker installé. Le launcher y déploie et met à jour les applications.</p>
      <button class="btn primary" onclick={() => (editing = null)}><Icon name="plus" size={17} /> Ajouter un serveur</button>
    </div>
  {/if}
</div>

{#if editing !== undefined}
  <ServerForm server={editing ?? undefined} onclose={() => (editing = undefined)} onsaved={(s) => test(s)} />
{/if}

{#if removing}
  <Modal title="Supprimer « {removing.name} » ?" width={460} onclose={() => (removing = null)}>
    <p>Le serveur est retiré du launcher. Rien n'est modifié sur la machine elle-même.</p>
    {#snippet footer()}
      <button class="btn" onclick={() => (removing = null)}>Annuler</button>
      <button class="btn danger solid" onclick={remove}>Supprimer</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .page {
    max-width: 1100px;
    margin: 0 auto;
    padding: 28px 32px 40px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  header {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  h1 {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  header p {
    margin-top: 4px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: 16px;
  }
  .srv {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px;
  }
  .srv-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .srv-icon,
  .empty-icon {
    display: grid;
    place-items: center;
    width: 46px;
    height: 46px;
    border-radius: 14px;
    background: var(--primary-soft);
    color: var(--primary);
    flex: none;
  }
  .srv-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
    font-size: 13px;
  }
  .srv-text strong {
    font-size: 16px;
  }
  .srv-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .srv-actions {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .srv-actions .icon {
    margin-left: auto;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 48px 24px;
    text-align: center;
  }
  .empty p {
    max-width: 460px;
  }
  @media (max-width: 760px) {
    .page {
      padding: 20px 16px 32px;
    }
    header .btn {
      width: 100%;
    }
    .grid {
      grid-template-columns: 1fr;
    }
  }
</style>
