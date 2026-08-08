// Zoom moved onto DocumentState (see ../doc-state.svelte.ts) so each open
// tab keeps its own zoom level, the same way each of a browser's tabs does.
// `docState` already proxies to whichever tab is active, so re-exporting it
// under this module's original name keeps every existing `viewerState.zoom`
// / `.zoomIn()` / etc. call site working unchanged.
export { docState as viewerState } from "../doc-state.svelte";
