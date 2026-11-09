<script lang="ts">
  import { api } from "../../lib/api";
  import { app, drop, humanSize } from "../../lib/state.svelte";
  import type { ProbedFile } from "../../lib/types";
  import DropZone from "../DropZone.svelte";
  import OptionsForm from "../OptionsForm.svelte";

  let { mode }: { mode: "toImages" | "compress" } = $props();

  let files = $state<(ProbedFile & { jobId: string | null })[]>([]);
  let imageFormat = $state("png");
  let values = $state<Record<string, unknown>>({});

  const optionSet = $derived(mode === "compress" ? "pdfcompress" : `pdfrender:${imageFormat}`);
  const specs = $derived(app.registry?.optionSets[optionSet] ?? []);

  async function add(paths: string[]) {
    const probed = await api.probe(paths);
    for (const p of probed) {
      if (p.format === "pdf" && !files.some((f) => f.path === p.path)) files.push({ ...p, jobId: null });
    }
  }

  $effect(() => {
    drop.handler = add;
    return () => (drop.handler = null);
  });

  async function run() {
    const to = mode === "compress" ? "pdf" : imageFormat;
    const options: Record<string, unknown> = {};
    for (const s of specs) {
      const v = values[s.key] ?? s.default;
      if (v !== null && v !== "") options[s.key] = v;
    }
    const requests = files.map((f) => {
      f.jobId = crypto.randomUUID();
      return { id: f.jobId, input: f.path, from: "pdf", to, options };
    });
    await api.start(requests);
  }

  const busy = $derived(files.some((f) => f.jobId && ["queued", "running"].includes(app.jobs[f.jobId]?.state ?? "")));
</script>

<DropZone compact={files.length > 0} label="Drop PDFs here" filter={{ name: "PDF", extensions: ["pdf"] }} onfiles={add} />

{#if files.length}
  <div class="panel" style="margin-bottom: 14px">
    {#if mode === "toImages"}
      <div class="field" style="margin-bottom: 10px">
        <label class="title" for="img-format">Image format</label>
        <select id="img-format" bind:value={imageFormat}>
          <option value="png">PNG (sharp text, larger)</option>
          <option value="jpg">JPG (smaller, for photos)</option>
          <option value="tiff">TIFF</option>
        </select>
      </div>
    {/if}
    <OptionsForm {specs} {values} uid="pdf-batch" onset={(k, v) => (values[k] = v)} onreset={() => (values = {})} />
  </div>

  <table class="list">
    <thead><tr><th>File</th><th style="width: 200px">Status</th><th style="width: 190px"></th></tr></thead>
    <tbody>
      {#each files as f, i (f.path)}
        {@const job = f.jobId ? app.jobs[f.jobId] : undefined}
        <tr>
          <td><div class="name">{f.name}</div><div class="muted small">{humanSize(f.size)}</div></td>
          <td>
            {#if !job}
              <span class="muted">Ready</span>
            {:else if job.state === "queued"}
              <span class="muted">Waiting</span>
            {:else if job.state === "running"}
              <div class="progress" class:busy={job.progress === null}><span style:width={job.progress === null ? null : `${Math.round(job.progress * 100)}%`}></span></div>
            {:else if job.state === "done"}
              <span class="status-done">Done</span>{#if job.message}<div class="muted small">{job.message}</div>{/if}
            {:else if job.state === "failed"}
              <span class="status-failed">Failed</span><div class="muted small">{job.message}</div>
            {:else}
              <span class="muted">Canceled</span>
            {/if}
          </td>
          <td>
            <div class="actions">
              {#if job?.state === "done" && job.outputs[0]}
                <button class="link" onclick={() => api.openPath(job.outputs[0])}>Open</button>
                <button class="link" onclick={() => api.reveal(job.outputs[0])}>Show in folder</button>
              {:else if job && ["queued", "running"].includes(job.state)}
                <button class="link" onclick={() => api.cancel(job.id)}>Cancel</button>
              {:else}
                <button class="link" onclick={() => files.splice(i, 1)}>Remove</button>
              {/if}
            </div>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>

  <div class="prevnext bottom">
    <button class="btn" onclick={() => (files = [])} disabled={busy}>Clear list</button>
    <button class="btn primary push" onclick={run} disabled={busy}>
      {mode === "compress" ? "Compress" : "Convert"} {files.length} {files.length === 1 ? "PDF" : "PDFs"}
    </button>
  </div>
{/if}
