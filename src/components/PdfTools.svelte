<script lang="ts">
  import { app } from "../lib/state.svelte";
  import PdfArrange from "./pdf/PdfArrange.svelte";
  import PdfBatch from "./pdf/PdfBatch.svelte";
  import PdfJoin from "./pdf/PdfJoin.svelte";
  import PdfSplit from "./pdf/PdfSplit.svelte";

  const INFO = {
    merge: { title: "Merge PDFs", intro: "Join several PDFs into one, in the order you choose. Pictures can go in too: each becomes a page." },
    split: { title: "Split a PDF", intro: "Save every page as its own PDF, or cut the file into ranges of pages." },
    arrange: { title: "Reorder and rotate pages", intro: "Move pages, turn sideways scans the right way up, and leave out pages you don't need." },
    images: { title: "Images to PDF", intro: "Put pictures into one PDF, one picture per page. Good for scans and photos of documents." },
    toImages: { title: "PDF to images", intro: "Save each page of a PDF as a picture." },
    compress: { title: "Compress PDFs", intro: "Make PDFs smaller by lowering the resolution of the pictures inside them. Text stays sharp." },
  } as const;

  const info = $derived(INFO[app.pdfTool]);
</script>

<h1>{info.title}</h1>
<p>{info.intro}</p>
<div class="prevnext"></div>

{#key app.pdfTool}
  {#if app.pdfTool === "merge"}
    <PdfJoin mode="merge" />
  {:else if app.pdfTool === "images"}
    <PdfJoin mode="images" />
  {:else if app.pdfTool === "split"}
    <PdfSplit />
  {:else if app.pdfTool === "arrange"}
    <PdfArrange />
  {:else if app.pdfTool === "toImages"}
    <PdfBatch mode="toImages" />
  {:else}
    <PdfBatch mode="compress" />
  {/if}
{/key}
