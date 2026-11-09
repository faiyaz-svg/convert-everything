<script lang="ts">
  import { api } from "../../lib/api";
  import { app, drop, fmt } from "../../lib/state.svelte";
  import type { ProbedFile } from "../../lib/types";
  import DropZone from "../DropZone.svelte";
  import JobStatus from "./JobStatus.svelte";

  let { mode }: { mode: "merge" | "images" } = $props();

  const IMAGES = ["png", "jpg", "webp", "heic", "avif", "gif", "bmp", "tiff", "psd", "jxl", "tga"];
  let files = $state<ProbedFile[]>([]);
  let name = $state("");
  let skipped = $state(0);
  let jobId = $state<string | null>(null);

  const accept = (f: string | null) => !!f && (IMAGES.includes(f) || (mode === "merge" && f === "pdf"));

  async function add(paths: string[]) {
    const probed = await api.probe(paths);
    const ok = probed.filter((p) => accept(p.format) && !files.some((f) => f.path === p.path));
    skipped = probed.filter((p) => !accept(p.format)).length;
    files.push(...ok);
  }

  $effect(() => {
    drop.handler = add;
    return () => (drop.handler = null);
  });

  function move(i: number, d: number) {
    const j = i + d;
    if (j < 0 || j >= files.length) return;
    [files[i], files[j]] = [files[j], files[i]];
  }

  async function run() {
    jobId = crypto.randomUUID();
    const tool = mode === "merge" ? "merge" : "imagesToPdf";
    await api.startPdf({ tool, id: jobId, inputs: files.map((f) => f.path), name: name || (mode === "merge" ? "Merged" : "Pictures") });
  }

  const busy = $derived(!!jobId && ["queued", "running"].includes(app.jobs[jobId]?.state ?? ""));
</script>

<DropZone
  compact={files.length > 0}
  label={mode === "merge" ? "Drop PDFs or pictures here" : "Drop pictures here"}
  folders={false}
  filter={{ name: mode === "merge" ? "PDFs and pictures" : "Pictures", extensions: mode === "merge" ? ["pdf", ...IMAGES, "jpeg", "tif"] : [...IMAGES, "jpeg", "tif"] }}
  onfiles={add}
/>

{#if skipped}
  <div class="note"><p>{skipped} {skipped === 1 ? "file was" : "files were"} left out because {mode === "merge" ? "only PDFs and pictures can be merged" : "only pictures can go in"}.</p></div>
{/if}

{#if files.length}
  <h3>Order</h3>
  <ol class="file-list">
    {#each files as f, i (f.path)}
      <li>
        <span class="muted small">{i + 1}.</span>
        <span class="grow">{f.name} <span class="muted small">{fmt(f.format)?.name}</span></span>
        <button class="link small" disabled={i === 0} onclick={() => move(i, -1)}>Up</button>
        <button class="link small" disabled={i === files.length - 1} onclick={() => move(i, 1)}>Down</button>
        <button class="link small" onclick={() => files.splice(i, 1)}>Remove</button>
      </li>
    {/each}
  </ol>

  <div class="field" style="margin-bottom: 14px">
    <label class="title" for="pdf-name">Name of the new PDF</label>
    <input id="pdf-name" type="text" bind:value={name} placeholder={mode === "merge" ? "Merged" : "Pictures"} />
    <div class="help">Saved next to the first file{app.settings?.outputMode === "folder" ? ", or in your chosen folder" : ""}.</div>
  </div>

  <div class="prevnext bottom">
    <button class="btn" onclick={() => (files = [])} disabled={busy}>Clear list</button>
    <button class="btn primary push" onclick={run} disabled={busy || (mode === "merge" && files.length < 2)}>
      {mode === "merge" ? `Merge ${files.length} files` : `Make a PDF from ${files.length} ${files.length === 1 ? "picture" : "pictures"}`}
    </button>
  </div>
{/if}

<JobStatus id={jobId} />
