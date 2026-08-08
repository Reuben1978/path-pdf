<script lang="ts">
  import { onMount } from "svelte";
  import { emit, listen } from "@tauri-apps/api/event";
  import Viewer from "./lib/viewer/Viewer.svelte";
  import PagesPanel from "./lib/panel/PagesPanel.svelte";
  import SignatureLibrary from "./lib/tools/SignatureLibrary.svelte";
  import { docState } from "./lib/doc-state.svelte";
  import { takeLaunchFile } from "./lib/ipc";

  onMount(async () => {
    // Tells the Rust side it's now safe to show this window -- see the
    // "frontend-ready" listener in lib.rs. Deliberately not based on
    // Tauri's on_page_load(Finished): that fires once the webview finishes
    // loading its HTML/JS/CSS, which can happen before Svelte has actually
    // mounted -- reaching onMount at all already means the DOM has been
    // built, a strictly later and more accurate point.
    //
    // Deliberately NOT gated on requestAnimationFrame either, despite that
    // being the usual way to wait for a confirmed paint: this window is
    // still hidden at this point (Rust shows it only after this signal),
    // and rAF callbacks never fire for a window that was never shown --
    // there's no compositor paint cycle to hook for something the OS isn't
    // presenting. Waiting on it here deadlocked the whole startup
    // permanently (found by testing, not by inspection -- the window
    // simply never appeared).
    await emit("frontend-ready");

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
      docState.open(event.payload);
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
