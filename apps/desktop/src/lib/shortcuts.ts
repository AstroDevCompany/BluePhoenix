export type ShortcutId =
  | "palette"
  | "projects"
  | "new"
  | "settings"
  | "timer"
  | "editor"
  | "shortcuts"
  | "back"
  | "workspace"
  | "jump";

export type Shortcut = { id: ShortcutId; keys: string; label: string; group: "Navigate" | "Project" | "General" };

/** `mod` is ⌘ on macOS and Ctrl elsewhere. */
export const SHORTCUTS: Shortcut[] = [
  { id: "palette", keys: "mod+k", label: "Command palette", group: "Navigate" },
  { id: "projects", keys: "mod+p", label: "Jump to project", group: "Navigate" },
  { id: "workspace", keys: "mod+1…9", label: "Switch workspace", group: "Navigate" },
  { id: "jump", keys: "alt+1…9", label: "Open project from sidebar", group: "Navigate" },
  { id: "back", keys: "escape", label: "Back to project list", group: "Navigate" },
  { id: "new", keys: "mod+n", label: "New project / item", group: "General" },
  { id: "settings", keys: "mod+,", label: "Open settings", group: "General" },
  { id: "shortcuts", keys: "?", label: "Show keyboard shortcuts", group: "General" },
  { id: "timer", keys: "mod+shift+t", label: "Start / stop timer", group: "Project" },
  { id: "editor", keys: "mod+e", label: "Open project in editor", group: "Project" },
];

export const isMac = typeof navigator !== "undefined" && /mac/i.test(navigator.userAgent);

export function matchesCombo(e: KeyboardEvent, combo: string): boolean {
  const parts = combo.toLowerCase().split("+");
  const key = parts[parts.length - 1];
  const wantMod = parts.includes("mod");
  const wantShift = parts.includes("shift");
  const wantAlt = parts.includes("alt");
  const mod = isMac ? e.metaKey : e.ctrlKey;
  if (key === "?") return e.key === "?" && !mod && !e.altKey;
  if (wantMod !== mod) return false;
  if (wantShift !== e.shiftKey) return false;
  if (wantAlt !== e.altKey) return false;
  if (key === "escape") return e.key === "Escape";
  return e.key.toLowerCase() === key;
}

/** Index (0-8) for ⌘/Ctrl+digit or Alt+digit combos; uses `code` so Option on macOS still works. */
export function digitIndex(e: KeyboardEvent, modifier: "mod" | "alt"): number | null {
  const mod = isMac ? e.metaKey : e.ctrlKey;
  if (modifier === "mod" ? !mod || e.altKey : !e.altKey || mod) return null;
  if (e.shiftKey) return null;
  const m = /^Digit([1-9])$/.exec(e.code);
  return m ? Number(m[1]) - 1 : null;
}

const SYMBOLS: Record<string, string> = isMac
  ? { mod: "⌘", shift: "⇧", alt: "⌥", escape: "Esc" }
  : { mod: "Ctrl", shift: "Shift", alt: "Alt", escape: "Esc" };

export function formatCombo(combo: string): string {
  return combo
    .split("+")
    .map((p) => SYMBOLS[p] ?? (p.length === 1 ? p.toUpperCase() : p))
    .join(isMac ? "" : "+");
}

export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}
