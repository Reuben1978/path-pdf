<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
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

    // Emitted by the Rust side (tauri-plugin-single-instance) when the OS
    // launches a second instance -- e.g. double-clicking another PDF while
    // this window is already open. The second process exits immediately;
    // this window gets focused and opens the file instead.
    await listen<string>("open-file", (event) => {
      docState.open(event.payload);
    });
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
