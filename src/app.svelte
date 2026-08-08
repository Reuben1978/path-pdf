<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import Viewer from "./lib/viewer/Viewer.svelte";
  import PagesPanel from "./lib/panel/PagesPanel.svelte";
  import SignatureLibrary from "./lib/tools/SignatureLibrary.svelte";
  import TabBar from "./lib/tabs/TabBar.svelte";
  import { docState, tabsState } from "./lib/doc-state.svelte";
  import { takeLaunchFile } from "./lib/ipc";

  // How long the branded splash overlay stays up. It's baked into
  // index.html so it paints with the window's first frame; this just takes
  // it away again once the app underneath is worth looking at.
  const SPLASH_MS = 1500;
  const SPLASH_FADE_MS = 250; // keep in sync with #splash's transition

  function dismissSplashOverlay() {
    const splash = document.getElementById("splash");
    if (!splash) return;
    setTimeout(() => {
      splash.classList.add("hide");
      setTimeout(() => splash.remove(), SPLASH_FADE_MS);
    }, SPLASH_MS);
  }

  onMount(async () => {
    dismissSplashOverlay();

    // Register this before the launch-file work below, and unconditionally.
    // Tauri's emit() doesn't buffer events for listeners that don't exist
    // yet, so if this were registered after an awaited call that can fail
    // (see below), a single-instance-forwarded launch arriving in that
    // window would be silently dropped for the rest of this window's
    // lifetime -- there's no retry. Emitted by the Rust side
    // (tauri-plugin-single-instance) when the OS launches a second instance
    // -- e.g. double-clicking another PDF while this window is already
    // open. The second process exits immediately; this window gets focused
    // and opens the file instead.
    await listen<string>("open-file", (event) => {
      tabsState.openNewTab(event.payload);
    });

    // `take_launch_file` requires the Rust-side AppState to be managed,
    // which happens on a background thread (see lib.rs) after PDFium binds
    // -- this component can mount and reach this point before that thread
    // finishes, so the very first call routinely loses that race. Retry a
    // few times rather than treating one failure as "nothing to open".
    let launchFile: string | null = null;
    for (let attempt = 0; attempt < 20; attempt++) {
      try {
        launchFile = await takeLaunchFile();
        break;
      } catch {
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
    }
    if (launchFile) {
      await tabsState.openNewTab(launchFile);
    }
  });
</script>

<main>
  <div class="content">
    {#if tabsState.tabs.length > 0}
      <TabBar />
    {/if}
    <div class="panes">
      {#if docState.id !== null}
        <PagesPanel />
      {/if}
      <Viewer />
    </div>
  </div>
  <SignatureLibrary />
</main>

<style>
  main {
    height: 100vh;
    display: flex;
  }

  .content {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .panes {
    display: flex;
    min-height: 0;
    flex: 1;
  }
</style>
