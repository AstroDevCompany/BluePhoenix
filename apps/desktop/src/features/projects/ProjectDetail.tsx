import { useEffect, useRef, useState, type ReactNode, Children } from "react";
import { ChevronDown } from "lucide-react";
import type { Category, CommandOutput, GitStatus, ProjectCard, Todo } from "../../lib/types";
import { hasCap } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { formatDate, formatDuration, formatExamStatus, EXAM_STATUS_OPTIONS } from "../../lib/format";
import { TrophyRow } from "../achievements/TrophyRow";
import { useUi } from "../../stores/ui";
import { TimerControls } from "../time/TimerControls";
import { TimeEntries } from "../time/TimeEntries";
import { TodoList } from "../todos/TodoList";
import { EmptyState } from "../../components/ui/EmptyState";
import { Select } from "../../components/ui/Select";
import { useOpenTransition } from "../../components/ui/useOpenTransition";
import { SoftwareAiActions } from "../ai/SoftwareAiActions";

type DetailTab =
  | "overview"
  | "todos"
  | "time"
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
      ...(hasCap(category, "todos") ? [{ id: "todos" as const, label: "TODOs" }] : []),
      ...(hasCap(category, "timeTracking") ? [{ id: "time" as const, label: "Time" }] : []),
      ...(hasCap(category, "developmentCommands") ? [{ id: "commands" as const, label: "Commands" }] : []),
      ...(hasCap(category, "versions") ? [{ id: "versions" as const, label: "Versions" }] : []),
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
  const [project, setProject] = useState<ProjectCard | null>(null);
  const [todos, setTodos] = useState<Todo[]>([]);
  const [links, setLinks] = useState<{ id: string; title: string; url: string }[]>([]);
  const [files, setFiles] = useState<{ id: string; displayName: string; absolutePath?: string; indexStatus?: string; indexStage?: string; indexLimitation?: string }[]>([]);
  const [folder, setFolder] = useState<{ name: string; path: string; isDir: boolean }[]>([]);
  const [versions, setVersions] = useState<{ version: string; changelog: string; releasedAt?: string }[]>([]);
  const [topics, setTopics] = useState<{ id: string; title: string; status: string; trackedSeconds: number }[]>([]);
  const [exams, setExams] = useState<{ id: string; date?: string; grade?: number; status: string }[]>([]);
  const [commands, setCommands] = useState<{ id: string; name: string; command: string; dangerous: boolean }[]>([]);
  const [lessons, setLessons] = useState<{ id: string; date: string; attended: boolean }[]>([]);
  const [output, setOutput] = useState<CommandOutput | null>(null);
  const [topicId, setTopicId] = useState<string>("");
  const [error, setError] = useState<string | null>(null);
  const tabs = tabsFor(category);
  const [tab, setTab] = useState<DetailTab>(tabs[0]?.id ?? "overview");
  const toast = useUi((s) => s.showToast);
  const confirm = useUi((s) => s.askConfirm);
  const activeTab = tabs.find((item) => item.id === tab) ?? tabs[0];

  const reload = () => {
    void api.getProject(projectId).then(setProject).catch((e) => setError(formatError(e)));
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
  if (!project) return <p className="muted">Loading…</p>;

  const moreOpen = (
    <>
      {hasCap(category, "vscode") && project.localPath ? <button className="btn" type="button" onClick={() => void api.openVscode(project.localPath!)}>VS Code</button> : null}
      {hasCap(category, "terminal") && project.localPath ? <button className="btn" type="button" onClick={() => void api.openTerminal(project.localPath!)}>Terminal</button> : null}
      {hasCap(category, "github") && project.githubUrl ? <button className="btn" type="button" onClick={() => void api.openUrl(project.githubUrl!)}>GitHub</button> : null}
      {project.websiteUrl ? <button className="btn" type="button" onClick={() => void api.openUrl(project.websiteUrl!)}>Website</button> : null}
      {hasCap(category, "git") ? (
        <>
          <button className="btn" type="button" onClick={() => api.gitRun(project.id, "fetch").then(() => toast("Fetched")).catch((e) => toast(formatError(e), "error"))}>Fetch</button>
          <button className="btn" type="button" onClick={() => api.gitRun(project.id, "pull").then(() => toast("Pulled")).catch((e) => toast(formatError(e), "error"))}>Pull</button>
          <button className="btn" type="button" onClick={() => api.gitRun(project.id, "push").then(() => toast("Pushed")).catch((e) => toast(formatError(e), "error"))}>Push</button>
        </>
      ) : null}
    </>
  );

  return (
    <div className="detail-page">
      <header className="detail-head">
        <div className="detail-head-row">
          <div className="detail-head-id">
            <h1 className="h1">{project.name}</h1>
            <div className="row" style={{ marginTop: 6 }}>
              <span className="badge">{project.status}</span>
              {project.tags.map((t) => <span className="tag" key={t.id}>{t.name}</span>)}
            </div>
          </div>
          <div className="row detail-head-actions">
            {hasCap(category, "topics") && topics.length > 0 ? (
              <Select
                value={topicId}
                onChange={setTopicId}
                options={[{ value: "", label: "Whole course" }, ...topics.map((t) => ({ value: t.id, label: t.title }))]}
              />
            ) : null}
            <TimerControls projectId={project.id} label={category.terminology.timerStart} topicId={topicId || undefined} onDone={reload} />
            {project.localPath ? <button className="btn" type="button" onClick={() => void api.openPath(project.localPath!)}>Folder</button> : null}
            {project.primaryFilePath ? <button className="btn primary" type="button" onClick={() => void api.openPath(project.primaryFilePath!)}>Open primary file</button> : null}
            <MoreMenu>{moreOpen}</MoreMenu>
          </div>
        </div>
        <div className="detail-tabs" role="tablist" aria-label="Item sections">
          {tabs.map((item) => (
            <button key={item.id} type="button" role="tab" aria-selected={tab === item.id} onClick={() => setTab(item.id)}>
              {item.label}
            </button>
          ))}
        </div>
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
                  {project.git?.isRepo ? <GitTable git={project.git} /> : <p className="muted">{project.git?.error}</p>}
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

          {tab === "commands" ? (
            <>
              {commands.length === 0 ? <QuietEmpty title="No commands" body="Save a pinned command to run it from here." /> : null}
              {commands.map((c) => (
                <div key={c.id} className="list-row">
                  <span className="list-row-title">{c.name}</span>
                  <span className="list-row-meta">{c.command}</span>
                  <button className="btn" type="button" onClick={() => {
                    const go = (confirmed?: boolean) => api.runCommand({ projectId, command: c.command, confirmed }).then((out) => {
                      setOutput(out);
                      toast(out.code === 0 ? "Finished" : `Exit ${out.code}`);
                    }).catch((e) => toast(formatError(e), "error"));
                    if (c.dangerous) confirm("Run dangerous command?", c.command, () => void go(true), true);
                    else void go();
                  }}>Run</button>
                </div>
              ))}
              {output ? <pre className="glass-panel command-output">{output.stdout || output.stderr || "(no output)"}</pre> : null}
              <AddDisclosure label="Add command">
                <CommandEditor projectId={project.id} onDone={reload} />
              </AddDisclosure>
            </>
          ) : null}

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
  return (
    <div className="detail-empty">
      <p className="detail-empty-title">{title}</p>
      <p className="muted">{body}</p>
    </div>
  );
}

function AddDisclosure({ label, children }: { label: string; children: ReactNode }) {
  return (
    <details className="add-disclosure">
      <summary className="btn">{label}</summary>
      <div className="add-disclosure-body glass-panel">{children}</div>
    </details>
  );
}

function GitTable({ git }: { git: GitStatus }) {
  return (
    <table className="git-table">
      <thead>
        <tr>
          <th>Branch</th>
          <th>Status</th>
          <th>Ahead</th>
          <th>Behind</th>
          <th>Last commit</th>
          <th>When</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>{git.branch || "—"}</td>
          <td>{git.dirty ? "Dirty" : "Clean"}</td>
          <td>{git.ahead}</td>
          <td>{git.behind}</td>
          <td className="git-commit">{git.lastCommit || "—"}</td>
          <td>{formatDate(git.lastCommitAt)}</td>
        </tr>
      </tbody>
    </table>
  );
}

function MoreMenu({ children }: { children: ReactNode }) {
  const { open, visible, hide, toggle } = useOpenTransition();
  const root = useRef<HTMLDivElement>(null);
  const items = Children.toArray(children).filter(Boolean);

  useEffect(() => {
    const onDoc = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) hide();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") hide();
    };
    document.addEventListener("mousedown", onDoc);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDoc);
      document.removeEventListener("keydown", onKey);
    };
  }, [hide]);

  if (items.length === 0) return null;
  return (
    <div className="more-menu" ref={root}>
      <button className="btn" type="button" aria-haspopup="menu" aria-expanded={open} onClick={toggle}>
        Open <ChevronDown size={14} />
      </button>
      {visible ? (
        <div
          className={`menu-pop more-menu-pop glass-panel ${open ? "is-open" : "is-closing"}`}
          role="menu"
          onClick={hide}
        >
          {children}
        </div>
      ) : null}
    </div>
  );
}

function CommandEditor({ projectId, onDone }: { projectId: string; onDone: () => void }) {
  const [name, setName] = useState("dev");
  const [command, setCommand] = useState("pnpm dev");
  return (
    <div className="row">
      <input className="input" value={name} onChange={(e) => setName(e.target.value)} placeholder="Name" />
      <input className="input" value={command} onChange={(e) => setCommand(e.target.value)} placeholder="Command" />
      <button className="btn" type="button" onClick={() => api.upsertCommand({ id: "", projectId, name, command, description: "", pinned: true, favorite: true, sortOrder: 0, dangerous: false }).then(onDone)}>Save</button>
    </div>
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
