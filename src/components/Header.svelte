<script lang="ts">
  import { app, applyTheme, openFormat, saveSettings } from "../lib/state.svelte";
  import type { FormatInfo } from "../lib/types";

  let query = $state("");
  let open = $state(false);
  let hit = $state(0);

  const results = $derived.by((): FormatInfo[] => {
    const q = query.trim().toLowerCase().replace(/^\./, "");
    if (!q || !app.registry) return [];
    return app.registry.formats
      .filter((f) => f.id !== "folder" && (f.name.toLowerCase().includes(q) || f.exts.some((e) => e.startsWith(q))))
      .slice(0, 12);
  });

  function choose(f: FormatInfo) {
    openFormat(f.id);
    query = "";
    open = false;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      hit = Math.min(hit + 1, results.length - 1);
      e.preventDefault();
    } else if (e.key === "ArrowUp") {
      hit = Math.max(hit - 1, 0);
      e.preventDefault();
    } else if (e.key === "Enter" && results[hit]) {
      choose(results[hit]);
    } else if (e.key === "Escape") {
      open = false;
    }
  }

  function toggleTheme() {
    if (!app.settings) return;
    app.settings.theme = app.dark ? "light" : "dark";
    applyTheme();
    saveSettings(true);
  }
</script>

<header class="header">
  <div class="brand">Convert <span>Everything</span></div>
  <div class="spacer"></div>
  <div class="search">
    <input
      type="text"
      placeholder="Find a format"
      aria-label="Find a format"
      bind:value={query}
      oninput={() => {
        open = true;
        hit = 0;
      }}
      onfocus={() => (open = true)}
      onblur={() => setTimeout(() => (open = false), 150)}
      onkeydown={onKey}
    />
    {#if open && results.length}
      <ul class="search-results">
        {#each results as f, i (f.id)}
          <li>
            <button class:hit={i === hit} onclick={() => choose(f)}>
              <span>{f.name}</span>
              <span class="muted small">.{f.exts.slice(0, 3).join(", .")}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
  <button class="header-btn" onclick={toggleTheme}>{app.dark ? "Light mode" : "Dark mode"}</button>
  <button class="header-btn" class:active={app.view === "settings"} onclick={() => (app.view = app.view === "settings" ? "list" : "settings")}>
    Settings
  </button>
</header>
