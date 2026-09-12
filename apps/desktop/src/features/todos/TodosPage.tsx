import { useEffect, useState } from "react";
import type { Category, Todo } from "../../lib/types";
import { api } from "../../lib/ipc";
import { EmptyState } from "../../components/ui/EmptyState";
import { VirtualList } from "../../components/ui/VirtualList";

export function TodosPage({ category }: { category: Category }) {
  const [todos, setTodos] = useState<Todo[]>([]);
  useEffect(() => {
    void api.listTodos({ categoryId: category.id }).then(setTodos);
  }, [category.id]);
  return (
    <div>
      <h1 className="h1">TODOs</h1>
      {todos.length === 0 ? (
        <EmptyState title="Inbox is empty" body={`TODOs from ${category.terminology.itemPlural.toLowerCase()} appear here.`} />
      ) : todos.length > 24 ? (
        <VirtualList
          items={todos}
          render={(t) => (
            <label className="row" style={{ margin: "8px 0" }}>
              <input
                type="checkbox"
                checked={t.status === "completed"}
                onChange={() =>
                  void api.setTodoStatus(t.id, t.status === "completed" ? "open" : "completed").then(() =>
                    api.listTodos({ categoryId: category.id }).then(setTodos),
                  )
                }
              />
              <span>{t.title}</span>
              <span className="muted">{t.projectName}</span>
            </label>
          )}
        />
      ) : (
        todos.map((t) => (
          <label key={t.id} className="row" style={{ margin: "8px 0" }}>
            <input
              type="checkbox"
              checked={t.status === "completed"}
              onChange={() =>
                void api.setTodoStatus(t.id, t.status === "completed" ? "open" : "completed").then(() =>
                  api.listTodos({ categoryId: category.id }).then(setTodos),
                )
              }
            />
            <span>{t.title}</span>
            <span className="muted">{t.projectName}</span>
          </label>
        ))
      )}
    </div>
  );
}
