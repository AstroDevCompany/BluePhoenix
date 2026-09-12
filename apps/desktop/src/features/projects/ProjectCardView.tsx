import type { PointerEvent } from "react";
import { FolderOpen, Github, Globe, Play, Terminal, Code2 } from "lucide-react";
import type { Category, ProjectCard } from "../../lib/types";
import { hasCap } from "../../lib/types";
import { formatDuration } from "../../lib/format";
import { TrophyRow } from "../achievements/TrophyRow";
import { IconButton } from "../../components/ui/Tooltip";
import { api, formatError } from "../../lib/ipc";
import { useUi } from "../../stores/ui";

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
}: {
  project: ProjectCard;
  category: Category;
  onOpen: () => void;
  onRefresh: () => void;
}) {
  const Body =
    category.kind === "university"
      ? UniversityBody
      : category.kind === "software"
        ? SoftwareBody
        : GenericBody;
  return (
    <article
      className="project-card"
      onClick={onOpen}
      onKeyDown={(e) => e.key === "Enter" && onOpen()}
      onPointerMove={onCardSpot}
      role="button"
      tabIndex={0}
    >
      <div className="row" style={{ justifyContent: "space-between" }}>
        <strong style={{ fontSize: 16 }}>{project.name}</strong>
        <span className="badge">{project.status}</span>
      </div>
      <div className="muted">{project.description || "No description"}</div>
      <Body project={project} category={category} onRefresh={onRefresh} />
      {hasCap(category, "achievements") ? <TrophyRow items={project.achievements} /> : null}
    </article>
  );
}

function SoftwareBody({
  project,
  onRefresh,
}: {
  project: ProjectCard;
  category: Category;
  onRefresh: () => void;
}) {
  const toast = useUi((s) => s.showToast);
  const confirm = useUi((s) => s.askConfirm);
  const pinned = project.pinnedCommand;
  const run = async (fn: () => Promise<unknown>, ok: string) => {
    try {
      await fn();
      toast(ok);
      onRefresh();
    } catch (e) {
      toast(formatError(e), "error");
    }
  };
  return (
    <>
      <div className="row">
        {project.tags.map((t) => (
          <span className="tag" key={t.id}>{t.name}</span>
        ))}
      </div>
      <div className="row muted" style={{ fontSize: 12, justifyContent: "space-between" }}>
        <span>{formatDuration(project.totalTrackedSeconds)}</span>
        <span>{project.currentVersion ? `v${project.currentVersion}` : "No version"}</span>
        <span>Lv.{project.level}</span>
        <span>{project.openTodos} TODOs</span>
      </div>
      {project.git?.isRepo ? (
        <div className="muted" style={{ fontSize: 12 }}>
          {project.git.branch} {project.git.dirty ? "• dirty" : "• clean"} {project.git.lastCommit ? `• ${project.git.lastCommit}` : ""}
        </div>
      ) : null}
      <div className="row" onClick={(e) => e.stopPropagation()}>
        {pinned ? (
          <button className="btn primary" type="button" onClick={() => {
            const exec = (confirmed?: boolean) =>
              run(() => api.runCommand({ projectId: project.id, command: pinned.command, workingDirectory: project.localPath, confirmed }), `Ran ${pinned.name}`);
            if (pinned.dangerous) {
              confirm("Run dangerous command?", pinned.command, () => void exec(true));
            } else void exec();
          }}>
            <Play size={14} /> {pinned.name}
          </button>
        ) : null}
        <button className="btn" type="button" onClick={() => run(() => api.gitRun(project.id, "pull"), "Pulled")}>Pull</button>
        {project.localPath ? (
          <>
            <IconButton label="VS Code" className="btn icon" onClick={() => run(() => api.openVscode(project.localPath!), "Opened VS Code")}><Code2 size={14} /></IconButton>
            <IconButton label="Open folder" className="btn icon" onClick={() => run(() => api.openPath(project.localPath!), "Opened folder")}><FolderOpen size={14} /></IconButton>
            <IconButton label="Open terminal" className="btn icon" onClick={() => run(() => api.openTerminal(project.localPath!), "Opened terminal")}><Terminal size={14} /></IconButton>
          </>
        ) : null}
        {project.githubUrl ? <IconButton label="GitHub" className="btn icon" onClick={() => run(() => api.openUrl(project.githubUrl!), "Opened GitHub")}><Github size={14} /></IconButton> : null}
        {project.websiteUrl ? <IconButton label="Website" className="btn icon" onClick={() => run(() => api.openUrl(project.websiteUrl!), "Opened website")}><Globe size={14} /></IconButton> : null}
      </div>
    </>
  );
}

function UniversityBody({ project }: { project: ProjectCard; category: Category; onRefresh: () => void }) {
  const toast = useUi((s) => s.showToast);
  return (
    <>
      <div className="row muted" style={{ fontSize: 12, justifyContent: "space-between" }}>
        <span>{formatDuration(project.totalTrackedSeconds)} study</span>
        {project.cfu != null ? <span>{project.cfu} CFU</span> : null}
      </div>
      {project.attendance ? (
        <div className="muted" style={{ fontSize: 12 }}>
          {project.attendance.lessons} lessons · {project.attendance.attended} attended · {project.attendance.percent.toFixed(1)}%
        </div>
      ) : null}
      <div className="row muted" style={{ fontSize: 12 }}>
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
      <div className="muted" style={{ fontSize: 12 }}>{formatDuration(project.totalTrackedSeconds)}</div>
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
