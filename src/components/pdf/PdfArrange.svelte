<script lang="ts">
  import { api } from "../../lib/api";
  import { app, baseName, drop } from "../../lib/state.svelte";
  import DropZone from "../DropZone.svelte";
  import JobStatus from "./JobStatus.svelte";

  interface Page {
    page: number;
    rotate: number;
    removed: boolean;
    thumb: string;
  }

  let path = $state<string | null>(null);
  let pages = $state<Page[]>([]);
  let loading = $state(false);
  let error = $state("");
  let jobId = $state<string | null>(null);

  async function add(paths: string[]) {
    const probed = await api.probe(paths);
    const pdf = probed.find((p) => p.format === "pdf");
    error = pdf ? "" : "That is not a PDF.";
    if (!pdf) return;
    path = pdf.path;
    jobId = null;
    loading = true;
    try {
      const info = await api.pdfInfo(pdf.path, true);
      pages = Array.from({ length: info.pages }, (_, i) => ({ page: i + 1, rotate: 0, removed: false, thumb: info.thumbnails[i] ?? "" }));
    } catch (e) {
      error = String(e);
      path = null;
    }
    loading = false;
  }

  $effect(() => {
    drop.handler = add;
    return () => (drop.handler = null);
  });

  function move(i: number, d: number) {
    const j = i + d;
    if (j < 0 || j >= pages.length) return;
    [pages[i], pages[j]] = [pages[j], pages[i]];
  }

  async function run() {
    if (!path) return;
    jobId = crypto.randomUUID();
    const plan = pages.filter((p) => !p.removed).map((p) => ({ page: p.page, rotate: ((p.rotate % 360) + 360) % 360 }));
    await api.startPdf({ tool: "arrange", id: jobId, input: path, pages: plan });
  }

  const kept = $derived(pages.filter((p) => !p.removed).length);
  const busy = $derived(!!jobId && ["queued", "running"].includes(app.jobs[jobId]?.state ?? ""));
</script>

<DropZone compact={!!path} label="Drop a PDF here" folders={false} filter={{ name: "PDF", extensions: ["pdf"] }} onfiles={add} />
{#if error}<div class="warning"><p>{error}</p></div>{/if}
{#if loading}<p class="muted">Drawing the pages</p>{/if}

{#if path && !loading}
  <p><strong>{baseName(path)}</strong> <span class="muted">({pages.length} pages). Move, turn or remove pages, then save a new copy.</span></p>
  <div class="pages">
    {#each pages as p, i (p.page)}
      <div class="page" class:removed={p.removed}>
        <div class="thumb">
          {#if p.thumb}<img src={p.thumb} alt="Page {p.page}" style:transform="rotate({p.rotate}deg)" />{/if}
        </div>
        <div>Page {p.page}</div>
        <div class="row">
          <button title="Move earlier" onclick={() => move(i, -1)} disabled={i === 0}>Earlier</button>
          <button title="Move later" onclick={() => move(i, 1)} disabled={i === pages.length - 1}>Later</button>
        </div>
        <div class="row">
          <button title="Turn left" onclick={() => (p.rotate -= 90)}>Left</button>
          <button title="Turn right" onclick={() => (p.rotate += 90)}>Right</button>
          <button onclick={() => (p.removed = !p.removed)}>{p.removed ? "Keep" : "Remove"}</button>
        </div>
      </div>
    {/each}
  </div>
  <div class="prevnext bottom">
    <button class="btn" onclick={() => (path = null)} disabled={busy}>Choose another PDF</button>
    <button class="btn primary push" onclick={run} disabled={busy || kept === 0}>Save a new PDF with {kept} {kept === 1 ? "page" : "pages"}</button>
  </div>
{/if}

<JobStatus id={jobId} />
