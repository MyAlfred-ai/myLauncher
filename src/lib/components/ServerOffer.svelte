<script lang="ts">
  import { launcher } from "../launcher.svelte";
  import Icon from "./Icon.svelte";

  // Installs on a server are done by MyAlfred.ai: this card leads to the contact page.
  let { appId, onnavigate }: { appId?: string; onnavigate?: () => void } = $props();

  const app = $derived(appId ? launcher.app(appId) : undefined);

  function more() {
    // Read before onnavigate: closing the install dialog unmounts this card.
    const id = appId;
    onnavigate?.();
    launcher.go({ name: "contact", appId: id });
  }
</script>

<div class="offer">
  <span class="offer-icon"><Icon name="users" size={20} /></span>
  <div class="offer-text">
    <strong>{app ? `${app.name} pour toute l'équipe` : "Pour toute l'équipe"}</strong>
    <span>Installation sur votre serveur ou hébergement en Europe : MyAlfred.ai s'en charge.</span>
  </div>
  <button class="btn sm" onclick={more}>En savoir plus <Icon name="chevron" size={15} /></button>
</div>

<style>
  .offer {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    border-radius: 16px;
    border: 1.5px solid transparent;
    background:
      linear-gradient(var(--surface), var(--surface)) padding-box,
      var(--brand-gradient) border-box;
  }
  .offer-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: 12px;
    background: var(--primary-soft);
    color: var(--primary);
    flex: none;
  }
  .offer-text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
    font-size: 13px;
    color: var(--text-2);
  }
  .offer-text strong {
    font-size: 14.5px;
    color: var(--text);
  }
  .btn {
    flex: none;
  }
  @media (max-width: 560px) {
    .offer {
      flex-wrap: wrap;
    }
    .btn {
      width: 100%;
    }
  }
</style>
