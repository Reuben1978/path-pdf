import { openDocument as ipcOpenDocument, listPages, getPageSizes, type PageSummary, type PageSize } from "./ipc";

/**
 * Shared reactive state for the currently open document. Both the viewer and
 * the pages panel read/write this instead of passing everything through
 * props, since they need to stay in sync (e.g. deleting a page in the panel
 * changes the viewer's page count and possibly its current page).
 */
class DocumentState {
  id = $state<number | null>(null);
  path = $state<string | null>(null);
  pageCount = $state(0);
  currentPage = $state(0);
  pages = $state<PageSummary[]>([]);
  pageSizes = $state<PageSize[]>([]);
  error = $state<string | null>(null);

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
}

export const docState = new DocumentState();
