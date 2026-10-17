import { useQueryClient } from "@tanstack/react-query";
import { FolderOpen, Star, Trash2 } from "lucide-react";
import type { Category, ProjectCard } from "../../lib/types";
import { hasCap } from "../../lib/types";
import { formatDuration, formatRelative } from "../../lib/format";
import { TrophyRow } from "../achievements/TrophyRow";
import { IconButton } from "../../components/ui/Tooltip";
import { MenuItem, MenuSeparator } from "../../components/ui/Menu";
import { api, formatError } from "../../lib/ipc";
import { useUi } from "../../stores/ui";
import { EditorButton, GitButton, ProjectMoreMenu, RunButton } from "./ProjectActions";
import { GitSummary } from "./GitSummary";
import { StatusDot, projectState } from "../../components/SidebarProjects";

export function ProjectCardView({
  project,
  category,
  onOpen,
  onRefresh,
  layout = "grid",
}: {
  project: ProjectCard;
  category: Category;
  onOpen: () => void;
  onRefresh: () => void;
  layout?: "grid" | "list";
}) {
  const toast = useUi((s) => s.showToast);
  const confirm = useUi((s) => s.askConfirm);
  const client = useQueryClient();
  const noun = category.terminology.itemSingular.toLowerCase();
  const isSoftware = category.kind === "software";

  const askRemove = () => {
    confirm(
      `Remove ${noun}?`,
      `"${project.name}" will be removed from ${category.name}. Files on disk are not deleted.`,
      () => {
        void api
          .deleteProject(project.id)
          .then(() => {
            toast(`Removed ${project.name}`);
            onRefresh();
          })
          .catch((e) => toast(formatError(e), "error"));
      },
      true,
    );
  };

  const toggleFavorite = () => {
    void api
      .updateProject({ id: project.id, favorite: !project.favorite })
      .then(() => client.invalidateQueries({ queryKey: ["projects"] }))
      .catch((e) => toast(formatError(e), "error"));
  };

  const Body =
    category.kind === "university"
      ? UniversityBody
      : isSoftware
        ? SoftwareBody
        : GenericBody;
  return (
    <article className="project-card" onClick={onOpen}>
      <div className="project-card-top">
        {isSoftware ? <StatusDot state={projectState(project)} /> : null}
        <button
          type="button"
          className="project-card-title"
          onClick={(e) => {
            e.stopPropagation();
            onOpen();
          }}
        >
          {project.name}
        </button>
        {project.status !== "active" ? <span className="badge">{project.status}</span> : null}
        <div className="project-card-tools" onClick={(e) => e.stopPropagation()}>
          <IconButton
            label={project.favorite ? "Remove from favorites" : "Add to favorites"}
            className={`btn icon sm quiet star ${project.favorite ? "on" : ""}`}
            onClick={toggleFavorite}
          >
            <Star size={14} fill={project.favorite ? "currentColor" : "none"} />
          </IconButton>
          <ProjectMoreMenu
            project={project}
            extra={
              <>
                <MenuSeparator />
                <MenuItem danger icon={<Trash2 size={14} />} onSelect={askRemove}>Remove {noun}</MenuItem>
              </>
            }
          />
        </div>
      </div>
      {project.description ? <div className="muted project-card-desc">{project.description}</div> : null}
      <Body project={project} category={category} onRefresh={onRefresh} />
      {hasCap(category, "achievements") && layout === "grid" && project.achievements.length > 0 ? (
        <TrophyRow items={project.achievements} />
      ) : null}
    </article>
  );
}

function SoftwareBody({ project }: { project: ProjectCard; category: Category; onRefresh: () => void }) {
  const pinned = project.pinnedCommand;
  const meta = [
    formatDuration(project.totalTrackedSeconds),
    project.currentVersion ? `v${project.currentVersion}` : null,
    `${project.openTodos} TODO${project.openTodos === 1 ? "" : "s"}`,
    project.git?.lastCommitAt ? `updated ${formatRelative(project.git.lastCommitAt)}` : null,
  ].filter(Boolean);
  return (
    <>
      <GitSummary git={project.git} />
      <div className="project-card-meta">{meta.join("  ·  ")}</div>
      <div className="project-card-foot" onClick={(e) => e.stopPropagation()}>
        <EditorButton project={project} small quiet />
        {pinned ? <RunButton project={project} command={pinned} variant="quiet" small /> : null}
        <span className="grow" />
        <GitButton project={project} onlyWhenNeeded small />
      </div>
    </>
  );
}

function UniversityBody({ project }: { project: ProjectCard; category: Category; onRefresh: () => void }) {
  const toast = useUi((s) => s.showToast);
  return (
    <>
      <div className="row muted text-sm spread">
        <span>{formatDuration(project.totalTrackedSeconds)} study</span>
        {project.cfu != null ? <span>{project.cfu} CFU</span> : null}
      </div>
      {project.attendance ? (
        <div className="muted text-sm">
          {project.attendance.lessons} lessons · {project.attendance.attended} attended · {project.attendance.percent.toFixed(1)}%
        </div>
      ) : null}
      <div className="row muted text-sm">
        {project.averageGrade != null ? <span>Average {project.averageGrade.toFixed(1)}/30</span> : null}
        {project.finalGrade != null ? <span>Final {project.finalGrade}/30{project.honors ? " with honors" : ""}</span> : null}
      </div>
      <div className="row" onClick={(e) => e.stopPropagation()}>
        {project.localPath ? (
          <button className="btn" type="button" onClick={() => api.openPath(project.localPath!).catch((e) => toast(formatError(e), "error"))}>
            <FolderOpen size={14} /> Open folder
          </button>
        ) : null}
        {project.primaryFilePath ? (
          <button className="btn primary" type="button" onClick={() => api.openPath(project.primaryFilePath!).catch((e) => toast(formatError(e), "error"))}>
            Open primary file
          </button>
        ) : null}
      </div>
    </>
  );
}

function GenericBody({ project }: { project: ProjectCard; category: Category; onRefresh: () => void }) {
  const toast = useUi((s) => s.showToast);
  return (
    <>
      <div className="muted text-sm">{formatDuration(project.totalTrackedSeconds)}</div>
      <div className="row" onClick={(e) => e.stopPropagation()}>
        {project.localPath ? (
          <button className="btn" type="button" onClick={() => api.openPath(project.localPath!).catch((e) => toast(formatError(e), "error"))}>
            <FolderOpen size={14} /> Open folder
          </button>
        ) : null}
        {project.primaryFilePath ? (
          <button className="btn primary" type="button" onClick={() => api.openPath(project.primaryFilePath!).catch((e) => toast(formatError(e), "error"))}>
            Open primary file
          </button>
        ) : null}
      </div>
    </>
  );
}
