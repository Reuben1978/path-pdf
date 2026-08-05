<script lang="ts">
  import { renderPage, placeSignature, resizeSignatureAnnotation } from "../ipc";
  import { docState } from "../doc-state.svelte";
  import { placeText } from "../tools/typewriter";
  import { pixelToPdfPoint, SIGNATURE_DRAG_MIME } from "../tools/signature";
  import { signatureLibrary } from "../tools/signature-state.svelte";
  import { toolState } from "../tools/tool-state.svelte";
  import { typewriterSettings } from "../tools/typewriter-settings.svelte";

  let { pageIndex, widthPoints, heightPoints }: {
    pageIndex: number;
    widthPoints: number;
    heightPoints: number;
  } = $props();

  const DEFAULT_SIGNATURE_WIDTH_POINTS = 150;
  const MIN_SIGNATURE_WIDTH_PX = 20;

  type Corner = "tl" | "tr" | "bl" | "br";

  type PlacedSignature = {
    filename: string;
    annotationIndex: number;
    aspect: number; // height / width, fixed at drop time
    xPx: number;
    yPx: number;
    widthPx: number;
    heightPx: number;
  };

  let wrapper: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let hasRendered = false;

  // The target width doRender() last actually rendered at, so the resize
  // observer below can tell a real layout change from a no-op observation
  // (it fires once immediately on observe(), before anything has moved).
  let lastRenderedWidth = 0;

  // Set from the actual render response; used for pixel<->point conversion
  // inside event handlers only (never read in the template), so this
  // doesn't need to be reactive $state. Falls back to the cheap pre-fetched
  // size until the first render lands.
  let renderedWidthPoints = (() => widthPoints)();
  let renderedHeightPoints = (() => heightPoints)();

  let pendingClick = $state<{ x: number; y: number } | null>(null);
  let pendingText = $state("");
  let dragOverSignature = $state<{ x: number; y: number; width: number; height: number } | null>(null);

  // The signature just dropped, if any -- immediately selected with resize
  // handles, no extra click needed. Only ever tracks the most recent drop
  // (this page's own, since a drop elsewhere sets state in that PageSlot
  // instance instead); re-selecting an older placement isn't in scope.
  let placedSignature = $state<PlacedSignature | null>(null);

  async function doRender() {
    if (docState.id === null || !canvas || !wrapper) return;
    const targetWidth = Math.max(100, Math.round(wrapper.clientWidth));
    const page = await renderPage(docState.id, pageIndex, targetWidth);
    canvas.width = page.width;
    canvas.height = page.height;
    renderedWidthPoints = page.pageWidthPoints;
    renderedHeightPoints = page.pageHeightPoints;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.putImageData(new ImageData(new Uint8ClampedArray(page.rgba), page.width, page.height), 0, 0);
    hasRendered = true;
    lastRenderedWidth = targetWidth;
  }

  // Re-render if this page's content changed (e.g. an annotation was added)
  // after it had already been rendered once.
  $effect(() => {
    docState.pageDirtyTick[pageIndex];
    if (hasRendered) doRender();
  });

  // Lazy-render only when this page scrolls near the viewport, per
  // CLAUDE.md's "never render the whole document up front" rule. Also
  // tracks which page is most visible, to drive the page counter.
  function lazyRender(node: HTMLDivElement) {
    const root = node.closest(".canvas-scroll");
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting && !hasRendered) {
            doRender();
          }
          if (entry.intersectionRatio > 0.5) {
            docState.currentPage = pageIndex;
          }
        }
      },
      { root, rootMargin: "600px 0px", threshold: [0, 0.5] },
    );
    observer.observe(node);

    // A page can render its very first frame before the window has settled
    // into its final size -- e.g. opening a file via double-click fires
    // app.svelte's auto-open on mount, earlier in the layout lifecycle than
    // a manual Open... click ever happens, so the first IntersectionObserver
    // callback can land while the window is still mid-resize. That bakes a
    // too-small target width into the rendered bitmap, which CSS then
    // stretches to fill the real (larger) space -- a blurry page. Re-render
    // whenever the wrapper's actual width settles somewhere meaningfully
    // different from what was last rendered, debounced so a window drag
    // doesn't spam re-renders on every intermediate frame.
    let resizeTimer: ReturnType<typeof setTimeout> | undefined;
    const resizeObserver = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (!width || !hasRendered) return;
      if (Math.abs(width - lastRenderedWidth) < lastRenderedWidth * 0.05) return;
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(doRender, 200);
    });
    resizeObserver.observe(node);

    return {
      destroy() {
        observer.disconnect();
        resizeObserver.disconnect();
        clearTimeout(resizeTimer);
      },
    };
  }

  function onCanvasClick(event: MouseEvent) {
    if (placedSignature) placedSignature = null;
    if (!toolState.textToolActive || docState.id === null) return;
    pendingClick = { x: event.offsetX, y: event.offsetY };
    pendingText = "";
  }

  async function commitPendingText() {
    if (!pendingClick || docState.id === null || pendingText.trim() === "") {
      pendingClick = null;
      return;
    }
    try {
      await placeText(
        docState.id,
        pageIndex,
        pendingClick.x,
        pendingClick.y,
        canvas.width,
        canvas.height,
        renderedWidthPoints,
        renderedHeightPoints,
        pendingText,
        typewriterSettings.fontSize,
        typewriterSettings.fontName,
      );
      pendingClick = null;
      pendingText = "";
      docState.touchPage(pageIndex);
    } catch (e) {
      docState.error = String(e);
    }
  }

  function cancelPendingText() {
    pendingClick = null;
    pendingText = "";
  }

  function focusOnMount(node: HTMLElement) {
    node.focus();
  }

  function onCanvasDragOver(event: DragEvent) {
    if (docState.id === null) return;
    if (!event.dataTransfer?.types.includes(SIGNATURE_DRAG_MIME)) return;
    event.preventDefault();
    event.dataTransfer.dropEffect = "copy";

    const sig = signatureLibrary.selected;
    if (!sig) return;
    const aspect = sig.height / sig.width;
    const widthPx = (DEFAULT_SIGNATURE_WIDTH_POINTS / renderedWidthPoints) * canvas.width;
    const heightPx = widthPx * aspect;
    dragOverSignature = {
      x: event.offsetX - widthPx / 2,
      y: event.offsetY - heightPx / 2,
      width: widthPx,
      height: heightPx,
    };
  }

  function onCanvasDragLeave() {
    dragOverSignature = null;
  }

  async function onCanvasDrop(event: DragEvent) {
    event.preventDefault();
    const filename = event.dataTransfer?.getData(SIGNATURE_DRAG_MIME);
    dragOverSignature = null;
    placedSignature = null;
    if (!filename || docState.id === null) return;

    const sig = signatureLibrary.signatures.find((s) => s.filename === filename);
    if (!sig) return;

    const aspect = sig.height / sig.width;
    const widthPoints = DEFAULT_SIGNATURE_WIDTH_POINTS;
    const heightPoints = widthPoints * aspect;
    const widthPx = (widthPoints / renderedWidthPoints) * canvas.width;
    const heightPx = (heightPoints / renderedHeightPoints) * canvas.height;
    const xPx = event.offsetX - widthPx / 2;
    const yPx = event.offsetY - heightPx / 2;

    const { x, y } = pixelToPdfPoint(
      xPx,
      yPx + heightPx,
      canvas.width,
      canvas.height,
      renderedWidthPoints,
      renderedHeightPoints,
    );

    try {
      const annotationIndex = await placeSignature(docState.id, pageIndex, filename, x, y, widthPoints, heightPoints);
      docState.touchPage(pageIndex);
      placedSignature = { filename, annotationIndex, aspect, xPx, yPx, widthPx, heightPx };
    } catch (e) {
      docState.error = String(e);
    }
  }

  // Corner-drag resize of the just-placed signature. Only the CSS overlay
  // box updates live on pointermove -- calling the backend (and therefore
  // re-rasterizing the whole page) on every move would blow the 60fps
  // rendering budget. The real resize commits once, on pointerup.
  function onHandlePointerDown(event: PointerEvent, corner: Corner) {
    if (!placedSignature) return;
    event.preventDefault();
    event.stopPropagation();

    const aspect = placedSignature.aspect;
    const anchorX = corner.includes("l") ? placedSignature.xPx + placedSignature.widthPx : placedSignature.xPx;
    const anchorY = corner.includes("t") ? placedSignature.yPx + placedSignature.heightPx : placedSignature.yPx;

    function onMove(moveEvent: PointerEvent) {
      if (!placedSignature) return;
      const rect = canvas.getBoundingClientRect();
      const pointerX = moveEvent.clientX - rect.left;

      const widthPx = Math.max(MIN_SIGNATURE_WIDTH_PX, Math.abs(pointerX - anchorX));
      const heightPx = widthPx * aspect;
      const xPx = corner.includes("l") ? anchorX - widthPx : anchorX;
      const yPx = corner.includes("t") ? anchorY - heightPx : anchorY;

      placedSignature = { ...placedSignature, xPx, yPx, widthPx, heightPx };
    }

    function onUp() {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      commitResize();
    }

    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  }

  async function commitResize() {
    if (!placedSignature || docState.id === null) return;
    const sig = placedSignature;

    const { x, y } = pixelToPdfPoint(
      sig.xPx,
      sig.yPx + sig.heightPx,
      canvas.width,
      canvas.height,
      renderedWidthPoints,
      renderedHeightPoints,
    );
    const widthPoints = (sig.widthPx / canvas.width) * renderedWidthPoints;
    const heightPoints = (sig.heightPx / canvas.height) * renderedHeightPoints;

    try {
      const annotationIndex = await resizeSignatureAnnotation(
        docState.id,
        pageIndex,
        sig.annotationIndex,
        sig.filename,
        x,
        y,
        widthPoints,
        heightPoints,
      );
      placedSignature = { ...sig, annotationIndex };
      docState.touchPage(pageIndex);
    } catch (e) {
      docState.error = String(e);
    }
  }

  function onWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && placedSignature) {
      placedSignature = null;
    }
  }
