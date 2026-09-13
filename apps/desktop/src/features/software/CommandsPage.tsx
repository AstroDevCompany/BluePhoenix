import { useEffect, useState } from "react";
import type { Category, ProjectCard } from "../../lib/types";
import { api } from "../../lib/ipc";
import { EmptyState } from "../../components/ui/EmptyState";

export function CommandsPage({ category, refreshKey = 0 }: { category: Category; refreshKey?: number }) {
  const [projects, setProjects] = useState<ProjectCard[]>([]);
  useEffect(() => {
    void api.listProjects(category.id).then(setProjects);
  }, [category.id, refreshKey]);
  const pinned = projects.filter((p) => p.pinnedCommand);
  return (
    <div>
      <h1 className="h1">Commands</h1>
      {pinned.length === 0 ? (
        <EmptyState title="No pinned commands" body="Pin a command on a software project to see it here." />
      ) : (
        pinned.map((p) => (
          <div key={p.id} className="list-row">
            <strong>{p.name}</strong>
            <span className="muted">{p.pinnedCommand?.name}</span>
          </div>
        ))
      )}
    </div>
  );
}
