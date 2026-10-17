import { useEffect, useState, type ReactNode } from "react";
import { useNavigate } from "@tanstack/react-router";
import { useQueryClient } from "@tanstack/react-query";
import * as Tabs from "@radix-ui/react-tabs";
import { ArrowLeft, FolderOpen, Github, Globe } from "lucide-react";
import type { Category, CommandDto, ProjectCard, Todo } from "../../lib/types";
import { hasCap } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { formatDate, formatDuration, formatExamStatus, EXAM_STATUS_OPTIONS } from "../../lib/format";
import { TrophyRow } from "../achievements/TrophyRow";
import { TimerControls } from "../time/TimerControls";
import { TimeEntries } from "../time/TimeEntries";
import { TodoList } from "../todos/TodoList";
import { EmptyState } from "../../components/ui/EmptyState";
import { SkeletonRows } from "../../components/ui/Skeleton";
import { Select } from "../../components/ui/Select";
import { SoftwareAiActions } from "../ai/SoftwareAiActions";
import { IconButton } from "../../components/ui/Tooltip";
import { keys, useProject } from "../../lib/queries";
import { EditorButton, GitButton, RunButton, TerminalButton } from "./ProjectActions";
import { GitSummary } from "./GitSummary";
import { GitTab } from "./detail/GitTab";
import { CommandsTab } from "./detail/CommandsTab";

type DetailTab =
  | "overview"
  | "todos"
  | "time"
  | "git"
  | "commands"
  | "versions"
  | "ai"
  | "files"
  | "topics"
  | "attendance"
  | "exams"
  | "docs";

function tabsFor(category: Category): { id: DetailTab; label: string }[] {
  const filesLabel = category.kind === "university" ? "Documents" : "Files";
  const filesId: DetailTab = category.kind === "university" ? "docs" : "files";
  if (category.kind === "university") {
    return [
      { id: "overview", label: "Overview" },
      ...(hasCap(category, "todos") ? [{ id: "todos" as const, label: "TODOs" }] : []),
      ...(hasCap(category, "topics") ? [{ id: "topics" as const, label: "Topics" }] : []),
      ...(hasCap(category, "timeTracking") ? [{ id: "time" as const, label: "Time" }] : []),
      ...(hasCap(category, "attendance") ? [{ id: "attendance" as const, label: "Attendance" }] : []),
      ...(hasCap(category, "exams") ? [{ id: "exams" as const, label: "Exams" }] : []),
      ...(hasCap(category, "files") ? [{ id: filesId, label: filesLabel }] : []),
    ];
  }
  if (category.kind === "software") {
    return [
      { id: "overview", label: "Overview" },
      ...(hasCap(category, "git") ? [{ id: "git" as const, label: "Git" }] : []),
      ...(hasCap(category, "developmentCommands") ? [{ id: "commands" as const, label: "Commands" }] : []),
      ...(hasCap(category, "todos") ? [{ id: "todos" as const, label: "TODOs" }] : []),
      ...(hasCap(category, "timeTracking") ? [{ id: "time" as const, label: "Time" }] : []),
      ...(hasCap(category, "versions") ? [{ id: "versions" as const, label: "Releases" }] : []),
      { id: "ai", label: "AI" },
      ...(hasCap(category, "files") ? [{ id: filesId, label: filesLabel }] : []),
    ];
  }
  return [
    { id: "overview", label: "Overview" },
    ...(hasCap(category, "todos") ? [{ id: "todos" as const, label: "TODOs" }] : []),
    ...(hasCap(category, "timeTracking") ? [{ id: "time" as const, label: "Time" }] : []),
    ...(hasCap(category, "files") ? [{ id: filesId, label: filesLabel }] : []),
  ];
}

