<script lang="ts">
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { saveDocument, saveDocumentAs } from "../ipc";
  import { docState, tabsState } from "../doc-state.svelte";
  import { listAnnotations, removeAnnotation } from "../tools/typewriter";
  import { viewerState } from "./viewer-state.svelte";
  import PageSlot from "./PageSlot.svelte";
  import StartScreen from "../start/StartScreen.svelte";
  import PrintArea from "../print/PrintArea.svelte";
  import { printDocument, printState } from "../print/print-state.svelte";
  import Button from "../components/Button.svelte";

  const PDF_FILTER = { name: "PDF", extensions: ["pdf"] };

  let scrollContainer: HTMLDivElement = $state()!;
  let annotations = $state<{ annotationIndex: number; contents: string }[]>([]);

  let saveLayers = $state(true);
  let saveStatus = $state<string | null>(null);

  async function openFile() {
    const path = await openDialog({ multiple: false, filters: [PDF_FILTER] });
    if (!path) return;
    await tabsState.openNewTab(path);
  }

  async function refreshAnnotations() {
    if (docState.id === null) return;
    try {
      annotations = await listAnnotations(docState.id, docState.currentPage);
    } catch (e) {
      docState.error = String(e);
    }
  }

  // Refresh the current page's annotation list whenever the visible page
  // changes (currentPage is kept live by PageSlot's IntersectionObserver) or
  // that page's content is touched.
  $effect(() => {
    docState.currentPage;
    docState.id;
    docState.pageDirtyTick[docState.currentPage];
    refreshAnnotations();
  });

  // Scrolls scrollContainer directly by the needed delta rather than calling
  // el.scrollIntoView(), which walks every scrollable ancestor -- including
  // the document itself -- and can leave the whole window nudged down with
  // no code that ever resets it.
  function scrollToPage(index: number) {
    if (!scrollContainer) return;
    const el = scrollContainer.querySelector(`[data-page-index="${index}"]`);
    if (!el) return;
    const delta = el.getBoundingClientRect().top - scrollContainer.getBoundingClientRect().top;
    scrollContainer.scrollBy({ top: delta, behavior: "smooth" });
  }

  // Explicit navigation (pages panel thumbnail click, Prev/Next) scrolls the
  // viewer. Deliberately watches scrollRequest, not currentPage directly --
  // currentPage is also updated as a side effect of scrolling itself (by
  // PageSlot's IntersectionObserver), so watching it here would feed back
  // into itself. See doc-state.svelte.ts.
  $effect(() => {
    if (docState.scrollRequest) {
      scrollToPage(docState.scrollRequest.page);
    }
  });

  async function deleteAnnotation(annotationIndex: number) {
    if (docState.id === null) return;
    try {
      await removeAnnotation(docState.id, docState.currentPage, annotationIndex);
      docState.touchPage(docState.currentPage);
      await refreshAnnotations();
    } catch (e) {
      docState.error = String(e);
    }
  }

  async function doSave() {
    if (docState.id === null) return;
    try {
      await saveDocument(docState.id, !saveLayers);
      saveStatus = `Saved to ${docState.path}`;
    } catch (e) {
      docState.error = String(e);
    }
  }

  async function doSaveAs() {
    if (docState.id === null) return;
    const path = await saveDialog({ filters: [PDF_FILTER], defaultPath: "untitled.pdf" });
    if (!path) return;
    try {
      await saveDocumentAs(docState.id, path, !saveLayers);
      saveStatus = `Saved to ${path}`;
    } catch (e) {
      docState.error = String(e);
    }
  }

  async function doPrint() {
    if (docState.id === null) return;
    try {
      await printDocument(docState.id, docState.pageSizes);
    } catch (e) {
      docState.error = String(e);
    }
  }

  // Ctrl+scroll to zoom -- the standard cross-app convention (browsers, most
  // PDF viewers), which is also why plain scrolling is left untouched for
  // normal page navigation.
  function onWheel(event: WheelEvent) {
    if (!event.ctrlKey) return;
    event.preventDefault();
    viewerState.zoomBy(event.deltaY > 0 ? -0.1 : 0.1);
  }

  // Ctrl+= / Ctrl+- (and the numpad/shifted variants) to zoom, Ctrl+0 to
  // reset -- only while a document is open, so it doesn't shadow browser-
  // devtools-style shortcuts on an empty window.
  function onKeydown(event: KeyboardEvent) {
    if (!event.ctrlKey || docState.pageCount === 0) return;
    if (event.key === "+" || event.key === "=") {
      event.preventDefault();
      viewerState.zoomIn();
    } else if (event.key === "-") {
      event.preventDefault();
      viewerState.zoomOut();
    } else if (event.key === "0") {
      event.preventDefault();
      viewerState.reset();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="viewer">
  <div class="toolbar">
    <Button onclick={openFile}>Open…</Button>
    {#if docState.path}
      <span class="doc-path" title={docState.path}>{docState.path}</span>
    {/if}
    {#if docState.pageCount > 0}
      <Button
        onclick={() => docState.navigateToPage(Math.max(0, docState.currentPage - 1))}
        disabled={docState.currentPage === 0}
      >
        Prev
      </Button>
      <span>{docState.currentPage + 1} / {docState.pageCount}</span>
      <Button
        onclick={() => docState.navigateToPage(Math.min(docState.pageCount - 1, docState.currentPage + 1))}
        disabled={docState.currentPage === docState.pageCount - 1}
      >
        Next
      </Button>
      <Button icon onclick={() => viewerState.zoomOut()} title="Zoom out (Ctrl+-)">−</Button>
      <Button class="zoom-level" onclick={() => viewerState.reset()} title="Reset zoom (Ctrl+0)">
        {Math.round(viewerState.zoom * 100)}%
      </Button>
      <Button icon onclick={() => viewerState.zoomIn()} title="Zoom in (Ctrl++)">+</Button>
    {/if}
  </div>
  {#if docState.pageCount > 0}
    <div class="save-bar">
      <label class="save-layers-toggle">
        <input type="checkbox" bind:checked={saveLayers} />
        Save layers
      </label>
      <Button onclick={doSave}>Save</Button>
      <Button onclick={doSaveAs}>Save As…</Button>
      <Button onclick={doPrint} disabled={printState.preparing}>
        {printState.preparing ? "Preparing…" : "Print"}
      </Button>
      {#if saveStatus}
        <span class="save-status">{saveStatus}</span>
      {/if}
    </div>
  {/if}
  {#if docState.error}
    <p class="error">{docState.error}</p>
  {/if}
  {#if annotations.length > 0}
    <div class="annotations-list">
      {#each annotations as annotation (annotation.annotationIndex)}
        <span class="annotation-chip">
          {annotation.contents}
          <button onclick={() => deleteAnnotation(annotation.annotationIndex)} title="Delete">×</button>
        </span>
      {/each}
    </div>
  {/if}
  {#if docState.id === null}
    <StartScreen />
  {:else}
    <div class="canvas-scroll" bind:this={scrollContainer} onwheel={onWheel}>
      <div class="page-stack" style:zoom={viewerState.zoom}>
        {#each docState.pageSizes as size, i (i)}
          <PageSlot pageIndex={i} widthPoints={size.widthPoints} heightPoints={size.heightPoints} />
        {/each}
      </div>
    </div>
  {/if}
  <PrintArea />
</div>

<style>
  .viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    flex: 1;
  }

  .toolbar {
    display: flex;
    gap: var(--space-sm, 0.5rem);
    align-items: center;
    padding: var(--space-sm, 0.5rem);
    border-bottom: 1px solid var(--color-border, #333);
  }

  :global(.zoom-level) {
    min-width: 3.5em;
    font-size: 12px;
  }

  .doc-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
    opacity: 0.7;
  }

  .save-bar {
    display: flex;
    gap: var(--space-sm, 0.5rem);
    align-items: center;
    padding: var(--space-sm, 0.5rem);
    border-bottom: 1px solid var(--color-border, #333);
    font-size: 13px;
  }

  .save-layers-toggle {
    display: flex;
    align-items: center;
    gap: 4px;
    white-space: nowrap;
  }

  .save-status {
    opacity: 0.7;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .error {
    color: var(--color-error, #f87171);
    padding: var(--space-sm, 0.5rem);
    margin: 0;
  }

  .annotations-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: var(--space-sm, 0.5rem);
    border-bottom: 1px solid var(--color-border, #333);
  }

  .annotation-chip {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 12px;
    background: var(--color-border, #2e303a);
    font-size: 12px;
  }

  .annotation-chip button {
    border: none;
    background: none;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 0;
    color: inherit;
  }

  .canvas-scroll {
    flex: 1;
    overflow-y: auto;
    overscroll-behavior: contain;
    background: var(--color-canvas-bg, #0f0d13);
  }

  .page-stack {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-md, 1rem);
    padding: var(--space-md, 1rem);
  }

  .page-stack :global(.page-slot) {
    max-width: 1100px;
  }
</style>
