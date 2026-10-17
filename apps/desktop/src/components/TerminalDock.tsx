import { useEffect, useRef } from "react";
import { ChevronDown, ChevronUp, ExternalLink, Plus, RotateCcw, Square, X } from "lucide-react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import "@xterm/xterm/css/xterm.css";
import type { TerminalSession } from "../lib/types";
import { api } from "../lib/ipc";
import { getBuffer, subscribeOutput, useTerminals } from "../stores/terminals";
import { closeSession, killSession, openEmbeddedTerminal, rerunSession } from "../lib/terminalActions";
import type { ProjectCard } from "../lib/types";
import { formatCombo } from "../lib/shortcuts";

const MIN_HEIGHT = 140;

function cssVar(name: string, fallback: string) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function TerminalView({ session, active }: { session: TerminalSession; active: boolean }) {
  const host = useRef<HTMLDivElement>(null);
  const term = useRef<Terminal | null>(null);
  const fit = useRef<FitAddon | null>(null);
  const activeRef = useRef(active);
  activeRef.current = active;

  useEffect(() => {
    const el = host.current;
    if (!el) return;
    const t = new Terminal({
      fontFamily: cssVar("--font-mono", "Menlo, monospace"),
      fontSize: 12.5,
      lineHeight: 1.25,
      cursorBlink: true,
      scrollback: 5000,
      allowTransparency: true,
      theme: {
        background: "rgba(0,0,0,0)",
        foreground: "#e8f4f2",
        cursor: "#3dd6c6",
        selectionBackground: "rgba(61, 214, 198, 0.28)",
      },
    });
    const f = new FitAddon();
    t.loadAddon(f);
    t.loadAddon(new WebLinksAddon((_e, uri) => void api.openUrl(uri)));
    t.open(el);
    term.current = t;
    fit.current = f;

    const sync = () => {
      if (!activeRef.current || el.clientWidth === 0) return;
      try {
        f.fit();
        void api.termResize(session.id, t.cols, t.rows).catch(() => undefined);
      } catch {
        /* container not measurable yet */
      }
    };
    sync();
    t.write(getBuffer(session.id));
    const unsubscribe = subscribeOutput(session.id, (chunk) => t.write(chunk));
    const input = t.onData((data) => void api.termWrite(session.id, data).catch(() => undefined));
    const observer = new ResizeObserver(sync);
    observer.observe(el);
    return () => {
      observer.disconnect();
      input.dispose();
      unsubscribe();
      t.dispose();
      term.current = null;
    };
  }, [session.id]);

  useEffect(() => {
    if (!active) return;
    const id = window.requestAnimationFrame(() => {
      try {
        fit.current?.fit();
        const t = term.current;
        if (t) {
          void api.termResize(session.id, t.cols, t.rows).catch(() => undefined);
          t.focus();
        }
      } catch {
        /* ignore */
      }
    });
    return () => window.cancelAnimationFrame(id);
  }, [active, session.id]);

  return <div ref={host} className="term-view" hidden={!active} />;
}

function statusOf(s: TerminalSession) {
  if (s.running) return "running";
  return s.exitCode === 0 ? "ok" : "failed";
}

export function TerminalDock({ currentProject }: { currentProject?: ProjectCard }) {
  const sessions = useTerminals((s) => s.sessions);
  const activeId = useTerminals((s) => s.activeId);
  const open = useTerminals((s) => s.dockOpen);
  const height = useTerminals((s) => s.dockHeight);
  const setActive = useTerminals((s) => s.setActive);
  const setOpen = useTerminals((s) => s.setDockOpen);
  const setHeight = useTerminals((s) => s.setDockHeight);
  const drag = useRef<{ startY: number; startHeight: number } | null>(null);

  if (sessions.length === 0) return null;
  const active = sessions.find((s) => s.id === activeId) ?? sessions[0];
  const runningCount = sessions.filter((s) => s.running).length;

  const onResizeStart = (e: React.PointerEvent<HTMLDivElement>) => {
    drag.current = { startY: e.clientY, startHeight: height };
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onResizeMove = (e: React.PointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    const max = Math.round(window.innerHeight * 0.7);
    setHeight(Math.min(max, Math.max(MIN_HEIGHT, drag.current.startHeight + (drag.current.startY - e.clientY))));
  };

  return (
    <section className={`term-dock ${open ? "open" : ""}`} style={open ? { height } : undefined} aria-label="Terminal">
      {open ? (
        <div
          className="term-resize"
          role="separator"
          aria-orientation="horizontal"
          onPointerDown={onResizeStart}
          onPointerMove={onResizeMove}
          onPointerUp={() => (drag.current = null)}
        />
      ) : null}
      <header className="term-head">
        <div className="term-tabs" role="tablist">
          {sessions.map((s) => (
            <div key={s.id} className={`term-tab ${s.id === active.id ? "active" : ""}`}>
              <button
                type="button"
                role="tab"
                aria-selected={s.id === active.id}
                className="term-tab-main"
                onClick={() => {
                  setActive(s.id);
                  setOpen(true);
                }}
              >
                <span className="term-status" data-state={statusOf(s)} aria-hidden />
                <span className="term-tab-title">{s.title}</span>
                {!s.running && s.exitCode != null && s.exitCode !== 0 ? <span className="term-exit">exit {s.exitCode}</span> : null}
              </button>
              <button type="button" className="term-tab-close" aria-label={`Close ${s.title}`} onClick={() => void closeSession(s.id)}>
                <X size={12} />
              </button>
            </div>
          ))}
          {currentProject ? (
            <button
              type="button"
              className="term-new"
              aria-label={`New terminal in ${currentProject.name}`}
              title={`New terminal in ${currentProject.name}`}
              onClick={() => void openEmbeddedTerminal(currentProject)}
            >
              <Plus size={14} />
            </button>
          ) : null}
        </div>
        <div className="term-actions">
          {active.running ? (
            <button type="button" className="btn ghost term-btn" onClick={() => void killSession(active.id)} title="Stop (SIGTERM)">
              <Square size={12} /> Stop
            </button>
          ) : active.command || active.commandId ? (
            <button type="button" className="btn ghost term-btn" onClick={() => void rerunSession(active)} title="Run again">
              <RotateCcw size={12} /> Rerun
            </button>
          ) : null}
          <button
            type="button"
            className="btn ghost term-btn"
            title="Open this folder in your system terminal"
            onClick={() => void api.openTerminal(active.cwd, { command: active.command ?? undefined, confirmed: true })}
          >
            <ExternalLink size={12} /> System terminal
          </button>
          <span className="muted term-meta">{runningCount > 0 ? `${runningCount} running` : "idle"} · {formatCombo("mod+j")}</span>
          <button type="button" className="btn ghost icon" aria-label={open ? "Collapse terminal" : "Expand terminal"} onClick={() => setOpen(!open)}>
            {open ? <ChevronDown size={14} /> : <ChevronUp size={14} />}
          </button>
        </div>
      </header>
      <div className="term-body" hidden={!open}>
        {sessions.map((s) => (
          <TerminalView key={s.id} session={s} active={open && s.id === active.id} />
        ))}
      </div>
    </section>
  );
}
