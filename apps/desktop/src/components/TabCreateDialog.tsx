import { useEffect, useState } from "react";
import type { Category, ProjectCard } from "../lib/types";
import { api, formatError } from "../lib/ipc";
import { EXAM_STATUS_OPTIONS } from "../lib/format";
import { useUi } from "../stores/ui";
import { Select } from "./ui/Select";

export type TabCreateKind = "todo" | "exam" | "command";

export function TabCreateDialog({
  kind,
  category,
  onClose,
  onCreated,
}: {
  kind: TabCreateKind;
  category: Category;
  onClose: () => void;
  onCreated: () => void;
}) {
  const toast = useUi((s) => s.showToast);
  const item = category.terminology.itemSingular.toLowerCase();
  const [projects, setProjects] = useState<ProjectCard[]>([]);
  const [projectId, setProjectId] = useState("");
  const [title, setTitle] = useState("");
  const [date, setDate] = useState(() => new Date().toISOString().slice(0, 10));
  const [status, setStatus] = useState("scheduled");
  const [grade, setGrade] = useState("");
  const [name, setName] = useState("dev");
  const [command, setCommand] = useState("pnpm dev");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void api.listProjects(category.id).then((rows) => {
      setProjects(rows);
      setProjectId((current) => current || rows[0]?.id || "");
    });
  }, [category.id]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const heading = kind === "todo" ? "New TODO" : kind === "exam" ? "New exam" : "New command";
  const canSave = Boolean(projectId) && (kind === "todo" ? title.trim() : kind === "command" ? name.trim() && command.trim() : true);

  const submit = async () => {
    if (!canSave) return;
    setBusy(true);
    try {
      if (kind === "todo") {
        await api.createTodo({ projectId, title: title.trim() });
      } else if (kind === "exam") {
        await api.upsertExam({
          id: "",
          projectId,
          projectName: "",
          date,
          grade: grade ? Number(grade) : null,
          status,
          notes: null,
        });
      } else {
        await api.upsertCommand({
          id: "",
          projectId,
          name: name.trim(),
          command: command.trim(),
          description: "",
          pinned: true,
          favorite: true,
          sortOrder: 0,
          dangerous: false,
        });
      }
      onCreated();
    } catch (e) {
      toast(formatError(e), "error");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="overlay" onClick={onClose}>
      <div className="dialog" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
        <h2 className="h2">{heading}</h2>
        {projects.length === 0 ? (
          <p className="muted">Create a {item} first, then you can add this from here.</p>
        ) : (
          <>
            <label className="field">
              <span className="label">{category.terminology.itemSingular}</span>
              <Select
                value={projectId}
                onChange={setProjectId}
                options={projects.map((p) => ({ value: p.id, label: p.name }))}
              />
            </label>
            {kind === "todo" ? (
              <label className="field">
                <span className="label">Title</span>
                <input className="input" value={title} onChange={(e) => setTitle(e.target.value)} autoFocus />
              </label>
            ) : null}
            {kind === "exam" ? (
              <>
                <label className="field">
                  <span className="label">Date</span>
                  <input className="input" type="date" value={date} onChange={(e) => setDate(e.target.value)} />
                </label>
                <label className="field">
                  <span className="label">Status</span>
                  <Select value={status} onChange={setStatus} options={EXAM_STATUS_OPTIONS} />
                </label>
                <label className="field">
                  <span className="label">Grade</span>
                  <input className="input" value={grade} onChange={(e) => setGrade(e.target.value)} placeholder="Optional" />
                </label>
              </>
            ) : null}
            {kind === "command" ? (
              <>
                <label className="field">
                  <span className="label">Name</span>
                  <input className="input" value={name} onChange={(e) => setName(e.target.value)} />
                </label>
                <label className="field">
                  <span className="label">Command</span>
                  <input className="input" value={command} onChange={(e) => setCommand(e.target.value)} />
                </label>
              </>
            ) : null}
          </>
        )}
        <div className="row" style={{ justifyContent: "flex-end", marginTop: 8 }}>
          <button className="btn ghost" type="button" onClick={onClose}>Cancel</button>
          {projects.length > 0 ? (
            <button className="btn primary" type="button" disabled={busy || !canSave} onClick={() => void submit()}>
              Create
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}
