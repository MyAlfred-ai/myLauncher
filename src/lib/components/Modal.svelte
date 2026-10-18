<script lang="ts">
  import type { Snippet } from "svelte";
  import { fade, scale } from "svelte/transition";
  import Icon from "./Icon.svelte";

  let {
    title,
    subtitle = "",
    onclose,
    width = 620,
    children,
    footer,
    header,
  }: {
    title: string;
    subtitle?: string;
    onclose: () => void;
    width?: number;
    children: Snippet;
    footer?: Snippet;
    header?: Snippet;
  } = $props();

  let dialog: HTMLDivElement | undefined = $state();

  function onkey(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }

  $effect(() => {
    dialog?.querySelector<HTMLElement>("input, select, button.primary")?.focus();
  });
</script>

<svelte:window onkeydown={onkey} />

<div class="backdrop" transition:fade={{ duration: 150 }} onclick={onclose} role="presentation"></div>
<div
  class="modal"
  style:--w="{width}px"
  role="dialog"
  aria-modal="true"
  aria-label={title}
  bind:this={dialog}
  transition:scale={{ start: 0.97, duration: 180 }}
>
  <header>
    {#if header}{@render header()}{/if}
    <div class="titles">
      <h2>{title}</h2>
      {#if subtitle}<p class="muted">{subtitle}</p>{/if}
    </div>
    <button class="btn ghost icon sm" aria-label="Fermer" onclick={onclose}><Icon name="x" /></button>
  </header>
  <div class="body">
    {@render children()}
  </div>
  {#if footer}
    <footer>{@render footer()}</footer>
  {/if}
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgb(12 6 28 / 45%);
    backdrop-filter: blur(4px);
    z-index: 100;
  }
  .modal {
    position: fixed;
    z-index: 101;
    left: 50%;
    top: 50%;
    translate: -50% -50%;
    width: min(var(--w), calc(100vw - 32px));
    max-height: min(820px, calc(100vh - 48px));
    display: flex;
    flex-direction: column;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    box-shadow: var(--shadow-lg);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 20px 20px 14px 24px;
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  h2 {
    font-size: 19px;
  }
  .titles p {
    font-size: 13.5px;
    margin-top: 2px;
  }
  .body {
    padding: 4px 24px 20px;
    overflow-y: auto;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 14px 20px;
    border-top: 1px solid var(--border);
    background: var(--surface);
  }
  @media (max-width: 760px) {
    .modal {
      top: auto;
      bottom: 0;
      translate: -50% 0;
      width: 100vw;
      max-height: calc(100vh - 24px);
      border-radius: var(--radius-xl) var(--radius-xl) 0 0;
    }
    .body {
      padding: 4px 16px 16px;
    }
    header {
      padding: 18px 12px 12px 16px;
    }
    footer {
      padding: 12px 16px calc(12px + env(safe-area-inset-bottom));
    }
  }
</style>