export function ProjectDetail({ category, projectId }: { category: Category; projectId: string }) {
  const [todos, setTodos] = useState<Todo[]>([]);
  const [links, setLinks] = useState<{ id: string; title: string; url: string }[]>([]);
  const [files, setFiles] = useState<{ id: string; displayName: string; absolutePath?: string; indexStatus?: string; indexStage?: string; indexLimitation?: string }[]>([]);
  const [folder, setFolder] = useState<{ name: string; path: string; isDir: boolean }[]>([]);
  const [versions, setVersions] = useState<{ version: string; changelog: string; releasedAt?: string }[]>([]);
  const [topics, setTopics] = useState<{ id: string; title: string; status: string; trackedSeconds: number }[]>([]);
  const [exams, setExams] = useState<{ id: string; date?: string; grade?: number; status: string }[]>([]);
  const [commands, setCommands] = useState<CommandDto[]>([]);
  const [lessons, setLessons] = useState<{ id: string; date: string; attended: boolean }[]>([]);
  const [topicId, setTopicId] = useState<string>("");
  const tabs = tabsFor(category);
  const [tab, setTab] = useState<DetailTab>(tabs[0]?.id ?? "overview");
  const client = useQueryClient();
  const navigate = useNavigate();
  const projectQuery = useProject(projectId);
  const project = projectQuery.data;
  const error = projectQuery.isError ? formatError(projectQuery.error) : null;
  const activeTab = tabs.find((item) => item.id === tab) ?? tabs[0];

  const reload = () => {
    void client.invalidateQueries({ queryKey: keys.project(projectId) });
    void client.invalidateQueries({ queryKey: ["projects"] });
    if (hasCap(category, "todos")) void api.listTodos({ projectId }).then(setTodos);
    if (hasCap(category, "links")) void api.listLinks(projectId).then(setLinks);
    if (hasCap(category, "files")) {
      void api.listProjectFiles(projectId).then((r) => setFiles(r as never));
      void api.cachedFiles(projectId).then((r) => setFolder(r as never));
      void api.enqueueIndexJob(projectId);
    }
    if (hasCap(category, "versions")) void api.listVersions(projectId).then((r) => setVersions(r as never));
    if (hasCap(category, "topics")) void api.listTopics(projectId).then((r) => setTopics(r as never));
    if (hasCap(category, "exams")) void api.listExams({ projectId }).then((r) => setExams(r as never));
    if (hasCap(category, "developmentCommands")) void api.listCommands(projectId).then((r) => setCommands(r as never));
    if (hasCap(category, "attendance")) void api.listLessons(projectId).then((r) => setLessons(r as never));
  };

  useEffect(() => {
    reload();
    setTab(tabsFor(category)[0]?.id ?? "overview");
  }, [projectId, category.id]);

  useEffect(() => {
    const pending = files.some((f) => f.indexStatus === "pending" || f.indexStatus === "processing");
    if (!pending) return;
    const id = window.setInterval(() => {
      void api.listProjectFiles(projectId).then((r) => setFiles(r as never));
    }, 1600);
    return () => window.clearInterval(id);
  }, [files, projectId]);

  if (error) return <EmptyState title="Could not open this item" body={error} />;
  if (!project) return <SkeletonRows count={6} />;

  const isSoftware = category.kind === "software";
  const backToList = () =>
    void navigate({ to: "/workspace/$categoryId/$page", params: { categoryId: category.id, page: "items" } });

  return (
    <div className="detail-page">
      <header className="detail-head">
        <button type="button" className="detail-back" onClick={backToList}>
          <ArrowLeft size={14} /> {category.terminology.itemPlural}
        </button>
        <div className="detail-head-row">
          <div className="detail-head-id">
            <h1 className="h1">{project.name}</h1>
            <div className="row mt-2">
              <span className="badge">{project.status}</span>
              {project.tags.map((t) => <span className="tag" key={t.id}>{t.name}</span>)}
            </div>
            {isSoftware ? <GitSummary git={project.git} showCommit={false} /> : null}
          </div>
          <div className="row detail-head-actions">
            {hasCap(category, "topics") && topics.length > 0 ? (
              <Select
                value={topicId}
                onChange={setTopicId}
                options={[{ value: "", label: "Whole course" }, ...topics.map((t) => ({ value: t.id, label: t.title }))]}
              />
            ) : null}
            {isSoftware ? (
              <>
                {hasCap(category, "vscode") ? <EditorButton project={project} /> : null}
                {hasCap(category, "terminal") ? <TerminalButton project={project} /> : null}
                {project.pinnedCommand ? <RunButton project={project} command={project.pinnedCommand} /> : null}
                {hasCap(category, "git") ? <GitButton project={project} /> : null}
              </>
            ) : null}
            <TimerControls projectId={project.id} label={category.terminology.timerStart} topicId={topicId || undefined} onDone={reload} />
            {project.localPath ? (
              <IconButton label="Open folder" className="btn icon" onClick={() => void api.openPath(project.localPath!)}>
                <FolderOpen size={14} />
              </IconButton>
            ) : null}
            {hasCap(category, "github") && project.githubUrl ? (
              <IconButton label="GitHub" className="btn icon" onClick={() => void api.openUrl(project.githubUrl!)}>
                <Github size={14} />
              </IconButton>
            ) : null}
            {project.websiteUrl ? (
              <IconButton label="Website" className="btn icon" onClick={() => void api.openUrl(project.websiteUrl!)}>
                <Globe size={14} />
              </IconButton>
            ) : null}
            {project.primaryFilePath ? <button className="btn primary" type="button" onClick={() => void api.openPath(project.primaryFilePath!)}>Open primary file</button> : null}
          </div>
        </div>
        <Tabs.Root value={tab} onValueChange={(v) => setTab(v as DetailTab)}>
          <Tabs.List className="detail-tabs" aria-label="Item sections">
            {tabs.map((item) => (
              <Tabs.Trigger key={item.id} value={item.id}>
                {item.label}
              </Tabs.Trigger>
            ))}
          </Tabs.List>
        </Tabs.Root>
      </header>

      <div className="detail-stage">
        <div key={tab} className="detail-pane page-enter">
          <h2 className="detail-hero">{activeTab?.label ?? "Overview"}</h2>
          {project.description && tab === "overview" ? <p className="muted detail-lede">{project.description}</p> : null}

          {tab === "overview" ? (
            <div className="detail-overview">
              <section className="detail-section">
                <h3 className="detail-section-title">Stats</h3>
                <div className="metric-row">
                  <div className="metric"><span>{category.terminology.timeLabel}</span><b>{formatDuration(project.totalTrackedSeconds)}</b></div>
                  <div className="metric"><span>XP</span><b>{project.xp}</b></div>
                  <div className="metric"><span>Level</span><b>{project.level}</b></div>
                  {hasCap(category, "versions") ? <div className="metric"><span>Version</span><b>{project.currentVersion ?? "—"}</b></div> : null}
                  {hasCap(category, "grades") ? <div className="metric"><span>Final grade</span><b>{project.finalGrade ?? "—"}</b></div> : null}
                  {hasCap(category, "attendance") && project.attendance ? <div className="metric"><span>Attendance</span><b>{project.attendance.percent.toFixed(1)}%</b></div> : null}
                </div>
              </section>
              {project.git?.isRepo || project.git?.error ? (
                <section className="detail-section">
                  <h3 className="detail-section-title">Git</h3>
                  <GitSummary git={project.git} />
                  {project.git?.isRepo ? (
                    <button className="btn ghost" type="button" onClick={() => setTab("git")}>Branches, changes and history</button>
                  ) : null}
                </section>
              ) : null}
              <section className="detail-section">
                <h3 className="detail-section-title">Achievements</h3>
                {project.achievements.length > 0 ? <TrophyRow items={project.achievements} /> : <p className="muted">No trophies on this item yet.</p>}
              </section>
              {hasCap(category, "grades") ? (
                <section className="detail-section">
                  <h3 className="detail-section-title">Grade</h3>
                  <p className="muted">GPA only includes courses where you store an explicit final grade here.</p>
                  <AddDisclosure label="Set final grade">
                    <GradeEditor project={project} onDone={reload} />
                  </AddDisclosure>
                </section>
              ) : null}
              {hasCap(category, "links") ? (
                <section className="detail-section">
                  <h3 className="detail-section-title">Resources</h3>
                  {links.length === 0 ? <p className="muted">No resources yet.</p> : null}
                  {links.map((l) => (
                    <div key={l.id} className="list-row">
                      <span className="list-row-title">{l.title}</span>
                      <button className="btn ghost" type="button" onClick={() => void api.openUrl(l.url)}>Open</button>
                    </div>
                  ))}
                  <AddDisclosure label="Add resource">
                    <LinkEditor projectId={project.id} onDone={reload} />
                  </AddDisclosure>
                </section>
              ) : null}
            </div>
          ) : null}

          {tab === "todos" ? (
            <TodoList projectId={project.id} categoryId={category.id} todos={todos} onChange={reload} stage />
          ) : null}

          {tab === "time" ? <TimeEntries projectId={project.id} topicId={topicId || undefined} onChanged={reload} stage /> : null}

          {tab === "topics" ? (
            <>
              {topics.length === 0 ? <QuietEmpty title="No topics" body="Break the course into study units." /> : null}
              {topics.map((t) => (
                <div key={t.id} className="list-row">
                  <span className="list-row-title">{t.title}</span>
                  <span className="badge">{t.status}</span>
                  <span className="list-row-meta">{formatDuration(t.trackedSeconds)}</span>
                </div>
              ))}
              <AddDisclosure label="Add topic">
                <TopicEditor projectId={project.id} onDone={reload} />
              </AddDisclosure>
            </>
          ) : null}

          {tab === "attendance" ? (
            <>
              {lessons.length === 0 ? <QuietEmpty title="No lessons yet" body="Log attended or missed sessions." /> : null}
              {lessons.map((l) => (
                <div key={l.id} className="list-row">
                  <span className="list-row-title">{l.date}</span>
                  <span className="list-row-meta">{l.attended ? "Attended" : "Missed"}</span>
                </div>
              ))}
              <AddDisclosure label="Add lesson">
                <LessonEditor projectId={project.id} onDone={reload} />
              </AddDisclosure>
            </>
          ) : null}

          {tab === "git" ? <GitTab project={project} /> : null}

          {tab === "commands" ? <CommandsTab project={project} commands={commands} onChange={reload} /> : null}

          {tab === "versions" ? (
            <>
              {versions.length === 0 ? <QuietEmpty title="No versions" body="Record a release when you ship." /> : null}
              {versions.map((v) => (
                <div key={v.version} className="list-row">
                  <span className="list-row-title">{v.version}</span>
                  <span className="list-row-meta">{formatDate(v.releasedAt)}{v.changelog ? ` · ${v.changelog}` : ""}</span>
                </div>
              ))}
              <AddDisclosure label="Add version">
                <VersionEditor projectId={project.id} onDone={reload} />
              </AddDisclosure>
            </>
          ) : null}

          {tab === "ai" ? <SoftwareAiActions projectId={project.id} changelog={versions[0]?.changelog} onDone={reload} /> : null}

          {tab === "exams" ? (
            <>
              {exams.length === 0 ? <QuietEmpty title="No exam attempts" body="Save a scheduled or completed attempt." /> : null}
              {exams.map((e) => (
                <div key={e.id} className="list-row">
                  <span className="list-row-title">{formatExamStatus(e.status)}</span>
                  <span className="list-row-meta">{formatDate(e.date)}{e.grade != null ? ` · ${e.grade}/30` : ""}</span>
                </div>
              ))}
              <AddDisclosure label="Add attempt">
                <ExamEditor projectId={project.id} onDone={reload} />
              </AddDisclosure>
            </>
          ) : null}

          {tab === "files" || tab === "docs" ? (
            <>
              {files.length === 0 && folder.length === 0 ? <QuietEmpty title="No files yet" body="Attach a file or refresh the project folder." /> : null}
              {files.map((f) => (
                <div key={f.id} className="list-row">
                  <span className="list-row-title">{f.displayName}{f.indexStage ? ` · ${f.indexStage}` : ""}{f.indexStatus && f.indexStatus !== "completed" ? ` (${f.indexStatus})` : ""}</span>
                  {f.indexLimitation ? <span className="list-row-meta">{f.indexLimitation}</span> : null}
                  {f.absolutePath ? <button className="btn" type="button" onClick={() => void api.openPath(f.absolutePath!)}>Open</button> : null}
                  {f.absolutePath ? <button className="btn" type="button" onClick={() => void api.revealPath(f.absolutePath!)}>Reveal</button> : null}
                </div>
              ))}
              {folder.map((f) => (
                <div key={f.path} className="list-row">
                  <span className="list-row-title">{f.name}</span>
                  <span className="list-row-meta">{f.isDir ? "Folder" : "File"}</span>
                  <button className="btn" type="button" onClick={() => void api.openPath(f.path)}>Open</button>
                </div>
              ))}
              <AddDisclosure label="Add or refresh">
                <div className="row">
                  <button className="btn" type="button" onClick={() => api.scanProjectFolder(projectId).then((r) => setFolder(r as never))}>Refresh folder</button>
                  <button className="btn" type="button" onClick={async () => {
                    const path = await api.pickFile();
                    if (!path) return;
                    const name = path.split(/[\\/]/).pop() ?? path;
                    await api.addProjectFile({ projectId, displayName: name, absolutePath: path });
                    reload();
                  }}>Add file</button>
                </div>
              </AddDisclosure>
            </>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function QuietEmpty({ title, body }: { title: string; body: string }) {
  return <EmptyState compact title={title} body={body} />;
}

function AddDisclosure({ label, children }: { label: string; children: ReactNode }) {
  return (
    <details className="add-disclosure">
      <summary className="btn">{label}</summary>
      <div className="add-disclosure-body glass-panel">{children}</div>
    </details>
  );
}

function VersionEditor({ projectId, onDone }: { projectId: string; onDone: () => void }) {
  const [version, setVersion] = useState("0.1.0");
  const [changelog, setChangelog] = useState("");
  return (
    <div className="row">
      <input className="input" value={version} onChange={(e) => setVersion(e.target.value)} />
      <input className="input" value={changelog} onChange={(e) => setChangelog(e.target.value)} placeholder="Changelog" />
      <button className="btn" type="button" onClick={() => api.addVersion({ projectId, version, changelog }).then(onDone)}>Release</button>
    </div>
  );
}

function TopicEditor({ projectId, onDone }: { projectId: string; onDone: () => void }) {
  const [title, setTitle] = useState("");
  return (
    <div className="row">
      <input className="input" value={title} onChange={(e) => setTitle(e.target.value)} placeholder="Topic title" />
      <button className="btn" type="button" onClick={() => api.upsertTopic({ id: "", projectId, title, description: "", sortOrder: 0, status: "not_started", completionPercent: 0 }).then(() => { setTitle(""); onDone(); })}>Add topic</button>
    </div>
  );
}

function ExamEditor({ projectId, onDone }: { projectId: string; onDone: () => void }) {
  const [date, setDate] = useState("");
  const [grade, setGrade] = useState("");
  const [status, setStatus] = useState("scheduled");
  return (
    <div className="row">
      <input className="input" type="date" value={date} onChange={(e) => setDate(e.target.value)} />
      <Select value={status} onChange={setStatus} options={EXAM_STATUS_OPTIONS} />
      <input className="input" placeholder="Grade" value={grade} onChange={(e) => setGrade(e.target.value)} />
      <button className="btn" type="button" onClick={() => api.upsertExam({ id: "", projectId, projectName: "", date, grade: grade ? Number(grade) : null, status, notes: null }).then(onDone)}>Save attempt</button>
    </div>
  );
}

function LessonEditor({ projectId, onDone }: { projectId: string; onDone: () => void }) {
  const [date, setDate] = useState("");
  const [attended, setAttended] = useState(true);
  return (
    <div className="row">
      <input className="input" type="date" value={date} onChange={(e) => setDate(e.target.value)} />
      <label className="row"><input type="checkbox" checked={attended} onChange={(e) => setAttended(e.target.checked)} /> Attended</label>
      <button className="btn" type="button" onClick={() => api.upsertLesson({ id: "", projectId, date, attended, durationSeconds: 3600 }).then(onDone)}>Add lesson</button>
    </div>
  );
}

function LinkEditor({ projectId, onDone }: { projectId: string; onDone: () => void }) {
  const [title, setTitle] = useState("");
  const [url, setUrl] = useState("");
  return (
    <div className="row">
      <input className="input" placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} />
      <input className="input" placeholder="https://" value={url} onChange={(e) => setUrl(e.target.value)} />
      <button className="btn" type="button" onClick={() => api.upsertLink({ id: "", projectId, title, url, pinned: false, sortOrder: 0 }).then(onDone)}>Add link</button>
    </div>
  );
}

function GradeEditor({ project, onDone }: { project: ProjectCard; onDone: () => void }) {
  const [grade, setGrade] = useState(project.finalGrade?.toString() ?? "");
  const [honors, setHonors] = useState(project.honors);
  return (
    <div className="row">
      <input className="input" value={grade} onChange={(e) => setGrade(e.target.value)} placeholder="27" />
      <label className="row"><input type="checkbox" checked={honors} onChange={(e) => setHonors(e.target.checked)} /> Honors</label>
      <button className="btn" type="button" onClick={() => api.updateProject({ id: project.id, finalGrade: grade ? Number(grade) : null, honors }).then(onDone)}>Save final grade</button>
    </div>
  );
}
