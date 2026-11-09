<script lang="ts">
  import { api } from "../lib/api";
  import { app } from "../lib/state.svelte";

  async function licenses() {
    api.openPath(await api.licensesFolder());
  }
</script>

<footer class="footer">
  <span>Files are converted on this computer. Nothing is uploaded.</span>
  <span class="spacer"></span>
  <span>Version {app.version}</span>
  <button onclick={licenses}>Licenses</button>
  <button onclick={() => api.openUrl("https://github.com/faiyaz-svg/convert-everything")}>Source code</button>
</footer>
