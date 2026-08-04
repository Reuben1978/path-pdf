<script lang="ts">
  import { printState } from "./print-state.svelte";
</script>

<div class="print-area">
  {#each printState.pageImages as src, i (i)}
    <img {src} alt="Page {i + 1}" />
  {/each}
</div>

<style>
  /* Invisible on screen -- this only exists to be the sole thing left
     visible once @media print hides the rest of the document below. */
  .print-area {
    display: none;
  }

  @media print {
    :global(body *) {
      visibility: hidden;
    }

    :global(.print-area),
    :global(.print-area *) {
      visibility: visible;
    }

    :global(.print-area) {
      display: block !important;
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
    }

    :global(.print-area img) {
      display: block;
      width: 100%;
      page-break-after: always;
    }

    :global(.print-area img:last-child) {
      page-break-after: auto;
    }
  }
</style>
