import type { Category, Todo } from "../../lib/types";
import { useTodos, useToggleTodo } from "../../lib/queries";
import { ErrorNote } from "../../components/ui/ErrorNote";
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

export function TodosPage({ category }: { category: Category }) {
  const todosQuery = useTodos(category.id);
  const toggle = useToggleTodo(category.id);
  const todos = todosQuery.data ?? [];
  const row = (t: Todo) => <TodoRow key={t.id} todo={t} onToggle={() => toggle.mutate(t)} />;
  return (
    <div>
      <PageHeader title="TODOs" kicker={category.name} />
      {todosQuery.isError ? (
        <ErrorNote error={todosQuery.error} onRetry={() => void todosQuery.refetch()} />
      ) : todosQuery.isPending ? (
        <p className="muted">Loading…</p>
      ) : todos.length === 0 ? (
        <EmptyState title="Inbox is empty" body={`TODOs from ${category.terminology.itemPlural.toLowerCase()} appear here.`} />
      ) : todos.length > 24 ? (
        <VirtualList items={todos} render={(t) => row(t)} />
      ) : (
        todos.map(row)
      )}
    </div>
  );
}
