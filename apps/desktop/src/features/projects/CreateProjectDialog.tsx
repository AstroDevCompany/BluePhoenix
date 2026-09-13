import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Plus, X } from "lucide-react";
import type { Bootstrap, Tag } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { useUi } from "../../stores/ui";
import { IconButton, Tooltip } from "../../components/ui/Tooltip";

export function CreateProjectDialog({
  bootstrap,
  initialCategory,
  onClose,
  onCreated,
}: {
  bootstrap: Bootstrap;
  initialCategory: string;
  onClose: () => void;
  onCreated: (id: string, categoryId: string) => void;
}) {
  const enabled = bootstrap.categories.filter((c) => c.enabled);
  const [categoryId, setCategoryId] = useState(initialCategory);
  const category = enabled.find((c) => c.id === categoryId) ?? enabled[0];
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [localPath, setLocalPath] = useState("");
  const [primaryFile, setPrimaryFile] = useState("");
  const [github, setGithub] = useState("");
  const [website, setWebsite] = useState("");
  const [university, setUniversity] = useState("");
  const [professor, setProfessor] = useState("");
  const [year, setYear] = useState("");
  const [semester, setSemester] = useState("");
  const [cfu, setCfu] = useState("");
  const [langIds, setLangIds] = useState<string[]>([]);
  const [fwIds, setFwIds] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [filling, setFilling] = useState(false);
  const toast = useUi((s) => s.showToast);
  const languages = bootstrap.tags.filter((t) => t.kind === "language");
  const frameworks = bootstrap.tags.filter((t) => t.kind === "framework");

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      const target = e.target as HTMLElement | null;
      if (target?.closest(".chip-editor")) return;
      onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const toggle = (list: string[], id: string, set: (v: string[]) => void) => {
    set(list.includes(id) ? list.filter((x) => x !== id) : [...list, id]);
  };

  const submit = async () => {
    if (!category) return;
    setBusy(true);
    try {
      const created = await api.createProject({
        categoryId: category.id,
        name,
        description,
        localPath: localPath || null,
        primaryFilePath: primaryFile || null,
        githubUrl: github || null,
        websiteUrl: website || null,
        languageIds: langIds,
        frameworkIds: fwIds,
        university: university || null,
        professor: professor || null,
        academicYear: year || null,
        semester: semester || null,
        cfu: cfu ? Number(cfu) : null,
      });
      onCreated(created.id, category.id);
    } catch (e) {
      toast(formatError(e), "error");
    } finally {
      setBusy(false);
    }
  };

  const fillWithAi = async () => {
    if (!localPath.trim()) return;
    setFilling(true);
    try {
      const res = await api.aiInspectSoftwareFolder(localPath);
      const draft = res.result;
      if (draft.name.trim()) setName(draft.name.trim());
      if (draft.description.trim()) setDescription(draft.description.trim());
      if (draft.githubUrl.trim()) setGithub(draft.githubUrl.trim());
      if (draft.websiteUrl.trim()) setWebsite(draft.websiteUrl.trim());
      if (draft.languageIds.length) setLangIds(draft.languageIds);
      if (draft.frameworkIds.length) setFwIds(draft.frameworkIds);
    } catch (e) {
      toast(formatError(e), "error");
    } finally {
      setFilling(false);
    }
  };

  const kind = category?.kind;
  const fields = useMemo(() => kind, [kind]);

  return (
    <div className="overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <h2 className="h2">What are you creating?</h2>
        <div className="row" style={{ marginBottom: 16 }}>
          {enabled.map((c) => (
            <button key={c.id} className={`btn ${c.id === category?.id ? "primary" : ""}`} type="button" onClick={() => setCategoryId(c.id)}>
              {c.name}
            </button>
          ))}
        </div>
        <PathField
          label="Folder"
          value={localPath}
          onChange={setLocalPath}
          folder
          extra={fields === "software" ? (
            <button
              className="btn"
              type="button"
              disabled={busy || filling || !localPath.trim()}
              onClick={() => void fillWithAi()}
            >
              {filling ? "Reading…" : "Fill with AI"}
            </button>
          ) : null}
        />
        <label className="field">
          <span className="label">{category?.terminology.itemSingular ?? "Name"}</span>
          <input className="input" value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label className="field">
          <span className="label">Description</span>
          <textarea className="textarea" value={description} onChange={(e) => setDescription(e.target.value)} />
        </label>
        {fields === "software" ? (
          <>
            <label className="field">
              <span className="label">GitHub</span>
              <input className="input" value={github} onChange={(e) => setGithub(e.target.value)} placeholder="https://github.com/..." />
            </label>
            <label className="field">
              <span className="label">Website</span>
              <input className="input" value={website} onChange={(e) => setWebsite(e.target.value)} />
            </label>
            <ChipPick
              label="Languages"
              kind="language"
              items={languages}
              selected={langIds}
              onToggle={(id) => toggle(langIds, id, setLangIds)}
            />
            <ChipPick
              label="Frameworks"
              kind="framework"
              items={frameworks}
              selected={fwIds}
              onToggle={(id) => toggle(fwIds, id, setFwIds)}
            />
          </>
        ) : null}
        {fields === "university" ? (
          <>
            <label className="field"><span className="label">University</span><input className="input" value={university} onChange={(e) => setUniversity(e.target.value)} /></label>
            <label className="field"><span className="label">Professor</span><input className="input" value={professor} onChange={(e) => setProfessor(e.target.value)} /></label>
            <label className="field"><span className="label">Academic year</span><input className="input" value={year} onChange={(e) => setYear(e.target.value)} /></label>
            <label className="field"><span className="label">Semester</span><input className="input" value={semester} onChange={(e) => setSemester(e.target.value)} /></label>
            <label className="field"><span className="label">CFU</span><input className="input" value={cfu} onChange={(e) => setCfu(e.target.value)} /></label>
          </>
        ) : null}
        {fields !== "software" ? <PathField label="Primary file" value={primaryFile} onChange={setPrimaryFile} /> : null}
        <div className="row" style={{ justifyContent: "flex-end", marginTop: 8 }}>
          <button className="btn ghost" type="button" onClick={onClose}>Cancel</button>
          <button className="btn primary" type="button" disabled={busy || filling || !name.trim()} onClick={() => void submit()}>Create</button>
        </div>
      </div>
    </div>
  );
}

