import type { Category } from "../../lib/types";
import { useProjects } from "../../lib/queries";
import { ErrorNote } from "../../components/ui/ErrorNote";
import { SkeletonRows } from "../../components/ui/Skeleton";
import { EmptyState } from "../../components/ui/EmptyState";
import { PageHeader } from "../../components/PageHeader";
import { RunButton } from "../projects/ProjectActions";

export function CommandsPage({ category }: { category: Category }) {
  const projectsQuery = useProjects(category.id);
  const pinned = (projectsQuery.data ?? []).filter((p) => p.pinnedCommand);
  return (
    <div>
      <PageHeader title="Commands" kicker={category.name} />
      {projectsQuery.isError ? (
        <ErrorNote error={projectsQuery.error} onRetry={() => void projectsQuery.refetch()} />
      ) : projectsQuery.isPending ? (
        <SkeletonRows />
      ) : pinned.length === 0 ? (
        <EmptyState title="No pinned commands" body="Pin a command on a software project (Commands tab) to run it from here." />
      ) : (
        pinned.map((p) => {
          const cmd = p.pinnedCommand!;
          return (
            <div key={p.id} className="list-row">
              <span className="list-row-title">{cmd.name}</span>
              <span className="list-row-meta">{p.name} · {cmd.command}</span>
              <RunButton project={p} command={cmd} variant="default" small />
            </div>
          );
        })
      )}
    </div>
  );
}
