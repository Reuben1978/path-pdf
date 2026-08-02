import { listSignatures, type SignatureInfo } from "../ipc";

/** Shared state for the signature library, read by both the library panel
 * (list/import/draw/delete) and the viewer (which signature is armed for
 * drag-to-place). */
class SignatureLibraryState {
  signatures = $state<SignatureInfo[]>([]);
  selectedFilename = $state<string | null>(null);

  get selected(): SignatureInfo | null {
    return this.signatures.find((s) => s.filename === this.selectedFilename) ?? null;
  }

  async refresh() {
    this.signatures = await listSignatures();
    if (this.selectedFilename && !this.signatures.some((s) => s.filename === this.selectedFilename)) {
      this.selectedFilename = null;
    }
  }
}

export const signatureLibrary = new SignatureLibraryState();
