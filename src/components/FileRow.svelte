<script lang="ts">
  import { api } from "../lib/api";
  import { CATEGORY_NAMES } from "../lib/tabs";
  import type { Item, Route } from "../lib/types";
  import {
    app,
    cancelItem,
    convertItems,
    copyOptions,
    findRoute,
    fmt,
    humanSize,
    optionSpecs,
    removeItem,
    resetOptions,
    setOption,
    setTarget,
    targetLabel,
    targets,
    toggleItem,
  } from "../lib/state.svelte";
  import OptionsForm from "./OptionsForm.svelte";

  let { item }: { item: Item } = $props();

  const f = $derived(fmt(item.format));
  const routes = $derived(targets(item.format));
  const groups = $derived.by(() => {
    const out: { label: string; routes: Route[] }[] = [];
    for (const r of routes) {
      const cat = fmt(r.to)?.category ?? "folder";
      const label = r.from === r.to ? "Same format" : CATEGORY_NAMES[cat];
      let g = out.find((x) => x.label === label);
      if (!g) out.push((g = { label, routes: [] }));
      g.routes.push(r);
    }
    return out;
  });
  const route = $derived(findRoute(item.format, item.to));
  const specs = $derived(optionSpecs(route));
  const busy = $derived(item.state === "running" || item.state === "queued");
  const pct = $derived(item.progress === null ? null : Math.round(item.progress * 100));
  const same = $derived(
    app.items.filter((o) => o !== item && o.format === item.format && o.to === item.to && o.state !== "running" && o.state !== "queued"),
  );
  const showSub = $derived((item.state === "failed" && !!item.message) || (item.showOptions && specs.length > 0 && !busy && item.state !== "done"));
</script>

<tr>
  <td>
    <div class="name">{item.name}</div>
    <div class="muted small">{f ? f.name : "Unknown type"} &middot; {humanSize(item.size)}</div>
  </td>
  <td>
    {#if routes.length}
      <select value={item.to} disabled={busy} aria-label="Convert {item.name} to" onchange={(e) => setTarget(item, e.currentTarget.value)}>
        {#each groups as g (g.label)}
          {#if groups.length > 1}
            <optgroup label={g.label}>
              {#each g.routes as r (r.to)}<option value={r.to}>{targetLabel(r)}</option>{/each}
            </optgroup>
          {:else}
            {#each g.routes as r (r.to)}<option value={r.to}>{targetLabel(r)}</option>{/each}
          {/if}
        {/each}
      </select>
    {:else}
      <span class="muted small">{f ? `The app can't convert ${f.name} files yet` : "The app doesn't know this type of file"}</span>
    {/if}
  </td>
  <td>
    {#if item.state === "ready"}
      <span class="muted">Ready</span>
    {:else if item.state === "queued"}
      <span class="muted">Waiting</span>
    {:else if item.state === "running"}
      <div class="progress" class:busy={pct === null}><span style:width={pct === null ? null : `${pct}%`}></span></div>
      <div class="muted small">{pct === null ? "Converting" : `${pct}%`}</div>
    {:else if item.state === "done"}
      <span class="status-done">Done</span>
      {#if item.message}<div class="muted small">{item.message}</div>{/if}
    {:else if item.state === "failed"}
      <span class="status-failed">Failed</span>
    {:else}
      <span class="muted">Canceled</span>
    {/if}
  </td>
  <td>
    <div class="actions">
      {#if busy}
        <button class="link" onclick={() => cancelItem(item)}>Cancel</button>
      {:else}
        {#if item.state === "done" && item.outputs[0]}
          <button class="link" onclick={() => api.openPath(item.outputs[0])}>Open</button>
          <button class="link" onclick={() => api.reveal(item.outputs[0])}>Show in folder</button>
        {/if}
        {#if item.state === "failed" || item.state === "canceled"}
          <button class="link" onclick={() => convertItems([item])}>Try again</button>
        {/if}
        {#if specs.length && item.state !== "done"}
          <button class="link" onclick={() => toggleItem(item, "showOptions")}>{item.showOptions ? "Fewer options" : "More options"}</button>
        {/if}
        <button class="link" onclick={() => removeItem(item)}>Remove</button>
      {/if}
    </div>
  </td>
</tr>
{#if showSub}
  <tr class="sub">
    <td colspan="4">
      {#if item.state === "failed" && item.message}
        <div class="warning" style="margin: 0 0 8px">
          <p>{item.message}</p>
          {#if item.log}
            <p><button class="link small" onclick={() => toggleItem(item, "showLog")}>{item.showLog ? "Hide details" : "Show details"}</button></p>
            {#if item.showLog}<pre class="log">{item.log}</pre>{/if}
          {/if}
        </div>
      {/if}
      {#if item.showOptions && specs.length && !busy && item.state !== "done"}
        <div class="panel">
          {#if route?.note}<p class="muted small">{route.note}</p>{/if}
          <OptionsForm
            {specs}
            values={item.options}
            uid={item.id}
            onset={(k, v) => setOption(item, k, v)}
            onreset={() => resetOptions(item)}
            sameCount={same.length}
            onApplyAll={() => copyOptions(item, same)}
          />
        </div>
      {/if}
    </td>
  </tr>
{/if}
