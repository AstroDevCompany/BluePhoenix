import { useEffect, useState } from "react";
import type { Category, Todo } from "../../lib/types";
import { api } from "../../lib/ipc";
import { EmptyState } from "../../components/ui/EmptyState";
import { PageHeader } from "../../components/PageHeader";
import { VirtualList } from "../../components/ui/VirtualList";

function TodoRow({
  todo,
  onToggle,
}: {
  todo: Todo;
  onToggle: () => void;
}) {
  return (
    <label className={`list-row ${todo.status === "completed" ? "is-done" : ""}`}>
      <input
        type="checkbox"
        checked={todo.status === "completed"}
        onChange={onToggle}
      />
      <span className="list-row-title">{todo.title}</span>
      <span className="list-row-meta">{todo.projectName}</span>
    </label>
  );
}

export function TodosPage({ category, refreshKey = 0 }: { category: Category; refreshKey?: number }) {
  const [todos, setTodos] = useState<Todo[]>([]);
  const [ready, setReady] = useState(false);
  const reload = () => api.listTodos({ categoryId: category.id }).then(setTodos);
  useEffect(() => {
    setReady(false);
    void reload().finally(() => setReady(true));
  }, [category.id, refreshKey]);
  return (
    <div>
      <PageHeader title="TODOs" kicker={category.name} />
      {!ready ? (
        <p className="muted">Loading…</p>
      ) : todos.length === 0 ? (
        <EmptyState title="Inbox is empty" body={`TODOs from ${category.terminology.itemPlural.toLowerCase()} appear here.`} />
      ) : todos.length > 24 ? (
        <VirtualList
          items={todos}
          render={(t) => (
            <TodoRow
              todo={t}
              onToggle={() =>
                void api.setTodoStatus(t.id, t.status === "completed" ? "open" : "completed").then(() => void reload())
              }
            />
          )}
        />
      ) : (
        todos.map((t) => (
          <TodoRow
            key={t.id}
            todo={t}
            onToggle={() =>
              void api.setTodoStatus(t.id, t.status === "completed" ? "open" : "completed").then(() => void reload())
            }
          />
        ))
      )}
    </div>
  );
}
