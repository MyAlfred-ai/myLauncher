<script lang="ts">
  import { onMount } from "svelte";
  import { api, on, openUrl } from "../api";
  import type { DockerSetupProgress, HostCheck, TargetDef } from "../types";
  import Icon from "./Icon.svelte";

  // `check` is shared with the install dialog, which enables "Continuer"
  // once Docker answers.
  let {
    check = $bindable(null),
    target,
    appName,
  }: { check: HostCheck | null; target: TargetDef | null; appName: string } = $props();

  type Phase = "checking" | "starting" | "installing" | "idle";
  let phase = $state<Phase>("checking");
  let error = $state<string | null>(null);
  let accepted = $state(false);
  let progress = $state<DockerSetupProgress | null>(null);
  let autoStarted = false;

  const GB = 1024 ** 3;
  const gb = (bytes: number) => (bytes / GB).toLocaleString("fr-FR", { maximumFractionDigits: 1 });
  const mb = (bytes: number) => Math.round(bytes / 1024 ** 2).toLocaleString("fr-FR");

  const installed = $derived(!!check && (!!check.docker || !!check.dockerApp));
  const running = $derived(!!check?.reachable && check.daemon);
  const name = $derived(check?.dockerApp ?? "Docker");
  const lowMemory = $derived(
    running && !!target?.minMemoryGb && check!.memTotal > 0 && check!.memTotal < target.minMemoryGb * GB * 0.9,
  );
  const lowDisk = $derived(
    !!target?.minDiskGb && !!check?.diskFree && check.diskFree < target.minDiskGb * GB,
  );

  async function refresh() {
    phase = "checking";
    error = null;
    try {
      check = await api.checkHost();
    } catch (e) {
      check = null;
      error = String(e).replace(/^Error:\s*/, "");
    }
    phase = "idle";
    // Installed but stopped: start it right away, once, without asking.
    if (check?.reachable && !check.daemon && (check.docker || check.dockerApp) && !autoStarted) {
      autoStarted = true;
      await start();
    }
  }

  async function start() {
    phase = "starting";
    error = null;
    try {
      check = await api.dockerStart();
    } catch (e) {
      error = String(e).replace(/^Error:\s*/, "");
    }
    phase = "idle";
  }

  async function install() {
    phase = "installing";
    error = null;
    progress = { phase: "download", received: 0, total: 0 };
    try {
      check = await api.dockerInstall();
    } catch (e) {
      error = String(e).replace(/^Error:\s*/, "");
    }
    progress = null;
    phase = "idle";
  }

  onMount(() => {
    refresh();
    let off: (() => void) | undefined;
    on<DockerSetupProgress>("docker-setup", (p) => (progress = p)).then((u) => (off = u));
    return () => off?.();
  });

  const pct = $derived(progress && progress.total ? Math.round((progress.received / progress.total) * 100) : 0);
</script>

