<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./lib/api";
  import { app, init } from "./lib/state.svelte";
  import Header from "./components/Header.svelte";
  import Tabs from "./components/Tabs.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import Footer from "./components/Footer.svelte";
  import ConvertView from "./components/ConvertView.svelte";
  import FormatPage from "./components/FormatPage.svelte";
  import SettingsView from "./components/SettingsView.svelte";
  import PdfTools from "./components/PdfTools.svelte";

  let error = $state("");

  onMount(() => {
    init().catch((e) => (error = String(e)));
  });
</script>

<div class="shell">
  <Header />
  <Tabs />
  <div class="body">
    <Sidebar />
    <main class="main" id="main">
      <div class="main-inner">
        {#if app.update}
          <div class="note">
            <p>
              Version {app.update.version} is available.
              <button class="link" onclick={() => api.openUrl(app.update!.url)}>Download it from GitHub</button>.
            </p>
          </div>
        {/if}
        {#if error}
          <div class="warning"><p>The app could not start: {error}</p></div>
        {:else if !app.registry}
          <p class="muted">Loading</p>
        {:else if app.view === "settings"}
          <SettingsView />
        {:else if app.view === "format" && app.formatPage}
          <FormatPage id={app.formatPage} />
        {:else if app.tab === "pdf"}
          <PdfTools />
        {:else}
          <ConvertView />
        {/if}
      </div>
    </main>
  </div>
  <Footer />
</div>

{#if app.dragging}
  <div class="drop-overlay">Drop to add the files</div>
{/if}
