import { useState } from "react";
import type { Todo } from "../../lib/types";
import { api } from "../../lib/ipc";

export function TodoList({
  projectId,
  categoryId,
  todos,
  onChange,
  stage = false,
}: {
  projectId: string;
  categoryId: string;
  todos: Todo[];
  onChange: () => void;
  stage?: boolean;
}) {
  const [title, setTitle] = useState("");
  const add = (
    <div className="row">
      <input className="input" placeholder="New TODO" value={title} onChange={(e) => setTitle(e.target.value)} />
      <button
        className="btn"
        type="button"
        onClick={() => {
          if (!title.trim()) return;
          void api.createTodo({ projectId, title, categoryId }).then(() => {
            setTitle("");
            onChange();
          });
        }}
      >
        Add
      </button>
    </div>
  );
  return (
    <div>
      {todos.length === 0 ? (
        <div className="detail-empty">
          <p className="detail-empty-title">No TODOs</p>
          <p className="muted">Add a task for this item.</p>
        </div>
      ) : null}
      {todos.slice(0, 40).map((t) => (
        <label key={t.id} className={`list-row ${t.status === "completed" ? "is-done" : ""}`}>
          <input
            type="checkbox"
            checked={t.status === "completed"}
            onChange={() => void api.setTodoStatus(t.id, t.status === "completed" ? "open" : "completed").then(onChange)}
          />
          <span className="list-row-title">{t.title}</span>
        </label>
      ))}
      {stage ? (
        <details className="add-disclosure">
          <summary className="btn">Add TODO</summary>
          <div className="add-disclosure-body glass-panel">{add}</div>
        </details>
      ) : (
        add
      )}
    </div>
  );
}