</script>

<svelte:window onkeydown={onWindowKeydown} />

<div
  class="page-slot"
  bind:this={wrapper}
  use:lazyRender
  data-page-index={pageIndex}
  style:aspect-ratio="{widthPoints} / {heightPoints}"
>
  <canvas
    bind:this={canvas}
    class:text-tool={toolState.textToolActive}
    onclick={onCanvasClick}
    ondragover={onCanvasDragOver}
    ondragleave={onCanvasDragLeave}
    ondrop={onCanvasDrop}
  ></canvas>
  {#if dragOverSignature}
    <div
      class="signature-preview"
      style:left="{dragOverSignature.x}px"
      style:top="{dragOverSignature.y}px"
      style:width="{dragOverSignature.width}px"
      style:height="{dragOverSignature.height}px"
    ></div>
  {/if}
  {#if placedSignature}
    <div
      class="signature-selection"
      style:left="{placedSignature.xPx}px"
      style:top="{placedSignature.yPx}px"
      style:width="{placedSignature.widthPx}px"
      style:height="{placedSignature.heightPx}px"
    >
      {#each ["tl", "tr", "bl", "br"] as const as corner (corner)}
        <div
          class="resize-handle handle-{corner}"
          role="button"
          tabindex="-1"
          aria-label="Resize signature"
          onpointerdown={(e) => onHandlePointerDown(e, corner)}
        ></div>
      {/each}
    </div>
  {/if}
  {#if pendingClick}
    <textarea
      class="text-input"
      style:left="{pendingClick.x}px"
      style:top="{pendingClick.y}px"
      style:font-size="{typewriterSettings.fontSize}px"
      bind:value={pendingText}
      onblur={commitPendingText}
      onkeydown={(e) => {
        if (e.key === "Enter" && !e.shiftKey) {
          e.preventDefault();
          commitPendingText();
        } else if (e.key === "Escape") {
          cancelPendingText();
        }
      }}
      use:focusOnMount
    ></textarea>
  {/if}
</div>

<style>
  .page-slot {
    position: relative;
    width: 100%;
    box-shadow: 0 0 8px rgba(0, 0, 0, 0.6);
  }

  canvas {
    display: block;
    width: 100%;
    height: 100%;
  }

  canvas.text-tool {
    cursor: text;
  }

  .signature-preview {
    position: absolute;
    border: 2px dashed #3b82f6;
    background: rgba(59, 130, 246, 0.15);
    pointer-events: none;
  }

  .signature-selection {
    position: absolute;
    border: 2px solid #3b82f6;
  }

  .resize-handle {
    position: absolute;
    width: 12px;
    height: 12px;
    border: 2px solid #3b82f6;
    border-radius: 50%;
    background: white;
    touch-action: none;
  }

  .handle-tl {
    top: -7px;
    left: -7px;
    cursor: nwse-resize;
  }

  .handle-tr {
    top: -7px;
    right: -7px;
    cursor: nesw-resize;
  }

  .handle-bl {
    bottom: -7px;
    left: -7px;
    cursor: nesw-resize;
  }

  .handle-br {
    bottom: -7px;
    right: -7px;
    cursor: nwse-resize;
  }

  .text-input {
    position: absolute;
    min-width: 120px;
    min-height: 1.6em;
    padding: 2px 4px;
    border: 1px solid #3b82f6;
    background: rgba(255, 255, 255, 0.95);
    color: black;
    font-family: Helvetica, Arial, sans-serif;
    resize: both;
  }
</style>