<div class="docker" class:ok={running && !lowMemory && !lowDisk}>
  {#if phase === "checking"}
    <p class="line muted"><Icon name="loader" size={16} class="spin" /> Vérification du poste…</p>
  {:else if phase === "starting"}
    <div class="line">
      <Icon name="loader" size={18} class="spin" />
      <div>
        <strong>Démarrage de {name}…</strong>
        <p class="muted">Cela peut prendre jusqu'à 3 minutes. Rien à faire de votre côté.</p>
      </div>
    </div>
  {:else if phase === "installing" && progress}
    <div class="setup">
      <strong>
        {#if progress.phase === "download"}Téléchargement de Docker Desktop…
        {:else if progress.phase === "install"}Installation de Docker Desktop…
        {:else if progress.phase === "start"}Premier démarrage de Docker…
        {:else}Docker est prêt.{/if}
      </strong>
      {#if progress.phase === "download"}
        <div class="bar"><div style:width="{pct}%"></div></div>
        <div class="row muted">
          <span>{progress.total ? `${mb(progress.received)} Mo sur ${mb(progress.total)} Mo` : "Connexion…"}</span>
          <button class="btn ghost sm" onclick={() => api.dockerInstallCancel()}>Annuler</button>
        </div>
      {:else if progress.phase === "install"}
        <div class="bar indeterminate"><div></div></div>
        <p class="muted">Confirmez avec le mot de passe administrateur de l'ordinateur quand il vous est demandé.</p>
      {:else if progress.phase === "start"}
        <div class="bar indeterminate"><div></div></div>
        <p class="muted">Docker prépare son environnement : quelques minutes la première fois.</p>
      {/if}
    </div>
  {:else if check && !check.reachable}
    <div class="line bad">
      <Icon name="x-circle" size={18} />
      <span>{check.error}</span>
    </div>
  {:else if check && running}
    <ul class="facts">
      <li class="good"><Icon name="check-circle" size={16} /> {check.engine || name} est démarré{check.compose ? "" : " (Compose v2 manquant)"}</li>
      {#if check.memTotal}
        <li class:warn={lowMemory} class:good={!lowMemory}>
          <Icon name={lowMemory ? "alert" : "check-circle"} size={16} />
          Mémoire de Docker : {gb(check.memTotal)} Go{#if lowMemory}, {target?.minMemoryGb} Go conseillés pour {appName}{/if}
        </li>
      {/if}
      {#if check.diskFree}
        <li class:warn={lowDisk} class:good={!lowDisk}>
          <Icon name={lowDisk ? "alert" : "check-circle"} size={16} />
          Espace libre : {gb(check.diskFree)} Go{#if lowDisk}, {target?.minDiskGb} Go nécessaires{/if}
        </li>
      {/if}
    </ul>
    {#if lowMemory}
      <p class="hint">Augmentez la mémoire dans {name} → Réglages → Ressources, puis vérifiez à nouveau. Vous pouvez aussi continuer : {appName} sera plus lent.</p>
    {/if}
    {#if lowDisk}
      <p class="hint">Libérez de l'espace disque avant d'installer : les images Docker et les données de {appName} en ont besoin.</p>
    {/if}
  {:else if check && installed}
    <div class="line bad">
      <Icon name="alert" size={18} />
      <div>
        <strong>{name} ne répond pas.</strong>
        <p>Ouvrez-le depuis vos applications et terminez sa configuration (première ouverture, conditions d'utilisation), puis réessayez.</p>
      </div>
    </div>
    <button class="btn primary sm" onclick={start}><Icon name="play" size={13} /> Démarrer {name}</button>
  {:else if check}
    <div class="need">
      <div class="need-head">
        <span class="n-icon"><Icon name="package" size={20} /></span>
        <div>
          <strong>Docker est nécessaire</strong>
          <p>{appName} fonctionne dans Docker, le logiciel qui fait tourner les applications sur ce poste. Il n'est pas installé.</p>
        </div>
      </div>
      {#if check.canInstallDocker}
        <ol class="how">
          <li>Le launcher télécharge Docker Desktop (environ 600 Mo) depuis le site officiel et vérifie sa signature.</li>
          <li>L'ordinateur vous demande une fois votre mot de passe administrateur.</li>
          <li>Docker démarre, puis l'installation de {appName} continue.</li>
        </ol>
        <label class="accept">
          <input type="checkbox" bind:checked={accepted} />
          <span>
            J'accepte les
            <button type="button" class="link" onclick={() => openUrl("https://www.docker.com/legal/docker-subscription-service-agreement/")}>conditions d'utilisation de Docker Desktop</button>.
          </span>
        </label>
        <p class="license muted">
          Gratuit pour un usage personnel et pour les entreprises de moins de 250 salariés et de moins de 10 M$ de chiffre
          d'affaires ; au-delà, un abonnement Docker est nécessaire. Alternative gratuite :
          <button type="button" class="link" onclick={() => openUrl("https://rancherdesktop.io/")}>Rancher Desktop</button>.
        </p>
        <button class="btn brand" disabled={!accepted} onclick={install}><Icon name="download" size={16} /> Installer Docker Desktop</button>
      {:else}
        <p>Installez Docker Engine et son extension Compose, puis vérifiez à nouveau.</p>
        <button class="btn sm" onclick={() => openUrl("https://docs.docker.com/engine/install/")}><Icon name="external" size={14} /> Guide d'installation de Docker</button>
      {/if}
    </div>
  {/if}

  {#if error}
    <div class="line bad err"><Icon name="alert" size={18} /><span class="selectable">{error}</span></div>
  {/if}

  {#if phase === "idle"}
    <button class="btn ghost sm again" onclick={refresh}><Icon name="refresh" size={14} /> Vérifier à nouveau</button>
  {/if}
</div>

<style>
  .docker {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px 16px;
    border-radius: 14px;
    background: var(--surface-2);
    font-size: 13.5px;
  }
  .docker.ok {
    background: var(--success-soft);
  }
  .line {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }
  .line p {
    margin-top: 2px;
  }
  .line.bad {
    color: var(--danger);
  }
  .line.bad strong,
  .line.bad p,
  .line.bad span {
    color: var(--text);
  }
  .line.err span {
    color: var(--danger);
  }
  .facts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .facts li {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .facts .good :global(svg) {
    color: var(--success);
  }
  .facts .warn {
    color: var(--warning);
    font-weight: 500;
  }
  .hint {
    color: var(--text-2);
  }
  .need {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .need-head {
    display: flex;
    gap: 12px;
  }
  .need-head strong {
    font-size: 15px;
  }
  .need-head p {
    margin-top: 2px;
    color: var(--text-2);
  }
  .n-icon {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    background: var(--primary-soft);
    color: var(--primary);
    flex: none;
  }
  .how {
    margin: 0;
    padding-left: 20px;
    display: grid;
    gap: 4px;
    color: var(--text-2);
  }
  .accept {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    cursor: pointer;
  }
  .accept input {
    margin-top: 3px;
    accent-color: var(--primary);
  }
  .license {
    font-size: 12.5px;
  }
  .need .btn.brand {
    align-self: flex-start;
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--primary);
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .setup {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12.5px;
  }
  .bar {
    height: 8px;
    border-radius: 99px;
    background: var(--surface-3);
    overflow: hidden;
  }
  .bar div {
    height: 100%;
    border-radius: inherit;
    background: var(--brand-gradient);
    transition: width 0.3s ease;
  }
  .bar.indeterminate div {
    width: 35%;
    animation: slide 1.4s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(290%);
    }
  }
  .again {
    align-self: flex-start;
    margin-left: -8px;
  }
</style>
