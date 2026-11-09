<script lang="ts">
  import { api } from "../lib/api";
  import { addPaths, app, fmt, openFormat, sourcesOf, targetLabel, targets } from "../lib/state.svelte";

  let { id }: { id: string } = $props();

  const f = $derived(fmt(id)!);
  const outs = $derived(targets(id));
  const ins = $derived(sourcesOf(id).filter((r) => r.from !== id));

  async function choose() {
    const paths = await api.pickFiles({ name: `${f.name} files`, extensions: f.exts });
    if (paths.length) addPaths(paths);
  }
</script>

<h1>{f.name} {#if f.exts.length}<small>.{f.exts.join(", .")}</small>{/if}</h1>

<div class="prevnext">
  <button class="btn" onclick={() => (app.view = "list")}>Back to the list</button>
  {#if outs.length}
    <button class="btn primary push" onclick={choose}>Choose {f.name} files</button>
  {/if}
</div>

<p>{f.description}</p>

{#if f.canRead && !f.canWrite}
  <div class="note"><p>The app can read {f.name} files but can't create them.</p></div>
{/if}

{#if f.id !== "folder"}
  <h2>Converts to</h2>
  {#if outs.length}
    <table class="list">
      <thead><tr><th style="width: 40%">Format</th><th>Notes</th></tr></thead>
      <tbody>
        {#each outs as r (r.to)}
          <tr>
            <td>
              {#if r.to === r.from || r.to === "folder"}
                {targetLabel(r)}
              {:else}
                <button class="link" onclick={() => openFormat(r.to)}>{targetLabel(r)}</button>
              {/if}
            </td>
            <td class="muted small">{r.note ?? ""}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <p>Nothing yet. The app can create {f.name} files from other formats.</p>
  {/if}
{/if}

<h2>Made from</h2>
{#if ins.length}
  <p>
    {#each ins as r, i (r.from)}
      <button class="link" onclick={() => openFormat(r.from)}>{fmt(r.from)?.name}</button>{i < ins.length - 1 ? ", " : ""}
    {/each}
  </p>
{:else}
  <p class="muted">No other format converts to {f.name}.</p>
{/if}
