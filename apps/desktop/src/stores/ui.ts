import { create } from "zustand";

type Toast = { id: number; message: string; tone?: "ok" | "error" };

type UiState = {
  paletteOpen: boolean;
  createOpen: boolean;
  aiPanelOpen: boolean;
  toast: Toast | null;
  confirm: { title: string; body: string; onConfirm: () => void; danger?: boolean } | null;
  setPalette: (open: boolean) => void;
  setAiPanel: (open: boolean) => void;
  openCreate: () => void;
  closeCreate: () => void;
  showToast: (message: string, tone?: "ok" | "error") => void;
  askConfirm: (title: string, body: string, onConfirm: () => void, danger?: boolean) => void;
  closeConfirm: () => void;
};

let toastId = 1;

export const useUi = create<UiState>((set) => ({
  paletteOpen: false,
  createOpen: false,
  aiPanelOpen: false,
  toast: null,
  confirm: null,
  setPalette: (paletteOpen) => set({ paletteOpen }),
  setAiPanel: (aiPanelOpen) => set({ aiPanelOpen }),
  openCreate: () => set({ createOpen: true }),
  closeCreate: () => set({ createOpen: false }),
  showToast: (message, tone = "ok") => {
    const id = toastId++;
    set({ toast: { id, message, tone } });
    window.setTimeout(() => {
      set((s) => (s.toast?.id === id ? { toast: null } : s));
    }, 4200);
  },
  askConfirm: (title, body, onConfirm, danger = false) => set({ confirm: { title, body, onConfirm, danger } }),
  closeConfirm: () => set({ confirm: null }),
}));
