<script lang="ts">
  import { api } from "../../lib/api";
  import { app, baseName, drop } from "../../lib/state.svelte";
  import DropZone from "../DropZone.svelte";
  import JobStatus from "./JobStatus.svelte";

  let path = $state<string | null>(null);
  let pages = $state(0);
  let mode = $state<"every" | "ranges">("every");
  let ranges = $state("");
  let error = $state("");
  let jobId = $state<string | null>(null);

  async function add(paths: string[]) {
    const probed = await api.probe(paths);
    const pdf = probed.find((p) => p.format === "pdf");
    error = pdf ? "" : "That is not a PDF.";
    if (!pdf) return;
    path = pdf.path;
    jobId = null;
    try {
      pages = (await api.pdfInfo(pdf.path, false)).pages;
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    drop.handler = add;
    return () => (drop.handler = null);
  });

  async function run() {
    if (!path) return;
    jobId = crypto.randomUUID();
    await api.startPdf({ tool: "split", id: jobId, input: path, ranges: mode === "every" ? "" : ranges });
  }

  const busy = $derived(!!jobId && ["queued", "running"].includes(app.jobs[jobId]?.state ?? ""));
</script>

<DropZone compact={!!path} label="Drop a PDF here" folders={false} filter={{ name: "PDF", extensions: ["pdf"] }} onfiles={add} />
{#if error}<div class="warning"><p>{error}</p></div>{/if}

{#if path}
  <p><strong>{baseName(path)}</strong> <span class="muted">has {pages} {pages === 1 ? "page" : "pages"}.</span></p>
  <div class="setting">
    <label class="radio"><input type="radio" bind:group={mode} value="every" /> One file per page</label>
    <label class="radio"><input type="radio" bind:group={mode} value="ranges" /> One file per range</label>
    {#if mode === "ranges"}
      <div class="field" style="margin: 6px 0 0 24px">
        <input type="text" bind:value={ranges} placeholder="1-3, 4-6, 7-" aria-label="Page ranges" />
        <div class="help">Separate ranges with commas. "7-" means page 7 to the end.</div>
      </div>
    {/if}
  </div>
  <div class="prevnext bottom">
    <button class="btn" onclick={() => (path = null)} disabled={busy}>Choose another PDF</button>
    <button class="btn primary push" onclick={run} disabled={busy || (mode === "ranges" && !ranges.trim())}>Split</button>
  </div>
{/if}

<JobStatus id={jobId} />
