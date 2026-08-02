<script lang="ts">
  import { save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { renderPage, reorderPages, deletePages, rotatePage, extractPages } from "../ipc";
  import { docState } from "../doc-state.svelte";

  const PDF_FILTER = { name: "PDF", extensions: ["pdf"] };

  let selected = $state<Set<number>>(new Set());
  let dragIndex = $state<number | null>(null);
  let thumbnails = $state<Record<number, string>>({});
  let busy = $state(false);
  let extractStatus = $state<string | null>(null);

  function toggleSelect(index: number, event: MouseEvent) {
    if (event.shiftKey && selected.size > 0) {
      const last = Math.max(...selected);
      const [from, to] = last < index ? [last, index] : [index, last];
      const next = new Set(selected);
      for (let i = from; i <= to; i++) next.add(i);
      selected = next;
    } else if (event.ctrlKey || event.metaKey) {
      const next = new Set(selected);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      selected = next;
    } else {
      selected = new Set([index]);
    }
    docState.navigateToPage(index);
  }

  async function afterMutation() {
    thumbnails = {};
    await docState.refreshPages();
  }

  function onDragStart(index: number) {
    dragIndex = index;
  }

  async function onDrop(targetIndex: number) {
    if (dragIndex === null || docState.id === null || dragIndex === targetIndex) {
      dragIndex = null;
      return;
    }
    const order = docState.pages.map((p) => p.logicalIndex);
    const [moved] = order.splice(dragIndex, 1);
    order.splice(targetIndex, 0, moved);
    dragIndex = null;

    busy = true;
    try {
      await reorderPages(docState.id, order);
      await afterMutation();
      selected = new Set();
    } catch (e) {
      docState.error = String(e);
    } finally {
      busy = false;
    }
  }

  async function rotateSelected(clockwise: boolean) {
    if (docState.id === null || selected.size === 0) return;
    busy = true;
    try {
      for (const index of selected) {
        await rotatePage(docState.id, index, clockwise);
      }
      await afterMutation();
    } catch (e) {
      docState.error = String(e);
    } finally {
      busy = false;
    }
  }

  async function deleteSelected() {
    if (docState.id === null || selected.size === 0) return;
    busy = true;
    try {
      await deletePages(docState.id, [...selected]);
      selected = new Set();
      await afterMutation();
    } catch (e) {
      docState.error = String(e);
    } finally {
      busy = false;
    }
  }

  async function extractSelected() {
    if (docState.id === null || selected.size === 0) return;
    const destPath = await saveDialog({ filters: [PDF_FILTER], defaultPath: "extracted.pdf" });
    if (!destPath) return;
    busy = true;
    extractStatus = null;
    try {
      const ordered = [...selected].sort((a, b) => a - b);
      await extractPages(docState.id, ordered, destPath);
      extractStatus = `Extracted ${ordered.length} page${ordered.length === 1 ? "" : "s"} to ${destPath}`;
    } catch (e) {
      docState.error = String(e);
    } finally {
      busy = false;
    }
  }

  async function loadThumbnail(index: number) {
    if (docState.id === null || thumbnails[index]) return;
    const page = await renderPage(docState.id, index, 140);
    const canvas = document.createElement("canvas");
    canvas.width = page.width;
    canvas.height = page.height;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.putImageData(new ImageData(new Uint8ClampedArray(page.rgba), page.width, page.height), 0, 0);
    thumbnails = { ...thumbnails, [index]: canvas.toDataURL() };
  }

  // Lazy-loads a thumbnail only once its placeholder scrolls near the
  // viewport, so opening a very long document doesn't render every page's
  // bitmap up front.
  function lazyThumbnail(node: HTMLElement, index: number) {
    const root = node.closest(".pages-list");
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            loadThumbnail(index);
            observer.disconnect();
          }
        }
      },
      { root, rootMargin: "200px" },
    );
    observer.observe(node);
    return {
      destroy() {
        observer.disconnect();
      },
    };
  }
</script>

<div class="panel">
  <div class="actions">
    <button onclick={() => rotateSelected(false)} disabled={busy || selected.size === 0} title="Rotate left">
      ⟲
    </button>
    <button onclick={() => rotateSelected(true)} disabled={busy || selected.size === 0} title="Rotate right">
      ⟳
    </button>
    <button onclick={deleteSelected} disabled={busy || selected.size === 0} title="Delete selected">
      Delete
    </button>
  </div>
  <div class="extract">
    <button onclick={extractSelected} disabled={busy || selected.size === 0}>
      Extract selected…
    </button>
  </div>
  {#if extractStatus}
    <p class="extract-status">{extractStatus}</p>
  {/if}

  <div class="pages-list">
    {#each docState.pages as page (page.logicalIndex)}
      <div
        class="thumb"
        class:selected={selected.has(page.logicalIndex)}
        draggable="true"
        role="button"
        tabindex="0"
        onclick={(e) => toggleSelect(page.logicalIndex, e)}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            toggleSelect(page.logicalIndex, e as unknown as MouseEvent);
          }
        }}
        ondragstart={() => onDragStart(page.logicalIndex)}
        ondragover={(e) => e.preventDefault()}
        ondrop={() => onDrop(page.logicalIndex)}
        use:lazyThumbnail={page.logicalIndex}
      >
        <div class="thumb-image" style:transform={`rotate(${page.rotationDegrees}deg)`}>
          {#if thumbnails[page.logicalIndex]}
            <img src={thumbnails[page.logicalIndex]} alt="Page {page.logicalIndex + 1}" />
          {:else}
            <div class="thumb-placeholder"></div>
          {/if}
        </div>
        <span class="thumb-label">{page.logicalIndex + 1}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    width: 200px;
    min-width: 200px;
    height: 100%;
    background: var(--color-panel-bg, #1e1b26);
    border-right: 1px solid var(--color-border, #ccc);
  }

  .actions,
  .extract {
    display: flex;
    gap: var(--space-sm, 0.5rem);
    padding: var(--space-sm, 0.5rem);
    border-bottom: 1px solid var(--color-border, #ccc);
  }

  .extract button {
    flex: 1;
  }

  .extract-status {
    margin: 0;
    padding: 0 var(--space-sm, 0.5rem) var(--space-sm, 0.5rem);
    font-size: 11px;
    opacity: 0.7;
    word-break: break-all;
  }

  .pages-list {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-sm, 0.5rem);
    display: flex;
    flex-direction: column;
    gap: var(--space-sm, 0.5rem);
  }

  .thumb {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 6px;
    border: 2px solid transparent;
    border-radius: 4px;
    cursor: grab;
  }

  .thumb.selected {
    border-color: var(--color-accent, #3b82f6);
    background: rgba(59, 130, 246, 0.15);
  }

  .thumb-image {
    width: 100%;
    display: flex;
    justify-content: center;
  }

  .thumb-image img {
    max-width: 100%;
    box-shadow: 0 0 4px rgba(0, 0, 0, 0.3);
  }

  .thumb-placeholder {
    width: 100px;
    height: 130px;
    background: var(--color-thumb-bg, #f2f1f4);
    opacity: 0.15;
  }

  .thumb-label {
    font-size: 12px;
    opacity: 0.7;
  }
</style>