function PathField({
  label,
  value,
  onChange,
  folder,
  extra,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  folder?: boolean;
  extra?: ReactNode;
}) {
  const toast = useUi((s) => s.showToast);
  return (
    <label className="field">
      <span className="label">{label}</span>
      <div className="row">
        <input className="input" value={value} onChange={(e) => onChange(e.target.value)} style={{ flex: 1 }} />
        <button className="btn" type="button" onClick={async () => {
          try {
            const picked = folder ? await api.pickFolder() : await api.pickFile();
            if (picked) onChange(picked);
          } catch (e) {
            toast(formatError(e), "error");
          }
        }}>Browse</button>
        {extra}
      </div>
    </label>
  );
}

function ChipPick({
  label,
  kind,
  items,
  selected,
  onToggle,
}: {
  label: string;
  kind: "language" | "framework";
  items: { id: string; name: string }[];
  selected: string[];
  onToggle: (id: string) => void;
}) {
  const toast = useUi((s) => s.showToast);
  const inputRef = useRef<HTMLInputElement>(null);
  const committing = useRef(false);
  const [hidden, setHidden] = useState<Set<string>>(() => new Set());
  const [extras, setExtras] = useState<Tag[]>([]);
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (adding) inputRef.current?.focus();
  }, [adding]);

  const visible = useMemo(() => {
    const byId = new Map<string, { id: string; name: string }>();
    for (const item of items) byId.set(item.id, item);
    for (const item of extras) byId.set(item.id, item);
    return [...byId.values()].filter((item) => !hidden.has(item.id));
  }, [items, extras, hidden]);

  const closeEditor = () => {
    setAdding(false);
    setDraft("");
  };

  const commit = async () => {
    if (committing.current) return;
    const name = draft.trim();
    if (!name) {
      closeEditor();
      return;
    }
    committing.current = true;
    const match = [...items, ...extras].find((item) => item.name.toLowerCase() === name.toLowerCase());
    if (match) {
      setHidden((current) => {
        if (!current.has(match.id)) return current;
        const next = new Set(current);
        next.delete(match.id);
        return next;
      });
      if (!selected.includes(match.id)) onToggle(match.id);
      committing.current = false;
      closeEditor();
      return;
    }
    setSaving(true);
    try {
      const tag = await api.createCustomTag(name, kind);
      setExtras((current) => (current.some((item) => item.id === tag.id) ? current : [...current, tag]));
      if (!selected.includes(tag.id)) onToggle(tag.id);
      closeEditor();
    } catch (e) {
      toast(formatError(e), "error");
    } finally {
      committing.current = false;
      setSaving(false);
    }
  };

  const remove = (id: string) => {
    if (selected.includes(id)) onToggle(id);
    setHidden((current) => new Set(current).add(id));
    setExtras((current) => current.filter((item) => item.id !== id));
  };

  return (
    <div className="field">
      <div className="label">{label}</div>
      <div className="chip-row">
        {visible.map((item) => (
          <div key={item.id} className="chip-wrap">
            <button
              type="button"
              className={`btn chip ${selected.includes(item.id) ? "primary" : ""}`}
              onClick={() => onToggle(item.id)}
            >
              {item.name}
            </button>
            <Tooltip content={`Remove ${item.name}`}>
              <button
                type="button"
                className="chip-x"
                aria-label={`Remove ${item.name}`}
                onClick={() => remove(item.id)}
              >
                <X size={11} strokeWidth={2.5} />
              </button>
            </Tooltip>
          </div>
        ))}
        {adding ? (
          <input
            ref={inputRef}
            className="input chip-editor"
            value={draft}
            disabled={saving}
            placeholder="New tag"
            aria-label={`Add ${label.toLowerCase().replace(/s$/, "")}`}
            style={{ width: `${Math.max(8, draft.length + 2)}ch` }}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={() => {
              if (!saving) void commit();
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                e.stopPropagation();
                void commit();
              }
              if (e.key === "Escape") {
                e.preventDefault();
                e.stopPropagation();
                closeEditor();
              }
            }}
          />
        ) : (
          <IconButton
            className="btn chip chip-add"
            label={`Add ${label.toLowerCase()}`}
            onClick={() => setAdding(true)}
          >
            <Plus size={14} />
          </IconButton>
        )}
      </div>
    </div>
  );
}
