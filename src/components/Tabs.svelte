<script lang="ts">
  import { TABS } from "../lib/tabs";
  import { app, openTab, tabCount } from "../lib/state.svelte";
</script>

<nav class="tabs" aria-label="File types">
  {#each TABS as t (t.id)}
    {@const n = tabCount(t.id)}
    <button class="tab" class:active={app.tab === t.id && app.view !== "settings"} onclick={() => openTab(t.id)}>
      {t.label}{#if n > 0 && t.id !== "all"}<span class="count">{n}</span>{/if}
    </button>
  {/each}
</nav>
