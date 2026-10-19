<script lang="ts">
  import { onMount, tick } from "svelte";
  import { appIcon, launcher } from "../launcher.svelte";
  import { api, isTauri, openUrl, revealDir } from "../api";
  import { actionLabels, formatDate } from "../format";
  import Icon from "../components/Icon.svelte";

  let { id }: { id: string } = $props();

  const op = $derived(launcher.ops[id]);
  const app = $derived(op ? launcher.app(op.appId) : undefined);
  const lines = $derived(launcher.logs[id] ?? []);
  const inst = $derived(op ? launcher.overview?.installations.find((i) => i.id === op.installationId) : undefined);
  const running = $derived(op?.status === "running");

  let consoleEl: HTMLPreElement | undefined = $state();
  let follow = $state(true);
  let showLog = $state(true);
  let cancelling = $state(false);

  onMount(() => {
    launcher.loadLog(id);
    showLog = window.innerWidth > 760 || op?.status === "failed";
  });

  $effect(() => {
    void lines.length;
    if (follow && consoleEl) {
      tick().then(() => consoleEl && (consoleEl.scrollTop = consoleEl.scrollHeight));
    }
  });

  function onScroll() {
    if (!consoleEl) return;
    follow = consoleEl.scrollHeight - consoleEl.scrollTop - consoleEl.clientHeight < 40;
  }

  async function cancel() {
    cancelling = true;
    await api.cancelOp(id);
  }

  function stepState(i: number): "done" | "current" | "failed" | "pending" {
    if (!op) return "pending";
    if (op.status === "success") return "done";
    if (i < op.currentStep) return "done";
    if (i === op.currentStep) return op.status === "running" ? "current" : op.status === "failed" ? "failed" : "pending";
    return "pending";
  }

  function copyLog() {
    navigator.clipboard.writeText(lines.map((l) => l.text).join("\n"));
    launcher.toast("Journal copié", "success");
  }
</script>

