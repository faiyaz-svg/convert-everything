<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { app, applyTheme, saveSettings } from "../lib/state.svelte";
  import type { EngineStatus } from "../lib/types";

  let engines = $state<EngineStatus[]>([]);
  const s = $derived(app.settings!);

  onMount(async () => {
    engines = await api.engines();
  });

  function changed(now = false) {
    saveSettings(now);
  }

  async function chooseFolder() {
    const p = await api.pickFolder();
    if (p) {
      s.outputFolder = p;
      s.outputMode = "folder";
      changed(true);
    }
  }

  async function clearHistory() {
    await api.clearHistory();
    app.history = [];
  }

  async function licenses() {
    api.openPath(await api.licensesFolder());
  }
</script>

<h1>Settings</h1>
<div class="prevnext">
  <button class="btn" onclick={() => (app.view = "list")}>Back to converting</button>
</div>

<h2>Appearance</h2>
<div class="setting">
  {#each [["system", "Same as Windows"], ["light", "Light"], ["dark", "Dark"]] as [value, label] (value)}
    <label class="radio">
      <input
        type="radio"
        name="theme"
        {value}
        checked={s.theme === value}
        onchange={() => {
          s.theme = value as typeof s.theme;
          applyTheme();
          changed(true);
        }}
      />
      {label}
    </label>
  {/each}
</div>

<h2>Where converted files go</h2>
<div class="setting">
  <label class="radio">
    <input type="radio" name="out" checked={s.outputMode === "source"} onchange={() => ((s.outputMode = "source"), changed(true))} />
    Next to the original file
  </label>
  <label class="radio">
    <input
      type="radio"
      name="out"
      checked={s.outputMode === "folder"}
      onchange={() => {
        if (!s.outputFolder) chooseFolder();
        else {
          s.outputMode = "folder";
          changed(true);
        }
      }}
    />
    In one folder{#if s.outputFolder}: <code>{s.outputFolder}</code>{/if}
  </label>
  <p style="margin-top: 6px"><button class="btn small" onclick={chooseFolder}>Choose a folder</button></p>
  <p class="muted small">The app never replaces a file. If the name is taken, it adds a number, as in "photo (1).jpg".</p>
</div>

<h2>Speed</h2>
<div class="setting field">
  <label class="title" for="parallel">Files converted at the same time</label>
  <input
    id="parallel"
    type="number"
    min="1"
    max="16"
    value={s.parallel}
    onchange={(e) => {
      s.parallel = Math.max(1, Math.min(16, Number(e.currentTarget.value) || 1));
      changed();
    }}
  />
  <div class="help">More is faster on a strong computer but uses more memory. Video conversions already use every processor core.</div>
</div>

<h2>File Explorer</h2>
<div class="setting">
  <label class="check">
    <input type="checkbox" checked={s.explorerMenu} onchange={(e) => ((s.explorerMenu = e.currentTarget.checked), changed(true))} />
    <span>Show "Convert with Convert Everything" when you right-click a file or folder</span>
  </label>
  <p class="muted small">On Windows 11 it is under "Show more options".</p>
</div>

<h2>Updates</h2>
<div class="setting">
  <label class="check">
    <input type="checkbox" checked={s.checkUpdates} onchange={(e) => ((s.checkUpdates = e.currentTarget.checked), changed(true))} />
    <span>Check GitHub for a new version when the app starts</span>
  </label>
  <p class="muted small">This is the only time the app goes online. Your files are never sent anywhere.</p>
</div>

<h2>History</h2>
<div class="setting">
  <p>The app keeps a list of the last 100 conversions on this computer. It shows under "Recent" in the sidebar.</p>
  <button class="btn small" onclick={clearHistory} disabled={!app.history.length}>Clear history</button>
</div>

<h2>Engines</h2>
<p>These open-source programs do the converting. They are part of the app and run on this computer.</p>
<table class="list">
  <thead><tr><th>Program</th><th>Version</th><th>License</th><th>Status</th></tr></thead>
  <tbody>
    {#each engines as e (e.name)}
      <tr>
        <td>{e.title}</td>
        <td>{e.version || "unknown"}</td>
        <td class="small">{e.license}</td>
        <td>{#if e.found}<span class="status-done">Ready</span>{:else}<span class="status-failed">Missing</span>{/if}</td>
      </tr>
    {/each}
  </tbody>
</table>
<p style="margin-top: 10px"><button class="btn small" onclick={licenses}>Show the license files</button></p>

<h2>About</h2>
<p>
  Convert Everything {app.version}. Free and open source under the GNU General Public License, version 3.
  <button class="link" onclick={() => api.openUrl("https://github.com/faiyaz-svg/convert-everything")}>Source code on GitHub</button>.
</p>
