<script lang="ts">
  import { onMount } from "svelte";
  import { emit, listen } from "@tauri-apps/api/event";
  import Viewer from "./lib/viewer/Viewer.svelte";
  import PagesPanel from "./lib/panel/PagesPanel.svelte";
  import SignatureLibrary from "./lib/tools/SignatureLibrary.svelte";
  import { docState } from "./lib/doc-state.svelte";
  import { takeLaunchFile } from "./lib/ipc";

  // Resolves once the browser has actually committed a paint. A single
  // requestAnimationFrame only promises "before the next paint", so it can
  // still run ahead of the paint that makes this frame's DOM visible; two
  // nested calls land after it.
  //
  // The timeout is not decoration. rAF does not fire while a window is
  // hidden, and an earlier version of this waited on rAF before the window
  // was ever shown -- which deadlocked startup permanently, since the app
  // then never signalled that it was safe to reveal it. This window *is*
  // shown before this runs now (Rust shows it behind the splash, see
  // lib.rs), so rAF should fire promptly -- but startup must never again
  // be able to hang on a frame that doesn't arrive.
  function waitForPaint(timeoutMs = 2000): Promise<void> {
    return new Promise((resolve) => {
      let settled = false;
      const finish = () => {
        if (settled) return;
        settled = true;
        resolve();
      };
      requestAnimationFrame(() => requestAnimationFrame(finish));
      setTimeout(finish, timeoutMs);
    });
  }

  onMount(async () => {
    // Tells the Rust side the window has real content on it, so the splash
    // covering it can be dismissed -- see the "frontend-painted" listener
    // in lib.rs. Deliberately not Tauri's on_page_load(Finished), which
    // fires once the webview has loaded HTML/JS/CSS and can precede Svelte
    // mounting anything; and deliberately after a confirmed paint rather
    // than merely at mount, because mount only means the DOM exists, not
    // that pixels have been drawn.
    waitForPaint().then(() => emit("frontend-painted"));

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
