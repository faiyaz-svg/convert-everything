<script lang="ts">
  import { api } from "../lib/api";
  import { addPaths, app } from "../lib/state.svelte";

  let {
    compact = false,
    label = "Drop files or folders here",
    folders = true,
    filter,
    onfiles,
  }: {
    compact?: boolean;
    label?: string;
    folders?: boolean;
    filter?: { name: string; extensions: string[] };
    onfiles?: (paths: string[]) => void;
  } = $props();

  const add = (paths: string[]) => (onfiles ?? addPaths)(paths);

  async function chooseFiles() {
    const paths = await api.pickFiles(filter);
    if (paths.length) add(paths);
  }

  async function chooseFolder() {
    const path = await api.pickFolder();
    if (path) add([path]);
  }
</script>

<div class="dropzone" class:active={app.dragging} class:compact>
  <p>{label}</p>
  <div class="buttons">
    <button class="btn small" onclick={chooseFiles}>Choose files</button>
    {#if folders}
      <button class="btn small" onclick={chooseFolder}>Choose a folder</button>
    {/if}
  </div>
</div>
