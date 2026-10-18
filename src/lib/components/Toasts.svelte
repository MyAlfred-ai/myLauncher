<script lang="ts">
  import { fly } from "svelte/transition";
  import { launcher } from "../launcher.svelte";
  import Icon from "./Icon.svelte";
</script>

<div class="toasts" aria-live="polite">
  {#each launcher.toasts as t (t.id)}
    <div class="toast {t.kind}" transition:fly={{ y: 16, duration: 200 }} role={t.kind === "error" ? "alert" : "status"}>
      <Icon name={t.kind === "error" ? "alert" : t.kind === "success" ? "check-circle" : "info"} size={18} />
      <span class="selectable">{t.text}</span>
      <button class="close" aria-label="Fermer" onclick={() => launcher.dismiss(t.id)}><Icon name="x" size={14} /></button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 20px;
    bottom: 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    z-index: 200;
    max-width: min(420px, calc(100vw - 32px));
  }
  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 12px 12px 14px;
    border-radius: 14px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-lg);
    font-size: 14px;
  }
  .toast :global(svg) {
    flex: none;
    margin-top: 1px;
  }
  .toast.success {
    color: var(--success);
  }
  .toast.error {
    color: var(--danger);
  }
  .toast.info {
    color: var(--primary);
  }
  .toast span {
    color: var(--text);
    flex: 1;
  }
  .close {
    border: none;
    background: none;
    color: var(--muted);
    cursor: pointer;
    padding: 2px;
    border-radius: 6px;
  }
  @media (max-width: 760px) {
    .toasts {
      right: 16px;
      left: 16px;
      bottom: 84px;
      max-width: none;
    }
  }
</style>
