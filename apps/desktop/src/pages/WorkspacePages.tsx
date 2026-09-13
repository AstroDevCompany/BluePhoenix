import { useEffect, useState } from "react";
import type { Category, GpaDashboard, ProjectCard } from "../lib/types";
import { api } from "../lib/ipc";
import { ProjectCardView } from "../features/projects/ProjectCardView";
import { ProjectDetail } from "../features/projects/ProjectDetail";
import { TodosPage } from "../features/todos/TodosPage";
import { CommandsPage } from "../features/software/CommandsPage";
import { ExamsPage } from "../features/university/ExamsPage";
import { KindDashboard } from "../features/workspaces/registry";
import { EmptyState } from "../components/ui/EmptyState";
import { PageHeader } from "../components/PageHeader";
import type { WorkspacePage } from "../components/router";
import { useUi } from "../stores/ui";

export function WorkspaceHome({
  category,
  page,
  projectId,
  onOpenProject,
  onCreate: _onCreate,
  refreshKey,
}: {
  category: Category;
  page: WorkspacePage;
  projectId?: string;
  onOpenProject: (id: string) => void;
  onCreate: () => void;
  refreshKey: number;
}) {
  const openCreate = useUi((s) => s.openCreate);
  if (projectId) return <ProjectDetail category={category} projectId={projectId} />;
  if (page === "todos") return <TodosPage category={category} refreshKey={refreshKey} />;
  if (page === "commands") return <CommandsPage category={category} refreshKey={refreshKey} />;
  if (page === "exams") return <ExamsPage category={category} refreshKey={refreshKey} />;
  if (page === "items") return <ItemsPage category={category} onOpen={onOpenProject} onCreate={openCreate} refreshKey={refreshKey} />;
  return <Overview category={category} onOpen={onOpenProject} onCreate={openCreate} refreshKey={refreshKey} />;
}

function Overview({
  category,
  onOpen,
  onCreate,
  refreshKey,
}: {
  category: Category;
  onOpen: (id: string) => void;
  onCreate: () => void;
  refreshKey: number;
}) {
  const [projects, setProjects] = useState<ProjectCard[]>([]);
  const [dash, setDash] = useState<GpaDashboard | null>(null);
  const [ready, setReady] = useState(false);
  useEffect(() => {
    setReady(false);
    void Promise.all([
      api.listProjects(category.id).then(setProjects),
      category.kind === "university" ? api.universityDashboard().then(setDash) : Promise.resolve(),
    ]).finally(() => setReady(true));
  }, [category.id, category.kind, refreshKey]);

  return (
    <div>
      <PageHeader title={category.name} kicker={category.terminology.itemPlural} />
      {!ready ? <p className="muted">Loading…</p> : null}
      {ready ? <KindDashboard category={category} projects={projects} gpa={dash} /> : null}
      <div className="section">
        <h2 className="h2">{category.terminology.itemPlural}</h2>
        {!ready ? null : projects.length === 0 ? (
          <EmptyState
            title={`No ${category.terminology.itemPlural.toLowerCase()} yet`}
            body={`Create your first ${category.terminology.itemSingular.toLowerCase()}. Cards, dashboards, and tools stay specific to this workspace.`}
            action={{ label: `New ${category.terminology.itemSingular}`, onClick: onCreate }}
          />
        ) : (
          <div className="grid-cards">
            {projects.map((p) => (
              <ProjectCardView key={p.id} project={p} category={category} onOpen={() => onOpen(p.id)} onRefresh={() => void api.listProjects(category.id).then(setProjects)} />
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
  refreshKey,
}: {
  category: Category;
  onOpen: (id: string) => void;
  onCreate: () => void;
  refreshKey: number;
}) {
  const [projects, setProjects] = useState<ProjectCard[]>([]);
  const [ready, setReady] = useState(false);
  useEffect(() => {
    setReady(false);
    void api.listProjects(category.id).then(setProjects).finally(() => setReady(true));
  }, [category.id, refreshKey]);
  return (
    <div>
      <PageHeader title={category.terminology.itemPlural} kicker={category.name} />
      {!ready ? <p className="muted">Loading…</p> : projects.length === 0 ? (
        <EmptyState title="Nothing here yet" body={`This list is only ${category.terminology.itemPlural.toLowerCase()} in ${category.name}.`} action={{ label: `New ${category.terminology.itemSingular}`, onClick: onCreate }} />
      ) : (
        <div className="grid-cards">
          {projects.map((p) => (
            <ProjectCardView key={p.id} project={p} category={category} onOpen={() => onOpen(p.id)} onRefresh={() => void api.listProjects(category.id).then(setProjects)} />
          ))}
        </div>
      )}
    </div>
  );
}
