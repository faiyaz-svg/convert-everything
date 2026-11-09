<script lang="ts">
  import { api } from "../../lib/api";
  import { app, baseName } from "../../lib/state.svelte";

  let { id }: { id: string | null } = $props();

  const job = $derived(id ? app.jobs[id] : undefined);
  const pct = $derived(job?.progress == null ? null : Math.round(job.progress * 100));
</script>

{#if job}
  {#if job.state === "queued" || job.state === "running"}
    <div class="panel">
      <div class="progress" class:busy={pct === null}><span style:width={pct === null ? null : `${pct}%`}></span></div>
      <p class="muted small" style="margin: 4px 0 0">
        Working{pct === null ? "" : ` (${pct}%)`}.
        <button class="link" onclick={() => api.cancel(job.id)}>Cancel</button>
      </p>
    </div>
  {:else if job.state === "done"}
    <div class="panel">
      <p style="margin: 0 0 4px"><span class="status-done">Done.</span> Saved as <strong>{baseName(job.outputs[0] ?? "")}</strong>.</p>
      {#if job.outputs[0]}
        <div class="actions">
          <button class="link" onclick={() => api.openPath(job.outputs[0])}>Open</button>
          <button class="link" onclick={() => api.reveal(job.outputs[0])}>Show in folder</button>
        </div>
      {/if}
    </div>
  {:else if job.state === "failed"}
    <div class="warning"><p>{job.message ?? "Something went wrong."}</p></div>
  {:else}
    <p class="muted">Canceled.</p>
  {/if}
{/if}
