import { useEffect, useMemo, useState } from "react";
import type { Bootstrap } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { useUi } from "../../stores/ui";

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
  const toast = useUi((s) => s.showToast);
  const languages = bootstrap.tags.filter((t) => t.kind === "language");
  const frameworks = bootstrap.tags.filter((t) => t.kind === "framework");

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
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
        <label className="field">
          <span className="label">{category?.terminology.itemSingular ?? "Name"}</span>
          <input className="input" value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label className="field">
          <span className="label">Description</span>
          <textarea className="textarea" value={description} onChange={(e) => setDescription(e.target.value)} />
        </label>
        <PathField label="Folder" value={localPath} onChange={setLocalPath} folder />
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
            <ChipPick label="Languages" items={languages} selected={langIds} onToggle={(id) => toggle(langIds, id, setLangIds)} />
            <ChipPick label="Frameworks" items={frameworks} selected={fwIds} onToggle={(id) => toggle(fwIds, id, setFwIds)} />
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
          <button className="btn primary" type="button" disabled={busy || !name.trim()} onClick={() => void submit()}>Create</button>
        </div>
      </div>
    </div>
  );
}

function PathField({ label, value, onChange, folder }: { label: string; value: string; onChange: (v: string) => void; folder?: boolean }) {
  return (
    <label className="field">
      <span className="label">{label}</span>
      <div className="row">
        <input className="input" value={value} onChange={(e) => onChange(e.target.value)} style={{ flex: 1 }} />
        <button className="btn" type="button" onClick={async () => {
          const picked = folder ? await api.pickFolder() : await api.pickFile();
          if (picked) onChange(picked);
        }}>Browse</button>
      </div>
    </label>
  );
}

function ChipPick({
  label,
  items,
  selected,
  onToggle,
}: {
  label: string;
  items: { id: string; name: string }[];
  selected: string[];
  onToggle: (id: string) => void;
}) {
  return (
    <div className="field">
      <div className="label">{label}</div>
      <div className="row">
        {items.map((item) => (
          <button key={item.id} type="button" className={`btn ${selected.includes(item.id) ? "primary" : ""}`} onClick={() => onToggle(item.id)} style={{ height: 28 }}>
            {item.name}
          </button>
        ))}
      </div>
    </div>
  );
}
