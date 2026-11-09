<script lang="ts">
  import { tabById } from "../lib/tabs";
  import { app, canConvert, clearTab, convertItems, findRoute, fmt, itemsInTab, setTarget, targets } from "../lib/state.svelte";
  import DropZone from "./DropZone.svelte";
  import FileRow from "./FileRow.svelte";

  const tab = $derived(tabById(app.tab));
  const items = $derived(itemsInTab(app.tab));
  const ready = $derived(items.filter(canConvert));
  const busy = $derived(items.filter((i) => i.state === "running" || i.state === "queued"));
  const notes = $derived([...new Set(items.map((i) => findRoute(i.format, i.to)?.note).filter((n): n is string => !!n))]);

  // Formats that every file in the list can become.
  const common = $derived.by(() => {
    const convertible = items.filter((i) => targets(i.format).length);
    if (convertible.length < 2) return [];
    let shared: string[] | null = null;
    for (const i of convertible) {
      const t = targets(i.format).map((r) => r.to);
      shared = shared ? shared.filter((x) => t.includes(x)) : t;
    }
    return shared ?? [];
  });

  let allTo = $state("");

  function applyAll() {
    for (const i of items) {
      if (i.state !== "running" && i.state !== "queued" && targets(i.format).some((r) => r.to === allTo)) setTarget(i, allTo);
    }
    allTo = "";
  }

  const convertLabel = $derived(ready.length === 0 ? "Convert" : ready.length === 1 ? "Convert 1 file" : `Convert ${ready.length} files`);
</script>

<h1>{tab.title}</h1>
<p>{tab.intro}</p>

<div class="prevnext">
  <button class="btn" onclick={() => clearTab(app.tab)} disabled={items.length === busy.length}>Clear list</button>
  {#if busy.length}<span class="muted small">{busy.length} converting</span>{/if}
  <button class="btn primary push" onclick={() => convertItems(items)} disabled={!ready.length}>{convertLabel}</button>
</div>

<DropZone compact={items.length > 0} />

{#if notes.length}
  <div class="note">
    {#each notes as n (n)}<p>{n}</p>{/each}
  </div>
{/if}

{#if items.length}
  {#if common.length}
    <div class="setall">
      <label for="setall">Convert every file to</label>
      <select id="setall" bind:value={allTo} onchange={applyAll}>
        <option value="" disabled>Choose a format</option>
        {#each common as t (t)}<option value={t}>{fmt(t)?.name}</option>{/each}
      </select>
    </div>
  {/if}
  <table class="list">
    <thead>
      <tr>
        <th>File</th>
        <th style="width: 235px">Convert to</th>
        <th style="width: 150px">Status</th>
        <th style="width: 210px"><span class="muted small">Actions</span></th>
      </tr>
    </thead>
    <tbody>
      {#each items as item (item)}
        <FileRow {item} />
      {/each}
    </tbody>
  </table>
  {#if items.length > 8}
    <div class="prevnext bottom">
      <button class="btn" onclick={() => clearTab(app.tab)} disabled={items.length === busy.length}>Clear list</button>
      <button class="btn primary push" onclick={() => convertItems(items)} disabled={!ready.length}>{convertLabel}</button>
    </div>
  {/if}
{/if}
