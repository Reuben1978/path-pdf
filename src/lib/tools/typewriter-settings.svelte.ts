export const FONT_LABELS: Record<string, string> = {
  helvetica: "Helvetica",
  "helvetica-bold": "Helvetica Bold",
  "times-roman": "Times Roman",
  "times-bold": "Times Bold",
  courier: "Courier",
};

export const FONT_SIZES = [9, 10, 12, 14, 18, 24, 32];

/** Shared font/size choice for the typewriter tool -- read by the font panel
 * (in SignatureLibrary.svelte, where the user asked for it) and by each
 * PageSlot when a new text annotation is placed. */
class TypewriterSettings {
  fontName = $state("helvetica");
  fontSize = $state(12);
}

export const typewriterSettings = new TypewriterSettings();
