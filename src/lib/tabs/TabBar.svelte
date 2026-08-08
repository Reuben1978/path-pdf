<script lang="ts">
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { tabsState } from "../doc-state.svelte";

  const PDF_FILTER = { name: "PDF", extensions: ["pdf"] };

  function basename(path: string | null): string {
    if (!path) return "Untitled";
    return path.split(/[/\\]/).pop() || path;
  }

  async function openNewTab() {
    const path = await openDialog({ multiple: false, filters: [PDF_FILTER] });
    if (path) await tabsState.openNewTab(path);
  }

  function closeTab(event: MouseEvent, id: number) {
    event.stopPropagation();
    tabsState.closeTab(id);
  }
</script>

<div class="tab-bar" role="tablist">
  {#each tabsState.tabs as tab (tab.id)}
    <div
      role="tab"
      tabindex="0"
      aria-selected={tab.id === tabsState.activeId}
      class="tab"
      class:active={tab.id === tabsState.activeId}
      title={tab.path ?? undefined}
      onclick={() => tab.id !== null && tabsState.switchTo(tab.id)}
      onkeydown={(e) => {
        if ((e.key === "Enter" || e.key === " ") && tab.id !== null) tabsState.switchTo(tab.id);
      }}
    >
      <span class="title">{basename(tab.path)}</span>
      <button
        type="button"
        class="close"
        title="Close tab"
        onclick={(e) => tab.id !== null && closeTab(e, tab.id)}
      >
        ×
      </button>
    </div>
  {/each}
  <button type="button" class="new-tab" onclick={openNewTab} title="Open a PDF…">+</button>
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: stretch;
    gap: 2px;
    padding: 4px 4px 0;
    background: var(--color-bg);
    border-bottom: 1px solid var(--color-border, #322e3d);
    overflow-x: auto;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 0.5em;
    max-width: 220px;
    min-width: 120px;
    padding: 6px 8px 6px 12px;
    border: 1px solid var(--color-border, #322e3d);
    border-bottom: none;
    border-radius: 8px 8px 0 0;
    background: var(--color-panel-bg, #1e1b26);
    color: var(--color-text, #e8e6ed);
    font: inherit;
    font-size: 13px;
    cursor: pointer;
    opacity: 0.7;
  }

  .tab:hover {
    opacity: 0.9;
  }

  .tab.active {
    background: var(--color-bg);
    opacity: 1;
  }

  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  .close {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.4em;
    height: 1.4em;
    border: none;
    background: none;
    border-radius: 50%;
    color: inherit;
    font: inherit;
    line-height: 1;
    opacity: 0.6;
    cursor: pointer;
    padding: 0;
  }

  .tab:focus-visible {
    outline: 2px solid var(--color-accent-purple-bright, #a78bfa);
    outline-offset: -2px;
  }

  .close:hover {
    opacity: 1;
    background: var(--color-border, #322e3d);
  }

  .new-tab {
    flex: none;
    width: 2em;
    align-self: center;
    margin-left: 2px;
    border: none;
    background: none;
    color: var(--color-text, #e8e6ed);
    font-size: 16px;
    line-height: 1;
    opacity: 0.6;
    cursor: pointer;
    border-radius: 4px;
    padding: 4px 0;
  }

  .new-tab:hover {
    opacity: 1;
    background: var(--color-panel-bg, #1e1b26);
  }
</style>
