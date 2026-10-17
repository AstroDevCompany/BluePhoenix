import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { Category, Todo } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { keys } from "../../lib/queries";
import { useUi } from "../../stores/ui";
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
  const client = useQueryClient();
  const todosQuery = useQuery({
    queryKey: keys.todos(category.id),
    queryFn: () => api.listTodos({ categoryId: category.id }),
  });
  const toggle = useMutation({
    mutationFn: (t: Todo) => api.setTodoStatus(t.id, t.status === "completed" ? "open" : "completed"),
    // Flip the row immediately; the refetch below reconciles with the backend.
    onMutate: async (t) => {
      await client.cancelQueries({ queryKey: keys.todos(category.id) });
      const previous = client.getQueryData<Todo[]>(keys.todos(category.id));
      client.setQueryData<Todo[]>(keys.todos(category.id), (old) =>
        old?.map((x) => (x.id === t.id ? { ...x, status: t.status === "completed" ? "open" : "completed" } : x)),
      );
      return { previous };
    },
    onError: (e, _t, ctx) => {
      if (ctx?.previous) client.setQueryData(keys.todos(category.id), ctx.previous);
      useUi.getState().showToast(formatError(e), "error");
    },
    onSettled: () => {
      void client.invalidateQueries({ queryKey: keys.todos(category.id) });
      void client.invalidateQueries({ queryKey: ["projects"] });
    },
  });
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
