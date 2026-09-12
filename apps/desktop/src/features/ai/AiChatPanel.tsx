import { useEffect, useRef, useState } from "react";
import { ArrowUp, MessageSquarePlus, Trash2 } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { api, formatError } from "../../lib/ipc";
import { useUi } from "../../stores/ui";
import type { AiMessage } from "../../lib/types";

const COMPOSER_LINE = 21;
const COMPOSER_PAD = 16;
const COMPOSER_MAX_LINES = 4;

function fitComposer(el: HTMLTextAreaElement | null) {
  if (!el) return;
  el.style.height = "0px";
  const next = Math.min(COMPOSER_LINE * COMPOSER_MAX_LINES + COMPOSER_PAD, Math.max(COMPOSER_LINE + COMPOSER_PAD, el.scrollHeight));
  el.style.height = `${next}px`;
}

export function AiChatPanel({ projectId }: { projectId?: string }) {
  const open = useUi((s) => s.aiPanelOpen);
  const toast = useUi((s) => s.showToast);
  const [conversationId, setConversationId] = useState<string | null>(null);
  const [messages, setMessages] = useState<AiMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [streaming, setStreaming] = useState("");
  const [busy, setBusy] = useState(false);
  const [fallbackNote, setFallbackNote] = useState<string | null>(null);
  const live = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLTextAreaElement>(null);
  const end = useRef<HTMLDivElement>(null);

  const load = async (id?: string | null) => {
    if (!id) {
      const cid = await api.aiNewConversation(projectId);
      setConversationId(cid);
      setMessages([]);
      return;
    }
    setConversationId(id);
    setMessages(await api.aiListMessages(id));
  };

  useEffect(() => {
    if (!open) return;
    void api.aiListConversations(projectId).then((list) => {
      const existing = list[0]?.id;
      void load(existing);
    }).catch((e) => toast(formatError(e), "error"));
  }, [open, projectId]);

  useEffect(() => {
    if (open) input.current?.focus();
  }, [open, conversationId]);

  useEffect(() => {
    fitComposer(input.current);
  }, [draft, open]);

  useEffect(() => {
    end.current?.scrollIntoView({ block: "end" });
  }, [messages, streaming]);

  useEffect(() => {
    let unlistenDelta: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;
    void listen<{ conversationId: string; delta: string }>("ai-chat-delta", (ev) => {
      if (ev.payload.conversationId !== conversationId) return;
      setStreaming((s) => s + ev.payload.delta);
    }).then((fn) => { unlistenDelta = fn; });
    void listen<{ conversationId: string; fallbackUsed: boolean; model: string }>("ai-chat-done", (ev) => {
      if (ev.payload.conversationId !== conversationId) return;
      setFallbackNote(ev.payload.fallbackUsed ? `Answered by fallback model ${ev.payload.model}` : null);
    }).then((fn) => { unlistenDone = fn; });
    return () => {
      unlistenDelta?.();
      unlistenDone?.();
    };
  }, [conversationId]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && open) useUi.getState().setAiPanel(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  const send = async () => {
    const text = draft.trim();
    if (!text || busy) return;
    setDraft("");
    setBusy(true);
    setStreaming("");
    setFallbackNote(null);
    try {
      const msg = await api.aiChatStream({ conversationId, projectId, message: text });
      setConversationId(msg.conversationId);
      setMessages(await api.aiListMessages(msg.conversationId));
      setStreaming("");
    } catch (e) {
      toast(formatError(e), "error");
    } finally {
      setBusy(false);
    }
  };

  return (
    <aside className="ai-rail" aria-label="AI chat" aria-hidden={!open} {...(!open ? { inert: true } : {})}>
      <div className="ai-rail-inner">
        <div className="ai-rail-head">
          <button
            className="btn icon ghost"
            type="button"
            aria-label="New chat"
            title="New chat"
            onClick={() => void load(null)}
          >
            <MessageSquarePlus size={16} />
          </button>
          <strong className="ai-rail-title">Chat</strong>
          <button
            className="btn icon ghost"
            type="button"
            aria-label="Clear chat"
            title="Clear chat"
            disabled={!conversationId}
            onClick={async () => {
              if (!conversationId) return;
              await api.aiClearConversation(conversationId);
              setMessages([]);
              setStreaming("");
            }}
          >
            <Trash2 size={16} />
          </button>
        </div>
        <div className="ai-rail-body" role="log" aria-live="polite">
          {messages.map((m) => (
            <div key={m.id} className={`ai-bubble ${m.role}`}>
              {m.content}
              {m.role === "assistant" && m.fallbackUsed ? <div className="muted" style={{ marginTop: 6, fontSize: 12 }}>Served by fallback {m.model}</div> : null}
            </div>
          ))}
          {busy && streaming ? <div className="ai-bubble assistant">{streaming}</div> : null}
          {busy && !streaming ? <div className="ai-bubble assistant muted">Thinking…</div> : null}
          {fallbackNote ? <p className="muted" style={{ fontSize: 12 }}>{fallbackNote}</p> : null}
          <div ref={end} />
        </div>
        <div ref={live} className="sr-only" aria-live="polite">{busy ? "Generating a reply" : ""}</div>
        <form className="ai-rail-composer" onSubmit={(e) => { e.preventDefault(); void send(); }}>
          <div className="ai-composer-box">
            <textarea
              ref={input}
              className="ai-composer-input"
              rows={1}
              value={draft}
              placeholder={projectId ? "Ask about this project…" : "Ask BluePhoenix…"}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  void send();
                }
              }}
            />
            <button
              className="btn icon primary ai-composer-send"
              type="submit"
              aria-label="Send"
              title="Send"
              disabled={busy || !draft.trim()}
            >
              <ArrowUp size={16} />
            </button>
          </div>
        </form>
      </div>
    </aside>
  );
}
