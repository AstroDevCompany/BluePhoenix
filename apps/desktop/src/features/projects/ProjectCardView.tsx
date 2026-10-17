import { useQueryClient } from "@tanstack/react-query";
import { Code2, FolderOpen, Github, Globe, MoreHorizontal, Star, Trash2, type LucideIcon } from "lucide-react";
import type { PointerEvent } from "react";
import type { Category, ProjectCard } from "../../lib/types";
import { hasCap } from "../../lib/types";
import { formatDuration } from "../../lib/format";
import { TrophyRow } from "../achievements/TrophyRow";
import { IconButton } from "../../components/ui/Tooltip";
import { Menu, MenuContent, MenuItem, MenuSeparator, MenuTrigger } from "../../components/ui/Menu";
import { api, formatError } from "../../lib/ipc";
import { openInEditor } from "../../lib/projectActions";
import { useUi } from "../../stores/ui";
import { EditorButton, GitButton, RunButton, TerminalButton } from "./ProjectActions";
import { GitSummary } from "./GitSummary";

function onCardSpot(e: PointerEvent<HTMLElement>) {
  const r = e.currentTarget.getBoundingClientRect();
  e.currentTarget.style.setProperty("--spot-x", `${((e.clientX - r.left) / r.width) * 100}%`);
  e.currentTarget.style.setProperty("--spot-y", `${((e.clientY - r.top) / r.height) * 100}%`);
}

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
      : category.kind === "software"
        ? SoftwareBody
        : GenericBody;
  return (
    <article className={`project-card ${layout === "list" ? "is-list" : ""}`} onClick={onOpen} onPointerMove={onCardSpot}>
      <div className="project-card-top">
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
        <div className="project-card-head" onClick={(e) => e.stopPropagation()}>
          <span className="badge">{project.status}</span>
          <IconButton
            label={project.favorite ? "Remove from favorites" : "Add to favorites"}
            className={`btn icon ghost ${project.favorite ? "fav-on" : ""}`}
            onClick={toggleFavorite}
          >
            <Star size={14} fill={project.favorite ? "currentColor" : "none"} />
          </IconButton>
          <Menu>
            <MenuTrigger asChild>
              <button type="button" className="btn icon ghost" aria-label={`More actions for ${project.name}`}>
                <MoreHorizontal size={15} />
              </button>
            </MenuTrigger>
            <MenuContent align="end">
              <MenuItem onSelect={onOpen}>Open</MenuItem>
              {project.localPath && hasCap(category, "vscode") ? (
                <MenuItem icon={<Code2 size={14} />} onSelect={() => void openInEditor(project)}>Open in editor</MenuItem>
              ) : null}
              {project.localPath ? (
                <MenuItem icon={<FolderOpen size={14} />} onSelect={() => void api.revealPath(project.localPath!)}>Reveal folder</MenuItem>
              ) : null}
              <MenuSeparator />
              <MenuItem danger icon={<Trash2 size={14} />} onSelect={askRemove}>Remove {noun}</MenuItem>
            </MenuContent>
          </Menu>
        </div>
      </div>
      {project.description ? <div className="muted project-card-desc">{project.description}</div> : null}
      <Body project={project} category={category} onRefresh={onRefresh} />
      {hasCap(category, "achievements") && layout === "grid" ? <TrophyRow items={project.achievements} /> : null}
    </article>
  );
}

function LinkIcon({ icon: Icon, label, onClick }: { icon: LucideIcon; label: string; onClick: () => void }) {
  return (
    <IconButton label={label} className="btn icon" onClick={onClick}>
      <Icon size={14} />
    </IconButton>
  );
}

function SoftwareBody({ project }: { project: ProjectCard; category: Category; onRefresh: () => void }) {
  const toast = useUi((s) => s.showToast);
  const open = (url: string) => void api.openUrl(url).catch((e) => toast(formatError(e), "error"));
  const pinned = project.pinnedCommand;
  return (
    <>
      {project.tags.length > 0 ? (
        <div className="row">
          {project.tags.map((t) => (
            <span className="tag" key={t.id}>{t.name}</span>
          ))}
        </div>
      ) : null}
      <GitSummary git={project.git} />
      <div className="project-card-stats muted">
        <span>{formatDuration(project.totalTrackedSeconds)}</span>
        <span>{project.currentVersion ? `v${project.currentVersion}` : "No version"}</span>
        <span>{project.openTodos} TODO{project.openTodos === 1 ? "" : "s"}</span>
      </div>
      <div className="project-card-actions" onClick={(e) => e.stopPropagation()}>
        <EditorButton project={project} compact />
        <TerminalButton project={project} compact />
        {pinned ? <RunButton project={project} command={pinned} /> : null}
        <GitButton project={project} />
        <span className="project-card-links">
          {project.githubUrl ? <LinkIcon icon={Github} label="GitHub" onClick={() => open(project.githubUrl!)} /> : null}
          {project.websiteUrl ? <LinkIcon icon={Globe} label="Website" onClick={() => open(project.websiteUrl!)} /> : null}
        </span>
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
