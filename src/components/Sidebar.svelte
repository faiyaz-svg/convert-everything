<script lang="ts">
  import { api } from "../lib/api";
  import { TABS, tabById, tabForCategory } from "../lib/tabs";
  import { app, baseName, fmt, formatsInTab, openFormat, openTab, tabCount, type PdfTool } from "../lib/state.svelte";

  const PDF_TOOLS: { id: PdfTool; label: string }[] = [
    { id: "merge", label: "Merge PDFs" },
    { id: "split", label: "Split a PDF" },
    { id: "arrange", label: "Reorder and rotate pages" },
    { id: "images", label: "Images to PDF" },
    { id: "toImages", label: "PDF to images" },
    { id: "compress", label: "Compress PDFs" },
  ];

  const recent = $derived.by(() => {
    const tab = app.tab;
    const list = tab === "all" || tab === "pdf" ? app.history : app.history.filter((h) => tabForCategory(fmt(h.from)?.category) === tab);
    return list.slice(0, 8);
  });
</script>

<aside class="sidebar">
  {#if app.view === "settings"}
    <h2>Settings</h2>
    <button class="side-link active">Settings</button>
    <button class="side-link" onclick={() => (app.view = "list")}>Back to converting</button>
  {:else if app.tab === "pdf"}
    <h2>PDF tools</h2>
    {#each PDF_TOOLS as t (t.id)}
      <button
        class="side-link"
        class:active={app.pdfTool === t.id}
        onclick={() => {
          app.pdfTool = t.id;
          app.view = "list";
        }}>{t.label}</button
      >
    {/each}
  {:else if app.tab === "all"}
    <h2>File types</h2>
    {#each TABS.filter((t) => t.id !== "all") as t (t.id)}
      {@const n = tabCount(t.id)}
      <button class="side-link" onclick={() => openTab(t.id)}>{t.label}{#if n}&nbsp;<small>({n})</small>{/if}</button>
    {/each}
  {:else}
    <h2>{tabById(app.tab).title}</h2>
    <button class="side-link" class:active={app.view === "list"} onclick={() => openTab(app.tab)}>Files to convert</button>
    {#each formatsInTab(app.tab) as f (f.id)}
      <button class="side-link" class:active={app.view === "format" && app.formatPage === f.id} onclick={() => openFormat(f.id)}>{f.name}</button>
    {/each}
  {/if}

  {#if recent.length && app.view !== "settings"}
    <h2>Recent</h2>
    {#each recent as h (h.time + h.output)}
      <button class="side-link" title="Show {h.output} in its folder" onclick={() => api.reveal(h.output)}>{baseName(h.output)}</button>
    {/each}
  {/if}
</aside>
