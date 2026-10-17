import { create } from "zustand";
import type { TerminalSession } from "../lib/types";

type TerminalState = {
  sessions: TerminalSession[];
  activeId: string | null;
  dockOpen: boolean;
  dockHeight: number;
  upsert: (session: TerminalSession) => void;
  markExit: (id: string, code?: number | null) => void;
  remove: (id: string) => void;
  setActive: (id: string) => void;
  setDockOpen: (open: boolean) => void;
  setDockHeight: (height: number) => void;
};

const HEIGHT_KEY = "bluephoenix.dockHeight";
const DEFAULT_HEIGHT = 300;

function loadHeight() {
  try {
    const raw = Number(window.localStorage.getItem(HEIGHT_KEY));
    return raw >= 140 ? raw : DEFAULT_HEIGHT;
  } catch {
    return DEFAULT_HEIGHT;
  }
}

export const useTerminals = create<TerminalState>((set) => ({
  sessions: [],
  activeId: null,
  dockOpen: false,
  dockHeight: loadHeight(),
  upsert: (session) =>
    set((s) => {
      const exists = s.sessions.some((x) => x.id === session.id);
      return {
        sessions: exists ? s.sessions.map((x) => (x.id === session.id ? session : x)) : [...s.sessions, session],
        activeId: s.activeId ?? session.id,
      };
    }),
  markExit: (id, code) =>
    set((s) => ({
      sessions: s.sessions.map((x) => (x.id === id ? { ...x, running: false, exitCode: code ?? null } : x)),
    })),
  remove: (id) =>
    set((s) => {
      const sessions = s.sessions.filter((x) => x.id !== id);
      const activeId = s.activeId === id ? (sessions[sessions.length - 1]?.id ?? null) : s.activeId;
      return { sessions, activeId, dockOpen: sessions.length === 0 ? false : s.dockOpen };
    }),
  setActive: (activeId) => set({ activeId }),
  setDockOpen: (dockOpen) => set({ dockOpen }),
  setDockHeight: (dockHeight) => {
    try {
      window.localStorage.setItem(HEIGHT_KEY, String(dockHeight));
    } catch {
      /* height is a convenience only */
    }
    set({ dockHeight });
  },
}));

// ---- Output buffers live outside React so xterm instances can mount late and still show history. ----

const MAX_BUFFER = 256 * 1024;
const buffers = new Map<string, string>();
const subscribers = new Map<string, Set<(chunk: string) => void>>();

export function appendOutput(id: string, chunk: string) {
  const next = (buffers.get(id) ?? "") + chunk;
  buffers.set(id, next.length > MAX_BUFFER ? next.slice(next.length - MAX_BUFFER) : next);
  subscribers.get(id)?.forEach((fn) => fn(chunk));
}

export function setBuffer(id: string, text: string) {
  buffers.set(id, text);
}

export function getBuffer(id: string) {
  return buffers.get(id) ?? "";
}

export function forgetBuffer(id: string) {
  buffers.delete(id);
  subscribers.delete(id);
}

export function subscribeOutput(id: string, fn: (chunk: string) => void) {
  let set = subscribers.get(id);
  if (!set) subscribers.set(id, (set = new Set()));
  set.add(fn);
  return () => {
    set?.delete(fn);
  };
}