{#if !op || !app}
  <div class="page">
    <p class="muted">Opération introuvable.</p>
    <button class="btn" onclick={() => launcher.go({ name: "activity" })}>Retour à l'activité</button>
  </div>
{:else}
  <div class="page" style:--app={app.color}>
    <button class="back btn ghost sm" onclick={() => launcher.go({ name: "app", id: app.id, tab: "installs" })}>
      <Icon name="back" size={16} /> {app.name}
    </button>

    <header class="head card">
      <img src={appIcon(app.icon)} alt="" width="72" height="72" />
      <div class="head-text">
        <h1>{actionLabels[op.action]} de {app.name}</h1>
        <p class="muted">{op.targetLabel} · démarrée {formatDate(op.startedAt)}</p>
      </div>
      <div class="head-status">
        {#if running}
          <span class="pill brand"><Icon name="loader" size={13} class="spin" /> En cours</span>
          <button class="btn sm danger" onclick={cancel} disabled={cancelling}>{cancelling ? "Annulation…" : "Annuler"}</button>
        {:else if op.status === "success"}
          <span class="pill success"><Icon name="check" size={13} stroke={2.6} /> Terminée</span>
        {:else if op.status === "cancelled"}
          <span class="pill">Annulée</span>
        {:else}
          <span class="pill danger"><Icon name="x" size={13} stroke={2.6} /> Échec</span>
        {/if}
      </div>
    </header>

    {#if op.status === "success" && (op.action === "install" || op.action === "update")}
      <div class="result success">
        <div class="result-icon"><Icon name="check" size={26} stroke={2.6} /></div>
        <div class="result-text">
          <strong>{app.name} est {op.action === "install" ? "installé" : "à jour"} !</strong>
          <span>
            {#if op.action === "install"}
              Connectez-vous avec les identifiants saisis lors de l'installation.
            {:else}
              La nouvelle version est en service.
            {/if}
          </span>
        </div>
        {#if inst?.url}
          <button class="btn" onclick={() => openUrl(inst.url!)}><Icon name="external" size={16} /> Se connecter à {app.name}</button>
        {/if}
      </div>
    {:else if op.status === "failed"}
      <div class="result failed">
        <div class="result-icon"><Icon name="alert" size={24} /></div>
        <div class="result-text">
          <strong>L'opération n'a pas abouti</strong>
          <span class="selectable">{op.error}</span>
        </div>
        {#if op.action === "install"}
          <button class="btn" onclick={() => (launcher.installing = { appId: app.id, target: inst?.target })}><Icon name="refresh" size={16} /> Réessayer</button>
        {:else if inst}
          <button class="btn" onclick={() => launcher.run(inst.id, op.action as "update")}><Icon name="refresh" size={16} /> Réessayer</button>
        {/if}
      </div>
    {/if}

    {#each op.warnings as w}
      <div class="callout warning"><Icon name="alert" size={18} /><span>{w}</span></div>
    {/each}

    <div class="body">
      <ol class="steps card">
        {#each op.steps as s, i}
          {@const st = stepState(i)}
          <li class={st}>
            <span class="bullet">
              {#if st === "done"}<Icon name="check" size={14} stroke={3} />
              {:else if st === "current"}<Icon name="loader" size={14} stroke={2.6} class="spin" />
              {:else if st === "failed"}<Icon name="x" size={14} stroke={3} />
              {:else}{i + 1}{/if}
            </span>
            <span class="step-label">{s}</span>
          </li>
        {/each}
      </ol>

      <section class="log card">
        <div class="log-head">
          <button class="toggle" onclick={() => (showLog = !showLog)} aria-expanded={showLog}>
            <Icon name="terminal" size={16} /> Journal détaillé
            <Icon name="chevron-down" size={15} class={showLog ? "" : "rot"} />
          </button>
          <div class="log-actions">
            <button class="btn ghost sm" onclick={copyLog} disabled={!lines.length}><Icon name="copy" size={14} /> Copier</button>
            {#if isTauri && op.logFile}
              <button class="btn ghost sm" onclick={() => revealDir(op.logFile!)}><Icon name="file" size={14} /> Fichier</button>
            {/if}
          </div>
        </div>
        {#if showLog}
          <pre class="console" bind:this={consoleEl} onscroll={onScroll}>{#each lines as l, i (i)}<span class="ln {l.kind}">{l.kind === "step" ? `▸ ${l.text}` : l.text}
</span>{/each}{#if running}<span class="cursor">▍</span>{/if}</pre>
        {/if}
      </section>
    </div>
  </div>
{/if}

<style>
  .page {
    max-width: 1180px;
    margin: 0 auto;
    padding: 18px 32px 40px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .back {
    align-self: flex-start;
    margin-left: -8px;
    color: var(--text-2);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 16px 20px;
    background:
      radial-gradient(50% 140% at 0% 50%, color-mix(in srgb, var(--app) 20%, transparent), transparent 70%),
      var(--surface);
  }
  .head img {
    width: 72px;
    height: 72px;
    object-fit: contain;
    filter: drop-shadow(0 8px 10px rgb(30 10 60 / 22%));
  }
  .head-text {
    flex: 1;
    min-width: 0;
  }
  h1 {
    font-size: 22px;
  }
  .head-text p {
    font-size: 13.5px;
    margin-top: 4px;
  }
  .head-status {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .result {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px 18px;
    border-radius: var(--radius-lg);
    animation: rise 0.35s ease both;
  }
  .result.success {
    background: var(--success-soft);
  }
  .result.failed {
    background: var(--danger-soft);
  }
  .result-icon {
    display: grid;
    place-items: center;
    width: 46px;
    height: 46px;
    border-radius: 50%;
    flex: none;
    color: #fff;
  }
  .success .result-icon {
    background: var(--success);
  }
  .failed .result-icon {
    background: var(--danger);
  }
  .result-text {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    font-size: 14px;
    color: var(--text-2);
  }
  .result-text strong {
    font-size: 16px;
    color: var(--text);
  }
  .body {
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .steps li {
    position: relative;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 8px;
    border-radius: 10px;
    font-size: 14px;
    color: var(--muted);
  }
  .steps li:not(:last-child)::after {
    content: "";
    position: absolute;
    left: 21px;
    top: 34px;
    height: 14px;
    width: 2px;
    border-radius: 2px;
    background: var(--border);
  }
  .steps li.done::after {
    background: var(--success);
  }
  .bullet {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--surface-2);
    font-size: 12px;
    font-weight: 600;
    flex: none;
  }
  .done {
    color: var(--text-2);
  }
  .done .bullet {
    background: var(--success);
    color: #fff;
  }
  .current {
    color: var(--text);
    font-weight: 500;
    background: var(--primary-soft);
  }
  .current .bullet {
    background: var(--primary);
    color: #fff;
  }
  .failed {
    color: var(--danger);
    font-weight: 500;
  }
  .failed .bullet {
    background: var(--danger);
    color: #fff;
  }
  .log {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .log-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px 8px 14px;
    border-bottom: 1px solid var(--border);
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    border: none;
    background: none;
    font-weight: 500;
    font-size: 14px;
    cursor: pointer;
    padding: 4px 0;
  }
  .toggle :global(.rot) {
    transform: rotate(-90deg);
  }
  .log-actions {
    display: flex;
    gap: 4px;
  }
  .console {
    margin: 0;
    height: min(56vh, 560px);
    overflow: auto;
    padding: 14px 16px;
    background: var(--console-bg);
    color: var(--console-text);
    font: 12px/1.6 var(--mono);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .ln.step {
    color: #c4b5fd;
    font-weight: 700;
  }
  .ln.warn {
    color: #fbbf24;
  }
  .ln.error {
    color: #fb7185;
    font-weight: 600;
  }
  .ln.err {
    color: #d8cfff;
    opacity: 0.85;
  }
  .cursor {
    animation: blink 1s steps(2) infinite;
    color: #c4b5fd;
  }
  @keyframes blink {
    to {
      opacity: 0;
    }
  }
  @media (max-width: 900px) {
    .body {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 760px) {
    .page {
      padding: 12px 16px 32px;
    }
    .head {
      flex-wrap: wrap;
    }
    .head img {
      width: 52px;
      height: 52px;
    }
    .head-status {
      width: 100%;
    }
    .result {
      flex-wrap: wrap;
    }
    .result .btn {
      width: 100%;
    }
  }
</style>
