<script lang="ts">
  import { onMount } from "svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import {
    listRecentDocuments,
    listRecentPlaces,
    setRecentDocumentPinned,
    setRecentPlacePinned,
    type RecentEntry,
  } from "../ipc";
  import { tabsState } from "../doc-state.svelte";
  import Button from "../components/Button.svelte";

  const PDF_FILTER = { name: "PDF", extensions: ["pdf"] };

  let documents = $state<RecentEntry[]>([]);
  let places = $state<RecentEntry[]>([]);
  let error = $state<string | null>(null);

  onMount(refresh);

  async function refresh() {
    try {
      documents = await listRecentDocuments();
      places = await listRecentPlaces();
    } catch (e) {
      error = String(e);
    }
  }

  function basename(path: string): string {
    return path.split(/[/\\]/).pop() || path;
  }

  async function openFile() {
    const path = await openDialog({ multiple: false, filters: [PDF_FILTER] });
    if (path) await tabsState.openNewTab(path);
  }

  async function openRecentDocument(path: string) {
    await tabsState.openNewTab(path);
  }

  async function openRecentPlace(directory: string) {
    const picked = await openDialog({ multiple: false, defaultPath: directory, filters: [PDF_FILTER] });
    if (picked) await tabsState.openNewTab(picked);
  }

  async function toggleDocumentPin(entry: RecentEntry) {
    await setRecentDocumentPinned(entry.path, !entry.pinned);
    await refresh();
  }

  async function togglePlacePin(entry: RecentEntry) {
    await setRecentPlacePinned(entry.path, !entry.pinned);
    await refresh();
  }
</script>

<div class="start">
  <div class="hero">
    <h1>Path PDF</h1>
    <Button onclick={openFile}>Open a PDF…</Button>
    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>

  <div class="columns">
    <section>
      <h2>Recent Documents</h2>
      {#if documents.length === 0}
        <p class="empty">No documents opened yet.</p>
      {:else}
        <ul>
          {#each documents as entry (entry.path)}
            <li>
              <button class="entry" onclick={() => openRecentDocument(entry.path)}>
                <span class="name">{basename(entry.path)}</span>
                <span class="path">{entry.path}</span>
              </button>
              <button
                class="pin"
                class:pinned={entry.pinned}
                onclick={() => toggleDocumentPin(entry)}
                title={entry.pinned ? "Unpin" : "Pin"}
              >
                📌
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section>
      <h2>Recent Places</h2>
      {#if places.length === 0}
        <p class="empty">No places yet.</p>
      {:else}
        <ul>
          {#each places as entry (entry.path)}
            <li>
              <button class="entry" onclick={() => openRecentPlace(entry.path)}>
                <span class="name">{basename(entry.path)}</span>
                <span class="path">{entry.path}</span>
              </button>
              <button
                class="pin"
                class:pinned={entry.pinned}
                onclick={() => togglePlacePin(entry)}
                title={entry.pinned ? "Unpin" : "Pin"}
              >
                📌
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
</div>

<style>
  .start {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: var(--space-md, 1rem) var(--space-md, 1rem) 2rem;
    overflow-y: auto;
  }

  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-sm, 0.5rem);
    padding: 2rem 0 1.5rem;
  }

  .hero h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 600;
  }

  .error {
    color: var(--color-error, #f87171);
    font-size: 13px;
    margin: 0;
  }

  .columns {
    display: flex;
    gap: 2rem;
    width: 100%;
    max-width: 900px;
    align-items: flex-start;
  }

  section {
    flex: 1;
    min-width: 0;
  }

  h2 {
    font-size: 14px;
    margin: 0 0 var(--space-sm, 0.5rem);
    opacity: 0.8;
  }

  .empty {
    font-size: 13px;
    opacity: 0.6;
    margin: 0;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  li {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .entry {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    padding: 6px 8px;
    border: 1px solid transparent;
    background: transparent;
    text-align: left;
    border-radius: 4px;
  }

  .entry:hover {
    background: var(--color-panel-bg, #1e1b26);
    border-color: var(--color-border, #322e3d);
  }

  .name {
    font-size: 13px;
    color: var(--color-text, #e8e6ed);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .path {
    font-size: 11px;
    opacity: 0.55;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }

  .pin {
    border: none;
    background: none;
    padding: 4px;
    font-size: 13px;
    opacity: 0.3;
    filter: grayscale(1);
  }

  .pin:hover {
    opacity: 0.7;
  }

  .pin.pinned {
    opacity: 1;
    filter: none;
  }
</style>
