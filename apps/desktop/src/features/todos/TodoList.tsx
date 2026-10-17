import { useState } from "react";
import { Trash2 } from "lucide-react";
import type { Todo } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { useUi } from "../../stores/ui";
import { IconButton } from "../../components/ui/Tooltip";
import { EmptyState } from "../../components/ui/EmptyState";

const PRIORITIES = ["low", "medium", "high"] as const;

export function TodoList({
  projectId,
  todos,
  onChange,
}: {
  projectId: string;
  categoryId?: string;
  todos: Todo[];
  onChange: () => void;
  stage?: boolean;
}) {
  const toast = useUi((s) => s.showToast);
  const [title, setTitle] = useState("");
  const [priority, setPriority] = useState<(typeof PRIORITIES)[number]>("medium");
  const [showDone, setShowDone] = useState(false);

  const open = todos.filter((t) => t.status !== "completed");
  const done = todos.filter((t) => t.status === "completed");
  const shown = showDone ? [...open, ...done] : open;

  const fail = (e: unknown) => toast(formatError(e), "error");
  const add = () => {
    const text = title.trim();
    if (!text) return;
    void api
      .createTodo({ projectId, title: text, priority })
      .then(() => {
        setTitle("");
        onChange();
      })
      .catch(fail);
  };

  return (
    <div className="todo-list">
      <form
        className="todo-add"
        onSubmit={(e) => {
          e.preventDefault();
          add();
        }}
      >
        <input
          className="input"
          placeholder="Add a TODO and press Enter…"
          aria-label="New TODO"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
        />
        <div className="seg seg-labels" role="group" aria-label="Priority">
          {PRIORITIES.map((p) => (
            <button key={p} type="button" className={priority === p ? "active" : ""} aria-pressed={priority === p} onClick={() => setPriority(p)}>
              {p}
            </button>
          ))}
        </div>
        <button className="btn primary" type="submit" disabled={!title.trim()}>Add</button>
      </form>
      {open.length === 0 ? (
        <EmptyState
          compact
          title={done.length > 0 ? "All done" : "No TODOs"}
          body={done.length > 0 ? `${done.length} completed.` : "Add a task for this project above."}
        />
      ) : null}
      {shown.slice(0, 200).map((t) => (
        <div key={t.id} className={`todo-row list-row ${t.status === "completed" ? "is-done" : ""}`}>
          <input
            type="checkbox"
            aria-label={`Complete ${t.title}`}
            checked={t.status === "completed"}
            onChange={() => void api.setTodoStatus(t.id, t.status === "completed" ? "open" : "completed").then(onChange).catch(fail)}
          />
          <span className="list-row-title">{t.title}</span>
          <span className={`priority-dot p-${t.priority}`} title={`${t.priority} priority`} aria-label={`${t.priority} priority`} />
          <IconButton label="Delete TODO" className="btn icon ghost todo-delete" onClick={() => void api.deleteTodo(t.id).then(onChange).catch(fail)}>
            <Trash2 size={13} />
          </IconButton>
        </div>
      ))}
      {done.length > 0 ? (
        <button className="btn ghost" type="button" onClick={() => setShowDone((v) => !v)}>
          {showDone ? "Hide" : "Show"} {done.length} completed
        </button>
      ) : null}
    </div>
  );
}
