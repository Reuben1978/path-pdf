import {
  openDocument as ipcOpenDocument,
  closeDocument as ipcCloseDocument,
  listPages,
  getPageSizes,
  type PageSummary,
  type PageSize,
} from "./ipc";

const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;
const ZOOM_STEP = 0.1;

function roundZoom(value: number): number {
  return Math.round(value * 100) / 100;
}

/**
 * One open document's state -- one tab's worth. Both the viewer and the
 * pages panel read/write the *active* tab's instance (via the `docState`
 * proxy below) instead of passing everything through props, since they need
 * to stay in sync (e.g. deleting a page in the panel changes the viewer's
 * page count and possibly its current page).
 */
class DocumentState {
  id = $state<number | null>(null);
  path = $state<string | null>(null);
  pageCount = $state(0);
  currentPage = $state(0);
  pages = $state<PageSummary[]>([]);
  pageSizes = $state<PageSize[]>([]);
  error = $state<string | null>(null);

  /** Zoom is per-document, like a browser tab's zoom level -- switching
   * tabs shouldn't change how zoomed-in a different document is. */
  zoom = $state(1);

  /** Bumped per page index when that page's content changes (annotation or
   * signature added/removed), so the continuous-scroll viewer knows to
   * re-render just that page's already-visible canvas. */
  pageDirtyTick = $state<Record<number, number>>({});

  /**
   * An explicit "scroll the viewer to this page" request, distinct from
   * `currentPage` itself: `currentPage` is also updated continuously as a
   * side effect of scrolling (by the IntersectionObserver in PageSlot), so
   * the viewer can't just watch `currentPage` to decide when to scroll --
   * that would create a feedback loop. Callers that want to *navigate*
   * (pages panel thumbnail clicks, Prev/Next) call `navigateToPage`, which
   * bumps this with a fresh object identity every time so the viewer's
   * effect reliably re-fires even when re-navigating to the same page.
   */
  scrollRequest = $state<{ page: number; nonce: number } | null>(null);
  #scrollNonce = 0;

  navigateToPage(index: number) {
    this.currentPage = index;
    this.scrollRequest = { page: index, nonce: ++this.#scrollNonce };
  }

  async open(path: string) {
    this.error = null;
    try {
      const info = await ipcOpenDocument(path);
      this.id = info.id;
      this.path = info.path;
      this.pageCount = info.pageCount;
      this.currentPage = 0;
      this.pageDirtyTick = {};
      await this.refreshPages();
      this.pageSizes = await getPageSizes(this.id);
    } catch (e) {
      this.error = String(e);
    }
  }

  async refreshPages() {
    if (this.id === null) return;
    try {
      this.pages = await listPages(this.id);
      this.pageCount = this.pages.length;
      if (this.currentPage >= this.pageCount) {
        this.currentPage = Math.max(0, this.pageCount - 1);
      }
      this.pageSizes = await getPageSizes(this.id);
    } catch (e) {
      this.error = String(e);
    }
  }

  touchPage(pageIndex: number) {
    this.pageDirtyTick = { ...this.pageDirtyTick, [pageIndex]: (this.pageDirtyTick[pageIndex] ?? 0) + 1 };
  }

  zoomIn() {
    this.zoom = Math.min(MAX_ZOOM, roundZoom(this.zoom + ZOOM_STEP));
  }

  zoomOut() {
    this.zoom = Math.max(MIN_ZOOM, roundZoom(this.zoom - ZOOM_STEP));
  }

  zoomBy(delta: number) {
    this.zoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, roundZoom(this.zoom + delta)));
  }

  reset() {
    this.zoom = 1;
  }
}

/** Never opened, `id` stays null forever -- what `docState` proxies to when
 * there's no active tab, so `docState.id === null` keeps meaning "nothing
 * open" exactly as it did before tabs existed. */
const EMPTY_DOCUMENT = new DocumentState();

/**
 * The open tabs, in display order, plus which one is active. Each tab wraps
 * its own `DocumentState` -- opening a second document no longer replaces
 * the first, since the Rust side has always kept every opened document
 * alive in its own registry keyed by id (see `AppState` in
 * `src-tauri/src/state.rs`); this class is what was missing on the frontend
 * to actually let more than one be open (and visible) at once.
 */
class TabsState {
  tabs = $state<DocumentState[]>([]);
  activeId = $state<number | null>(null);

  get active(): DocumentState | null {
    return this.tabs.find((t) => t.id === this.activeId) ?? null;
  }

  /** Opens `path` in a brand-new tab and makes it active. */
  async openNewTab(path: string): Promise<DocumentState> {
    const doc = new DocumentState();
    await doc.open(path);
    if (doc.id !== null) {
      this.tabs.push(doc);
      this.activeId = doc.id;
    } else {
      // Open failed -- `doc` never becomes a tab, so its `.error` would
      // otherwise vanish with it. Surface it on whatever's currently shown
      // instead (the active tab if there is one, else the empty-state
      // placeholder that backs the StartScreen) rather than swallowing it.
      (this.active ?? EMPTY_DOCUMENT).error = doc.error;
    }
    return doc;
  }

  switchTo(id: number) {
    this.activeId = id;
  }

  async closeTab(id: number) {
    const index = this.tabs.findIndex((t) => t.id === id);
    if (index === -1) return;
    this.tabs.splice(index, 1);
    if (this.activeId === id) {
      // Prefer the tab that slid into this one's spot (the next tab over);
      // fall back to the previous one if this was the last tab.
      this.activeId = this.tabs[index]?.id ?? this.tabs[index - 1]?.id ?? null;
    }
    await ipcCloseDocument(id);
  }
}

export const tabsState = new TabsState();

/**
 * Proxies every read/write/method-call through to whichever `DocumentState`
 * is currently active (or `EMPTY_DOCUMENT` when no tab is), so the existing
 * single-document call sites throughout the viewer and pages panel keep
 * working unchanged -- they don't need to know tabs exist at all.
 */
function activeDocumentProxy(): DocumentState {
  return new Proxy({} as DocumentState, {
    // `$state` fields compile to private-backed accessors on the class, so
    // `this` inside them must be the real `doc` instance -- passing the
    // trap's own `receiver` (the Proxy) through as Reflect's third argument
    // runs those accessors with `this` bound to the Proxy instead, which
    // throws ("invalid private field") since the Proxy was never
    // constructed with that private field at all. Passing `doc` itself as
    // the receiver fixes that.
    get(_target, prop) {
      const doc = tabsState.active ?? EMPTY_DOCUMENT;
      const value = Reflect.get(doc, prop, doc);
      return typeof value === "function" ? value.bind(doc) : value;
    },
    set(_target, prop, value) {
      const doc = tabsState.active ?? EMPTY_DOCUMENT;
      return Reflect.set(doc, prop, value, doc);
    },
  });
}

export const docState = activeDocumentProxy();
