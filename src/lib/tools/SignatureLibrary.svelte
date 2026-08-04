<script lang="ts">
  import { onMount } from "svelte";
  import { importSignature, saveDrawnSignature, deleteSignature, getSignatureBytes } from "../ipc";
  import { signatureLibrary } from "./signature-state.svelte";
  import { SIGNATURE_DRAG_MIME } from "./signature";
  import { toolState } from "./tool-state.svelte";
  import { typewriterSettings, FONT_LABELS, FONT_SIZES } from "./typewriter-settings.svelte";
  import Button from "../components/Button.svelte";

  let collapsed = $state(false);
  let importPath = $state("");
  let thumbnails = $state<Record<string, string>>({});
  let error = $state<string | null>(null);

  let drawCanvas: HTMLCanvasElement = $state()!;
  let drawing = false;
  let hasDrawn = $state(false);

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    try {
      await signatureLibrary.refresh();
      for (const sig of signatureLibrary.signatures) {
        if (!thumbnails[sig.filename]) {
          const bytes = await getSignatureBytes(sig.filename);
          const blob = new Blob([Uint8Array.from(bytes)], { type: "image/png" });
          thumbnails = { ...thumbnails, [sig.filename]: URL.createObjectURL(blob) };
        }
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function doImport() {
    if (!importPath) return;
    try {
      await importSignature(importPath);
      importPath = "";
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  async function doDelete(filename: string) {
    try {
      await deleteSignature(filename);
      if (thumbnails[filename]) {
        URL.revokeObjectURL(thumbnails[filename]);
        const { [filename]: _removed, ...rest } = thumbnails;
        thumbnails = rest;
      }
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }

  function selectSignature(filename: string) {
    signatureLibrary.selectedFilename = filename;
  }

  function onThumbDragStart(event: DragEvent, filename: string) {
    if (!event.dataTransfer) return;
    event.dataTransfer.setData(SIGNATURE_DRAG_MIME, filename);
    event.dataTransfer.setData("text/plain", filename);
    event.dataTransfer.effectAllowed = "copy";
    selectSignature(filename);
  }

  function drawStart(event: MouseEvent) {
    drawing = true;
    const ctx = drawCanvas.getContext("2d");
    if (!ctx) return;
    ctx.strokeStyle = "#1e3a8a";
    ctx.lineWidth = 3;
    ctx.lineCap = "round";
    ctx.lineJoin = "round";
    ctx.beginPath();
    ctx.moveTo(event.offsetX, event.offsetY);
  }

  function drawMove(event: MouseEvent) {
    if (!drawing) return;
    const ctx = drawCanvas.getContext("2d");
    if (!ctx) return;
    ctx.lineTo(event.offsetX, event.offsetY);
    ctx.stroke();
    hasDrawn = true;
  }

  function drawEnd() {
    drawing = false;
  }

  function clearDrawing() {
    const ctx = drawCanvas.getContext("2d");
    if (!ctx) return;
    ctx.clearRect(0, 0, drawCanvas.width, drawCanvas.height);
    hasDrawn = false;
  }

  async function saveDrawing() {
    if (!hasDrawn) return;
    drawCanvas.toBlob(async (blob) => {
      if (!blob) return;
      try {
        const buffer = await blob.arrayBuffer();
        await saveDrawnSignature(new Uint8Array(buffer));
        clearDrawing();
        await refresh();
      } catch (e) {
        error = String(e);
      }
    }, "image/png");
  }
</script>

{#if collapsed}
  <button class="expand-tab" onclick={() => (collapsed = false)} title="Open signatures panel">
    Signatures ‹
  </button>
{:else}
  <div class="library">
    <div class="library-header">
      <h3>Signatures</h3>
      <button class="collapse" onclick={() => (collapsed = true)} title="Close signatures panel">›</button>
    </div>
    {#if error}
      <p class="error">{error}</p>
    {/if}

    {#if signatureLibrary.signatures.length > 0}
      <p class="hint">Drag one onto the page to place it.</p>
    {/if}
    <div class="thumbs">
      {#each signatureLibrary.signatures as sig (sig.filename)}
        <div
          class="thumb"
          class:selected={signatureLibrary.selectedFilename === sig.filename}
          draggable="true"
          role="button"
          tabindex="0"
          aria-label="Signature {sig.filename}, draggable onto the page"
          ondragstart={(e) => onThumbDragStart(e, sig.filename)}
          onclick={() => selectSignature(sig.filename)}
          onkeydown={(e) => {
            if (e.key === "Enter" || e.key === " ") {
              e.preventDefault();
              selectSignature(sig.filename);
            }
          }}
        >
          {#if thumbnails[sig.filename]}
            <img src={thumbnails[sig.filename]} alt="Signature" draggable="false" />
          {/if}
          <span
            class="delete"
            onclick={(e) => {
              e.stopPropagation();
              doDelete(sig.filename);
            }}
            role="none"
          >
            ×
          </span>
        </div>
      {/each}
    </div>

    <div class="import">
      <input type="text" placeholder="/path/to/signature.png" bind:value={importPath} />
      <Button onclick={doImport}>Import</Button>
    </div>

    <div class="draw">
      <p class="hint">Or draw one:</p>
      <canvas
        bind:this={drawCanvas}
        width="240"
        height="80"
        onmousedown={drawStart}
        onmousemove={drawMove}
        onmouseup={drawEnd}
        onmouseleave={drawEnd}
      ></canvas>
      <div class="draw-actions">
        <Button onclick={clearDrawing} disabled={!hasDrawn}>Clear</Button>
        <Button onclick={saveDrawing} disabled={!hasDrawn}>Save</Button>
      </div>
    </div>

    <div class="text-tool-row">
      <Button
        active={toolState.textToolActive}
        onclick={() => (toolState.textToolActive = !toolState.textToolActive)}
        title="Click a page to add text"
      >
        Text
      </Button>
      <select bind:value={typewriterSettings.fontName} title="Font">
        {#each Object.entries(FONT_LABELS) as [value, label] (value)}
          <option {value}>{label}</option>
        {/each}
      </select>
      <select bind:value={typewriterSettings.fontSize} title="Size">
        {#each FONT_SIZES as size (size)}
          <option value={size}>{size}pt</option>
        {/each}
      </select>
    </div>
  </div>
{/if}

<style>
  .expand-tab {
    writing-mode: vertical-rl;
    padding: var(--space-sm, 0.5rem) 6px;
    border: none;
    border-left: 1px solid var(--color-border, #333);
    background: var(--color-panel-bg, #211d2c);
    color: var(--color-text, #e5e5e7);
    cursor: pointer;
    font-size: 13px;
  }

  .library {
    display: flex;
    flex-direction: column;
    gap: var(--space-sm, 0.5rem);
    width: 260px;
    min-width: 260px;
    height: 100%;
    padding: var(--space-sm, 0.5rem);
    border-left: 1px solid var(--color-border, #333);
    background: var(--color-panel-bg, #211d2c);
    overflow-y: auto;
  }

  .library-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h3 {
    margin: 0;
    font-size: 14px;
  }

  .collapse {
    border: none;
    background: none;
    color: inherit;
    cursor: pointer;
    font-size: 16px;
    padding: 0 4px;
  }

  .error {
    color: var(--color-error, #f87171);
    font-size: 12px;
    margin: 0;
  }

  .thumbs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .thumb {
    position: relative;
    width: 74px;
    height: 50px;
    padding: 2px;
    border: 2px solid var(--color-border, #3a3b3f);
    border-radius: 4px;
    background: var(--color-thumb-bg, #f5f5f5);
    cursor: grab;
  }

  .thumb.selected {
    border-color: #3b82f6;
  }

  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: contain;
  }

  .thumb .delete {
    position: absolute;
    top: -6px;
    right: -6px;
    width: 16px;
    height: 16px;
    line-height: 16px;
    text-align: center;
    border-radius: 50%;
    background: #c0392b;
    color: white;
    font-size: 12px;
    cursor: pointer;
  }

  .import,
  .draw-actions {
    display: flex;
    gap: 4px;
  }

  .import input {
    flex: 1;
    min-width: 0;
  }

  .hint {
    font-size: 12px;
    opacity: 0.7;
    margin: 0;
  }

  .draw canvas {
    border: 1px dashed var(--color-border, #3a3b3f);
    background: var(--color-thumb-bg, #f5f5f5);
    cursor: crosshair;
  }

  .text-tool-row {
    display: flex;
    gap: 4px;
    align-items: center;
    flex-wrap: wrap;
  }

  .text-tool-row select {
    min-width: 0;
    font-size: 12px;
  }
</style>
