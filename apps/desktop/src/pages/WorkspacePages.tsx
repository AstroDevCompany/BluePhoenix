import { useEffect, useState } from "react";
import type { ActivityEvent, Category, GpaDashboard, ProjectCard } from "../lib/types";
import { api } from "../lib/ipc";
import { ProjectCardView } from "../features/projects/ProjectCardView";
import { ProjectDetail } from "../features/projects/ProjectDetail";
import { TodosPage } from "../features/todos/TodosPage";
import { CommandsPage } from "../features/software/CommandsPage";
import { ExamsPage } from "../features/university/ExamsPage";
import { KindDashboard } from "../features/workspaces/registry";
import { EmptyState } from "../components/ui/EmptyState";
import type { WorkspacePage } from "../components/router";
import { formatDate } from "../lib/format";
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
  if (page === "todos") return <TodosPage category={category} />;
  if (page === "commands") return <CommandsPage category={category} />;
  if (page === "exams") return <ExamsPage category={category} />;
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
  const [activity, setActivity] = useState<ActivityEvent[]>([]);
  useEffect(() => {
    void api.listProjects(category.id).then(setProjects);
    void api.listActivity({ categoryId: category.id }).then((rows) => setActivity(rows as ActivityEvent[]));
    if (category.kind === "university") void api.universityDashboard().then(setDash);
  }, [category.id, category.kind, refreshKey]);

  return (
    <div>
      <div className="row" style={{ justifyContent: "space-between" }}>
        <div>
          <h1 className="h1">{category.name}</h1>
          <p className="muted">{category.terminology.itemPlural}</p>
        </div>
        <button className="btn primary" type="button" onClick={onCreate}>New {category.terminology.itemSingular}</button>
      </div>
      <KindDashboard category={category} projects={projects} gpa={dash} />
      <div className="section">
        <h2 className="h2">{category.terminology.itemPlural}</h2>
        {projects.length === 0 ? (
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
      <div className="section">
        <h2 className="h2">Recent activity</h2>
        {activity.length === 0 ? <p className="muted">Nothing recorded yet.</p> : null}
        {activity.slice(0, 8).map((a) => (
          <div key={a.id} className="list-row muted">{a.projectName ? `${a.projectName} · ` : ""}{a.eventType} · {formatDate(a.createdAt)}</div>
        ))}
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
  useEffect(() => { void api.listProjects(category.id).then(setProjects); }, [category.id, refreshKey]);
  return (
    <div>
      <div className="row" style={{ justifyContent: "space-between" }}>
        <h1 className="h1">{category.terminology.itemPlural}</h1>
        <button className="btn primary" type="button" onClick={onCreate}>New</button>
      </div>
      {projects.length === 0 ? (
        <EmptyState title="Nothing here yet" body={`This list is only ${category.terminology.itemPlural.toLowerCase()} in ${category.name}.`} action={{ label: "Create", onClick: onCreate }} />
      ) : (
        <div className="grid-cards" style={{ marginTop: 20 }}>
          {projects.map((p) => (
            <ProjectCardView key={p.id} project={p} category={category} onOpen={() => onOpen(p.id)} onRefresh={() => void api.listProjects(category.id).then(setProjects)} />
          ))}
        </div>
      )}
    </div>
  );
}
