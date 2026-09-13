import { useState } from "react";
import type { Todo } from "../../lib/types";
import { api } from "../../lib/ipc";
import { EmptyState } from "../../components/ui/EmptyState";

export function TodoList({
  projectId,
  categoryId,
  todos,
  onChange,
}: {
  projectId: string;
  categoryId: string;
  todos: Todo[];
  onChange: () => void;
}) {
  const [title, setTitle] = useState("");
  return (
    <div>
      {todos.length === 0 ? <EmptyState title="No TODOs" body="Add a task for this item." /> : null}
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
    </div>
  );
}
