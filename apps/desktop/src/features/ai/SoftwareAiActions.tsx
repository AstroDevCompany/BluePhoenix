import { useState } from "react";
import { api, formatError } from "../../lib/ipc";
import { useUi } from "../../stores/ui";

type Proposal = { title: string; description: string; priority: string; reasoning: string };

export function SoftwareAiActions({
  projectId,
  changelog,
  onDone,
}: {
  projectId: string;
  changelog?: string;
  onDone: () => void;
}) {
  const toast = useUi((s) => s.showToast);
  const confirm = useUi((s) => s.askConfirm);
  const [busy, setBusy] = useState<string | null>(null);
  const [prompt, setPrompt] = useState<{ prompt: string; known: string[]; assumptions: string[] } | null>(null);
  const [commit, setCommit] = useState<string | null>(null);
  const [summary, setSummary] = useState<string | null>(null);
  const [draft, setDraft] = useState<{ title: string; body: string } | null>(null);
  const [todos, setTodos] = useState<Proposal[]>([]);
  const [picked, setPicked] = useState<Record<number, boolean>>({});

  const run = async (label: string, fn: () => Promise<void>) => {
    setBusy(label);
    try {
      await fn();
    } catch (e) {
      toast(formatError(e), "error");
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="glass-panel ai-actions-card">
      <div className="row">
        <button className="btn" type="button" disabled={!!busy} onClick={() => void run("todos", async () => {
          const res = await api.aiPrioritizeTodos(projectId) as { result: { order: { id: string; reason: string }[]; summary: string } };
          confirm("Apply suggested TODO order?", res.result.summary, () => {
            void api.aiApplyTodoPriorities(res.result.order.map((o) => o.id)).then(onDone);
          });
        })}>Prioritize TODOs</button>
        <button className="btn" type="button" disabled={!!busy} onClick={() => void run("prompt", async () => {
          const res = await api.aiGeneratePrompt(projectId) as { result: { prompt: string; known: string[]; assumptions: string[] } };
          setPrompt(res.result);
        })}>Generate AI Prompt</button>
        <button className="btn" type="button" disabled={!!busy} onClick={() => void run("sum", async () => {
          setSummary(await api.aiSummarizeChangelog(projectId));
        })}>Summarize changelog</button>
        <button className="btn" type="button" disabled={!!busy} onClick={() => void run("gen", async () => {
          const res = await api.aiGenerateChangelog(projectId) as { result: { title: string; body: string } };
          setDraft(res.result);
        })}>Generate changelog</button>
        <button className="btn" type="button" disabled={!!busy} onClick={() => void run("from", async () => {
          const body = changelog || draft?.body || "";
          if (!body.trim()) {
            toast("Add changelog text first", "error");
            return;
          }
          const res = await api.aiTodosFromChangelog(projectId, body) as { todos: Proposal[] };
          setTodos(res.todos);
          setPicked(Object.fromEntries(res.todos.map((_, i) => [i, true])));
        })}>TODOs from changelog</button>
        <button className="btn" type="button" disabled={!!busy} onClick={() => void run("commit", async () => {
          const res = await api.aiSuggestCommit(projectId) as { result: { message: string; rationale: string } };
          setCommit(res.result.message);
          toast(res.result.rationale);
        })}>Suggest commit message</button>
      </div>
      {busy ? <p className="muted" aria-live="polite">{busy}…</p> : null}
      {summary ? <p className="glass-panel" style={{ padding: 12, marginTop: 12 }}>{summary}</p> : null}
      {prompt ? (
        <div className="glass-panel" style={{ padding: 12, marginTop: 12 }}>
          <p className="muted">Known</p>
          <ul>{prompt.known.map((k) => <li key={k}>{k}</li>)}</ul>
          <p className="muted">Assumptions</p>
          <ul>{prompt.assumptions.map((k) => <li key={k}>{k}</li>)}</ul>
          <textarea className="textarea" readOnly value={prompt.prompt} />
          <button className="btn" type="button" onClick={() => void navigator.clipboard.writeText(prompt.prompt).then(() => toast("Copied"))}>Copy prompt</button>
        </div>
      ) : null}
      {draft ? (
        <div className="glass-panel" style={{ padding: 12, marginTop: 12 }}>
          <input className="input" value={draft.title} onChange={(e) => setDraft({ ...draft, title: e.target.value })} />
          <textarea className="textarea" value={draft.body} onChange={(e) => setDraft({ ...draft, body: e.target.value })} />
          <button className="btn primary" type="button" onClick={() => {
            confirm("Add this version?", draft.title, () => {
              void api.addVersion({ projectId, version: draft.title || "0.1.0", changelog: draft.body }).then(onDone);
            });
          }}>Add version</button>
        </div>
      ) : null}
      {todos.length ? (
        <div className="glass-panel" style={{ padding: 12, marginTop: 12 }}>
          {todos.map((t, i) => (
            <label key={i} className="row" style={{ margin: "6px 0" }}>
              <input type="checkbox" checked={Boolean(picked[i])} onChange={(e) => setPicked((s) => ({ ...s, [i]: e.target.checked }))} />
              <span><b>{t.title}</b> · {t.priority}<br /><span className="muted">{t.reasoning}</span></span>
            </label>
          ))}
          <button className="btn primary" type="button" onClick={() => {
            confirm("Insert selected TODOs?", "They will be created as open tasks.", () => {
              void (async () => {
                for (const [i, t] of todos.entries()) {
                  if (!picked[i]) continue;
                  await api.createTodo({ projectId, title: t.title, description: t.description, priority: t.priority });
                }
                onDone();
              })();
            });
          }}>Insert selected</button>
        </div>
      ) : null}
      {commit ? (
        <div className="row" style={{ marginTop: 12 }}>
          <input className="input" readOnly value={commit} />
          <button className="btn" type="button" onClick={() => void navigator.clipboard.writeText(commit).then(() => toast("Copied — nothing was committed"))}>Copy</button>
        </div>
      ) : null}
    </div>
  );
}
