import { useQuery, useQueryClient } from "@tanstack/react-query";
import type { Category } from "../lib/types";
import { api } from "../lib/ipc";
import { useProjects } from "../lib/queries";
import { ProjectCardView } from "../features/projects/ProjectCardView";
import { ProjectDetail } from "../features/projects/ProjectDetail";
import { TodosPage } from "../features/todos/TodosPage";
import { CommandsPage } from "../features/software/CommandsPage";
import { ExamsPage } from "../features/university/ExamsPage";
import { KindDashboard } from "../features/workspaces/registry";
import { EmptyState } from "../components/ui/EmptyState";
import { ErrorNote } from "../components/ui/ErrorNote";
import { PageHeader } from "../components/PageHeader";
import type { WorkspacePage } from "../components/router";
import { useUi } from "../stores/ui";

export function WorkspaceHome({
  category,
  page,
  projectId,
  onOpenProject,
}: {
  category: Category;
  page: WorkspacePage;
  projectId?: string;
  onOpenProject: (id: string) => void;
  onCreate?: () => void;
}) {
  const openCreate = useUi((s) => s.openCreate);
  if (projectId) return <ProjectDetail category={category} projectId={projectId} />;
  if (page === "todos") return <TodosPage category={category} />;
  if (page === "commands") return <CommandsPage category={category} />;
  if (page === "exams") return <ExamsPage category={category} />;
  if (page === "items") return <ItemsPage category={category} onOpen={onOpenProject} onCreate={openCreate} />;
  return <Overview category={category} onOpen={onOpenProject} onCreate={openCreate} />;
}

function useProjectsRefresh() {
  const client = useQueryClient();
  return () => void client.invalidateQueries({ queryKey: ["projects"] });
}

function Overview({
  category,
  onOpen,
  onCreate,
}: {
  category: Category;
  onOpen: (id: string) => void;
  onCreate: () => void;
}) {
  const projects = useProjects(category.id);
  const dash = useQuery({
    queryKey: ["university-dashboard"],
    queryFn: () => api.universityDashboard(),
    enabled: category.kind === "university",
  });
  const refresh = useProjectsRefresh();
  const list = projects.data ?? [];

  return (
    <div>
      <PageHeader title={category.name} kicker={category.terminology.itemPlural} />
      {projects.isError ? <ErrorNote error={projects.error} onRetry={() => void projects.refetch()} /> : null}
      {projects.isPending ? <p className="muted">Loading…</p> : null}
      {projects.isSuccess ? <KindDashboard category={category} projects={list} gpa={dash.data ?? null} /> : null}
      <div className="section">
        <h2 className="h2">{category.terminology.itemPlural}</h2>
        {!projects.isSuccess ? null : list.length === 0 ? (
          <EmptyState
            title={`No ${category.terminology.itemPlural.toLowerCase()} yet`}
            body={`Create your first ${category.terminology.itemSingular.toLowerCase()}. Cards, dashboards, and tools stay specific to this workspace.`}
            action={{ label: `New ${category.terminology.itemSingular}`, onClick: onCreate }}
          />
        ) : (
          <div className="grid-cards">
            {list.map((p) => (
              <ProjectCardView key={p.id} project={p} category={category} onOpen={() => onOpen(p.id)} onRefresh={refresh} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

function ItemsPage({
  category,
  onOpen,
  onCreate,
}: {
  category: Category;
  onOpen: (id: string) => void;
  onCreate: () => void;
}) {
  const projects = useProjects(category.id);
  const refresh = useProjectsRefresh();
  const list = projects.data ?? [];
  return (
    <div>
      <PageHeader title={category.terminology.itemPlural} kicker={category.name} />
      {projects.isError ? (
        <ErrorNote error={projects.error} onRetry={() => void projects.refetch()} />
      ) : projects.isPending ? (
        <p className="muted">Loading…</p>
      ) : list.length === 0 ? (
        <EmptyState title="Nothing here yet" body={`This list is only ${category.terminology.itemPlural.toLowerCase()} in ${category.name}.`} action={{ label: `New ${category.terminology.itemSingular}`, onClick: onCreate }} />
      ) : (
        <div className="grid-cards">
          {list.map((p) => (
            <ProjectCardView key={p.id} project={p} category={category} onOpen={() => onOpen(p.id)} onRefresh={refresh} />
          ))}
        </div>
      )}
    </div>
  );
}
