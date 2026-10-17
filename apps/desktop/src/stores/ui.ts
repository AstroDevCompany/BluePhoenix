import { create } from "zustand";

export type ToastTone = "ok" | "error";
export type Toast = {
  id: number;
  message: string;
  tone: ToastTone;
  action?: { label: string; run: () => void };
};
export type ToastOptions = { action?: Toast["action"]; durationMs?: number };

export type PaletteMode = "all" | "projects";

type UiState = {
  paletteOpen: boolean;
  paletteMode: PaletteMode;
  shortcutsOpen: boolean;
  createOpen: boolean;
  aiPanelOpen: boolean;
  toasts: Toast[];
  recentProjectIds: string[];
  confirm: { title: string; body: string; onConfirm: () => void; danger?: boolean } | null;
  setPalette: (open: boolean) => void;
  openPalette: (mode?: PaletteMode) => void;
  setShortcuts: (open: boolean) => void;
  setAiPanel: (open: boolean) => void;
  openCreate: () => void;
  closeCreate: () => void;
  showToast: (message: string, tone?: ToastTone, options?: ToastOptions) => void;
  dismissToast: (id: number) => void;
  touchProject: (id: string) => void;
  askConfirm: (title: string, body: string, onConfirm: () => void, danger?: boolean) => void;
  closeConfirm: () => void;
};

const RECENTS_KEY = "bluephoenix.recentProjects";
const MAX_RECENTS = 12;
const MAX_TOASTS = 4;

function loadRecents(): string[] {
  try {
    const raw = window.localStorage.getItem(RECENTS_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    return Array.isArray(parsed) ? parsed.filter((v): v is string => typeof v === "string") : [];
  } catch {
    return [];
  }
}

function saveRecents(ids: string[]) {
  try {
    window.localStorage.setItem(RECENTS_KEY, JSON.stringify(ids));
  } catch {
    /* storage can be unavailable; recents are a convenience only */
  }
}

let toastId = 1;

export const useUi = create<UiState>((set) => ({
  paletteOpen: false,
  paletteMode: "all",
  shortcutsOpen: false,
  createOpen: false,
  aiPanelOpen: false,
  toasts: [],
  recentProjectIds: loadRecents(),
  confirm: null,
  setPalette: (paletteOpen) => set({ paletteOpen }),
  openPalette: (paletteMode = "all") => set({ paletteOpen: true, paletteMode }),
  setShortcuts: (shortcutsOpen) => set({ shortcutsOpen }),
  setAiPanel: (aiPanelOpen) => set({ aiPanelOpen }),
  openCreate: () => set({ createOpen: true }),
  closeCreate: () => set({ createOpen: false }),
  showToast: (message, tone = "ok", options = {}) => {
    const id = toastId++;
    const duration = options.durationMs ?? (options.action ? 8000 : tone === "error" ? 7000 : 4200);
    set((s) => ({
      toasts: [...s.toasts, { id, message, tone, action: options.action }].slice(-MAX_TOASTS),
    }));
    window.setTimeout(() => {
      set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) }));
    }, duration);
  },
  dismissToast: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
  touchProject: (id) =>
    set((s) => {
      if (s.recentProjectIds[0] === id) return s;
      const next = [id, ...s.recentProjectIds.filter((r) => r !== id)].slice(0, MAX_RECENTS);
      saveRecents(next);
      return { recentProjectIds: next };
    }),
  askConfirm: (title, body, onConfirm, danger = false) => set({ confirm: { title, body, onConfirm, danger } }),
  closeConfirm: () => set({ confirm: null }),
}));
