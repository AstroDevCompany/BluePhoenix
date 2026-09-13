import { useEffect, useState } from "react";
import type { Category, CommandOutput, ProjectCard, Todo } from "../../lib/types";
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
import { SoftwareAiActions } from "../ai/SoftwareAiActions";

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
  const toast = useUi((s) => s.showToast);
  const confirm = useUi((s) => s.askConfirm);

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
  if (!project) return <div className="muted">Loading…</div>;

  return (
    <div>
      <h1 className="h1">{project.name}</h1>
      <p className="muted">{project.description}</p>
      <div className="row" style={{ margin: "12px 0 8px" }}>
        <span className="badge">{project.status}</span>
        <span className="badge">{category.name}</span>
        {project.tags.map((t) => <span className="tag" key={t.id}>{t.name}</span>)}
      </div>
      {hasCap(category, "topics") && topics.length > 0 ? (
        <label className="field">
          <span className="label">Study topic</span>
          <Select
            value={topicId}
            onChange={setTopicId}
            options={[{ value: "", label: "Whole course" }, ...topics.map((t) => ({ value: t.id, label: t.title }))]}
          />
        </label>
      ) : null}
      <div className="row">
        <TimerControls projectId={project.id} label={category.terminology.timerStart} topicId={topicId || undefined} onDone={reload} />
        {hasCap(category, "vscode") && project.localPath ? <button className="btn" type="button" onClick={() => void api.openVscode(project.localPath!)}>VS Code</button> : null}
        {hasCap(category, "terminal") && project.localPath ? <button className="btn" type="button" onClick={() => void api.openTerminal(project.localPath!)}>Terminal</button> : null}
        {project.localPath ? <button className="btn" type="button" onClick={() => void api.openPath(project.localPath!)}>Folder</button> : null}
        {hasCap(category, "github") && project.githubUrl ? <button className="btn" type="button" onClick={() => void api.openUrl(project.githubUrl!)}>GitHub</button> : null}
        {project.websiteUrl ? <button className="btn" type="button" onClick={() => void api.openUrl(project.websiteUrl!)}>Website</button> : null}
        {hasCap(category, "git") ? (
          <>
            <button className="btn" type="button" onClick={() => api.gitRun(project.id, "fetch").then(() => toast("Fetched")).catch((e) => toast(formatError(e), "error"))}>Fetch</button>
            <button className="btn" type="button" onClick={() => api.gitRun(project.id, "pull").then(() => toast("Pulled")).catch((e) => toast(formatError(e), "error"))}>Pull</button>
            <button className="btn" type="button" onClick={() => api.gitRun(project.id, "push").then(() => toast("Pushed")).catch((e) => toast(formatError(e), "error"))}>Push</button>
          </>
        ) : null}
        {project.primaryFilePath ? <button className="btn primary" type="button" onClick={() => void api.openPath(project.primaryFilePath!)}>Open primary file</button> : null}
      </div>
      <div className="section">
        <h2 className="h2">Overview</h2>
        <div className="stat-grid">
          <div className="stat">{category.terminology.timeLabel}<b>{formatDuration(project.totalTrackedSeconds)}</b></div>
          <div className="stat">XP<b>{project.xp}</b></div>
          <div className="stat">Level<b>{project.level}</b></div>
          {hasCap(category, "versions") ? <div className="stat">Version<b>{project.currentVersion ?? "—"}</b></div> : null}
          {hasCap(category, "grades") ? <div className="stat">Final grade<b>{project.finalGrade ?? "—"}</b></div> : null}
          {hasCap(category, "attendance") && project.attendance ? <div className="stat">Attendance<b>{project.attendance.percent.toFixed(1)}%</b></div> : null}
        </div>
        {project.git?.isRepo ? <p className="muted">Git: {project.git.branch} {project.git.dirty ? "(dirty)" : ""} {project.git.ahead ? `↑${project.git.ahead}` : ""} {project.git.behind ? `↓${project.git.behind}` : ""} {project.git.lastCommit}</p> : null}
        {project.git?.error && !project.git.isRepo ? <p className="muted">{project.git.error}</p> : null}
      </div>
      {hasCap(category, "timeTracking") ? <TimeEntries projectId={project.id} onChanged={reload} /> : null}
      {hasCap(category, "todos") ? (
        <div className="section">
          <h2 className="h2">TODOs</h2>
          <TodoList projectId={project.id} categoryId={category.id} todos={todos} onChange={reload} />
        </div>
      ) : null}
      {category.kind === "software" ? (
        <SoftwareAiActions projectId={project.id} changelog={versions[0]?.changelog} onDone={reload} />
      ) : null}
      {hasCap(category, "developmentCommands") ? (
        <div className="section">
          <h2 className="h2">Commands</h2>
          {commands.map((c) => (
            <div key={c.id} className="list-row">
              <span>{c.name}</span>
              <button className="btn" type="button" onClick={() => {
                const go = (confirmed?: boolean) => api.runCommand({ projectId, command: c.command, confirmed }).then((out) => {
                  setOutput(out);
                  toast(out.code === 0 ? "Finished" : `Exit ${out.code}`);
                }).catch((e) => toast(formatError(e), "error"));
                if (c.dangerous) confirm("Run dangerous command?", c.command, () => void go(true));
                else void go();
              }}>Run</button>
            </div>
          ))}
          {output ? (
            <pre className="glass-panel" style={{ padding: 12, overflow: "auto", maxHeight: 240, fontSize: 12 }}>
              {output.stdout || output.stderr || "(no output)"}
            </pre>
          ) : null}
          <CommandEditor projectId={project.id} onDone={reload} />
        </div>
      ) : null}
      {hasCap(category, "versions") ? (
        <div className="section">
          <h2 className="h2">Versions</h2>
          {versions.map((v) => <div key={v.version} className="list-row muted">{v.version} · {formatDate(v.releasedAt)} · {v.changelog}</div>)}
          <VersionEditor projectId={project.id} onDone={reload} />
        </div>
      ) : null}
      {hasCap(category, "topics") ? (
        <div className="section">
          <h2 className="h2">Topics</h2>
          {topics.map((t) => <div key={t.id} className="list-row"><span>{t.title}</span><span className="badge">{t.status}</span><span className="muted">{formatDuration(t.trackedSeconds)}</span></div>)}
          <TopicEditor projectId={project.id} onDone={reload} />
        </div>
      ) : null}
      {hasCap(category, "files") ? (
        <div className="section">
          <h2 className="h2">{category.kind === "university" ? "Documents" : "Files"}</h2>
          <p className="muted">Folder listings are cached. Refresh runs a shallow scan; deep indexing is a background job.</p>
          <button className="btn" type="button" onClick={() => api.scanProjectFolder(projectId).then((r) => setFolder(r as never))}>Refresh folder</button>
          {files.map((f) => (
            <div key={f.id} className="list-row">
              <span>{f.displayName}{f.indexStage ? ` · ${f.indexStage}` : ""}{f.indexStatus && f.indexStatus !== "completed" ? ` (${f.indexStatus})` : ""}</span>
              {f.indexLimitation ? <span className="muted">{f.indexLimitation}</span> : null}
              {f.absolutePath ? <button className="btn" type="button" onClick={() => void api.openPath(f.absolutePath!)}>Open</button> : null}
              {f.absolutePath ? <button className="btn" type="button" onClick={() => void api.revealPath(f.absolutePath!)}>Reveal</button> : null}
            </div>
          ))}
          {folder.map((f) => (
            <div key={f.path} className="list-row">
              <span>{f.isDir ? "Folder" : "File"} · {f.name}</span>
              <button className="btn" type="button" onClick={() => void api.openPath(f.path)}>Open</button>
            </div>
          ))}
          <button className="btn" type="button" onClick={async () => {
            const path = await api.pickFile();
            if (!path) return;
            const name = path.split(/[\\/]/).pop() ?? path;
            await api.addProjectFile({ projectId, displayName: name, absolutePath: path });
            reload();
          }}>Add file</button>
        </div>
      ) : null}
      {hasCap(category, "exams") ? (
        <div className="section">
          <h2 className="h2">Exams</h2>
          {exams.map((e) => <div key={e.id} className="list-row muted">{formatDate(e.date)} · {formatExamStatus(e.status)} {e.grade != null ? `· ${e.grade}/30` : ""}</div>)}
          <ExamEditor projectId={project.id} onDone={reload} />
        </div>
      ) : null}
      {hasCap(category, "attendance") ? (
        <div className="section">
          <h2 className="h2">Attendance</h2>
          <p className="muted">Lessons are not time entries. Study time is tracked separately.</p>
          {lessons.map((l) => <div key={l.id} className="list-row muted">{l.date} · {l.attended ? "attended" : "missed"}</div>)}
          <LessonEditor projectId={project.id} onDone={reload} />
        </div>
      ) : null}
      {hasCap(category, "links") ? (
        <div className="section">
          <h2 className="h2">Resources</h2>
          {links.map((l) => (
            <button key={l.id} className="btn ghost" type="button" onClick={() => void api.openUrl(l.url)}>{l.title}</button>
          ))}
          <LinkEditor projectId={project.id} onDone={reload} />
        </div>
      ) : null}
      {hasCap(category, "grades") ? (
        <div className="section">
          <h2 className="h2">Final grade</h2>
          <p className="muted">GPA only includes courses where you store an explicit final grade here.</p>
          <GradeEditor project={project} onDone={reload} />
        </div>
      ) : null}
      <div className="section">
        <h2 className="h2">Achievements</h2>
        <TrophyRow items={project.achievements} />
      </div>
    </div>
  );
}

function CommandEditor({ projectId, onDone }: { projectId: string; onDone: () => void }) {
  const [name, setName] = useState("dev");
  const [command, setCommand] = useState("pnpm dev");
  return (
    <div className="row" style={{ marginTop: 8 }}>
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
    <div className="row" style={{ marginTop: 8 }}>
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
      <Select
        value={status}
        onChange={setStatus}
        options={EXAM_STATUS_OPTIONS}
      />
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
