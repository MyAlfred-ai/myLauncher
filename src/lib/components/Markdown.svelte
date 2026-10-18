<script lang="ts">
  import { marked } from "marked";
  import DOMPurify from "dompurify";
  import { openUrl } from "../api";

  let { source }: { source: string } = $props();

  // Changelogs come from GitHub: render them, then strip anything active.
  const html = $derived(
    DOMPurify.sanitize(marked.parse(source, { async: false, gfm: true, breaks: false }) as string, {
      FORBID_TAGS: ["style", "iframe", "form", "input", "img"],
    }),
  );

  function onclick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a");
    if (a?.href) {
      e.preventDefault();
      if (/^https?:/.test(a.href)) openUrl(a.href);
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="markdown" {onclick}>{@html html}</div>
