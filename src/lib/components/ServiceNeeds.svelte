<script lang="ts">
  import { openUrl } from "../api";
  import type { ServiceNeed } from "../types";
  import Icon from "./Icon.svelte";

  let { services, compact = false }: { services: ServiceNeed[]; compact?: boolean } = $props();

  // Indispensable first.
  const sorted = $derived([...services].sort((a, b) => (a.level === b.level ? 0 : a.level === "required" ? -1 : 1)));
</script>

{#if sorted.length}
  <ul class="needs" class:compact>
    {#each sorted as s}
      <li class={s.level}>
        <span class="n-icon"><Icon name={s.level === "required" ? "key" : "plug"} size={compact ? 17 : 19} /></span>
        <div class="n-body">
          <div class="n-head">
            <strong>{s.title}</strong>
            {#if s.level === "required"}
              <span class="pill warning">Indispensable</span>
            {:else}
              <span class="pill info">Facultatif · conseillé</span>
            {/if}
          </div>
          <p>{s.detail}</p>
          {#if s.links?.length}
            <div class="links">
              {#each s.links as l}
                <button type="button" class="link" onclick={() => openUrl(l.url)}>{l.label} <Icon name="external" size={12} /></button>
              {/each}
            </div>
          {/if}
        </div>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .needs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 12px;
  }
  li {
    display: flex;
    gap: 14px;
    padding: 16px 18px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--surface);
  }
  li.required {
    border-color: color-mix(in srgb, var(--warning) 35%, var(--border));
    background: color-mix(in srgb, var(--warning-soft) 55%, var(--surface));
  }
  .n-icon {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 11px;
    flex: none;
    background: var(--info-soft);
    color: var(--info);
  }
  .required .n-icon {
    background: var(--warning-soft);
    color: var(--warning);
  }
  .n-body {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .n-head {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .n-head strong {
    font-size: 15px;
  }
  p {
    font-size: 13.5px;
    color: var(--text-2);
    line-height: 1.55;
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: none;
    background: none;
    padding: 0;
    color: var(--primary);
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .link:hover {
    text-decoration: underline;
    text-underline-offset: 3px;
  }
  .compact {
    gap: 8px;
  }
  .compact li {
    padding: 12px 14px;
    gap: 12px;
  }
  .compact .n-icon {
    width: 32px;
    height: 32px;
    border-radius: 9px;
  }
  .compact .n-head strong {
    font-size: 14px;
  }
  .compact p {
    font-size: 12.5px;
  }
</style>
