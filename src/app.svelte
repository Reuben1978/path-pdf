<script lang="ts">
  import { onMount } from "svelte";
  import Viewer from "./lib/viewer/Viewer.svelte";
  import PagesPanel from "./lib/panel/PagesPanel.svelte";
  import SignatureLibrary from "./lib/tools/SignatureLibrary.svelte";
  import { docState } from "./lib/doc-state.svelte";
  import { takeLaunchFile } from "./lib/ipc";

  onMount(async () => {
    const launchFile = await takeLaunchFile();
    if (launchFile) {
      await docState.open(launchFile);
    }
  });
</script>

<main>
  {#if docState.id !== null}
    <PagesPanel />
  {/if}
  <Viewer />
  <SignatureLibrary />
</main>

<style>
  main {
    height: 100vh;
    display: flex;
  }
</style>
