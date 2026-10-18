<script lang="ts">
  import { appIcon, launcher } from "../launcher.svelte";
  import { actionLabels, relativeTime } from "../format";
  import Icon from "../components/Icon.svelte";

  const ops = $derived(Object.values(launcher.ops).sort((a, b) => b.startedAt.localeCompare(a.startedAt)));
</script>

<div class="page">
  <header>
    <h1>Activité</h1>
    <p class="muted">Installations, mises à jour et démarrages de cette session, avec leur journal.</p>
  </header>

  {#if ops.length}
    <ul class="list card">
      {#each ops as op (op.id)}
        {@const app = launcher.app(op.appId)}
        <li>
          <button onclick={() => launcher.go({ name: "op", id: op.id })}>
            <img src={app ? appIcon(app.icon) : ""} alt="" width="40" height="40" />
            <span class="text">
              <strong>{actionLabels[op.action]} de {app?.name ?? op.appId}</strong>
              <span class="muted">{op.targetLabel} · {relativeTime(op.startedAt)}</span>
            </span>
            {#if op.status === "running"}
              <span class="pill brand"><Icon name="loader" size={12} class="spin" /> {op.steps[op.currentStep] ?? "En cours"}</span>
            {:else if op.status === "success"}
              <span class="pill success"><Icon name="check" size={12} stroke={2.6} /> Terminée</span>
            {:else if op.status === "cancelled"}
              <span class="pill">Annulée</span>
            {:else}
              <span class="pill danger"><Icon name="x" size={12} stroke={2.6} /> Échec</span>
            {/if}
            <Icon name="chevron" size={16} class="chev" />
          </button>
        </li>
      {/each}
    </ul>
  {:else}
    <div class="empty card">
      <Icon name="activity" size={28} />
      <h3>Aucune activité pour l'instant</h3>
      <p class="muted">Les installations et mises à jour apparaîtront ici, avec leur progression en direct.</p>
    </div>
  {/if}
</div>

<style>
  .page {
    max-width: 960px;
    margin: 0 auto;
    padding: 28px 32px 40px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }
  h1 {
    font-size: 28px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  header p {
    margin-top: 4px;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 6px;
  }
  .list li + li {
    border-top: 1px solid var(--border);
  }
  .list button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px;
    border: none;
    background: none;
    border-radius: 12px;
    text-align: left;
    cursor: pointer;
  }
  .list button:hover {
    background: var(--surface-2);
  }
  img {
    width: 40px;
    height: 40px;
    object-fit: contain;
  }
  .text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    font-size: 13px;
  }
  .text strong {
    font-size: 14.5px;
  }
  .list :global(.chev) {
    color: var(--muted);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 48px 24px;
    text-align: center;
    color: var(--muted);
  }
  .empty h3 {
    color: var(--text);
  }
  @media (max-width: 760px) {
    .page {
      padding: 20px 16px 32px;
    }
    .list .pill {
      display: none;
    }
  }
</style>
