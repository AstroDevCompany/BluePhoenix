import { useState } from "react";
import { Pencil, Pin, PinOff, Trash2 } from "lucide-react";
import type { CommandDto, ProjectCard } from "../../../lib/types";
import { api, formatError } from "../../../lib/ipc";
import { useUi } from "../../../stores/ui";
import { RunButton } from "../ProjectActions";
import { IconButton } from "../../../components/ui/Tooltip";
import { EmptyState } from "../../../components/ui/EmptyState";

function CommandForm({
  initial,
  onSave,
  onCancel,
}: {
  initial: CommandDto;
  onSave: (draft: CommandDto) => void;
  onCancel?: () => void;
}) {
  const [draft, setDraft] = useState(initial);
  const valid = draft.name.trim() && draft.command.trim();
  const submit = () => valid && onSave({ ...draft, name: draft.name.trim(), command: draft.command.trim() });
  return (
    <form
      className="command-form"
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
      <input className="input" placeholder="Name (e.g. Dev server)" value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} autoFocus />
      <input className="input mono" placeholder="Command (e.g. pnpm dev)" value={draft.command} onChange={(e) => setDraft({ ...draft, command: e.target.value })} />
      <input className="input mono" placeholder="Subfolder (optional)" value={draft.workingDirectory ?? ""} onChange={(e) => setDraft({ ...draft, workingDirectory: e.target.value || null })} />
      <button className="btn primary" type="submit" disabled={!valid}>Save</button>
      {onCancel ? <button className="btn ghost" type="button" onClick={onCancel}>Cancel</button> : null}
    </form>
  );
}

export function CommandsTab({
  project,
  commands,
  onChange,
}: {
  project: ProjectCard;
  commands: CommandDto[];
  onChange: () => void;
}) {
  const toast = useUi((s) => s.showToast);
  const confirm = useUi((s) => s.askConfirm);
  const [editing, setEditing] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);

  const save = (draft: CommandDto) =>
    api
      .upsertCommand(draft)
      .then(() => {
        setEditing(null);
        setAdding(false);
        onChange();
      })
      .catch((e) => toast(formatError(e), "error"));

  const blank: CommandDto = {
    id: "",
    projectId: project.id,
    name: "",
    command: "",
    description: "",
    workingDirectory: null,
    pinned: !commands.some((c) => c.pinned),
    favorite: false,
    sortOrder: commands.length,
    dangerous: false,
  };

  return (
    <div className="commands-tab">
      {commands.length === 0 && !adding ? (
        <EmptyState
          compact
          title="No commands yet"
          body="Save the commands you run all the time. The pinned one gets a Run button on the project card. The AI tab can scan the project for scripts."
        />
      ) : null}
      {commands.map((c) =>
        editing === c.id ? (
          <CommandForm key={c.id} initial={c} onSave={save} onCancel={() => setEditing(null)} />
        ) : (
          <div key={c.id} className="command-row">
            <div className="command-row-text">
              <span className="command-row-name">
                {c.name}
                {c.pinned ? <span className="badge">pinned</span> : null}
                {c.dangerous ? <span className="badge danger-badge">dangerous</span> : null}
              </span>
              <code className="command-row-cmd">{c.command}{c.workingDirectory ? `  (in ${c.workingDirectory})` : ""}</code>
            </div>
            <RunButton project={project} command={c} primary={false} />
            <IconButton label={c.pinned ? "Unpin" : "Pin to card"} className="btn icon ghost" onClick={() => void save({ ...c, pinned: !c.pinned })}>
              {c.pinned ? <PinOff size={14} /> : <Pin size={14} />}
            </IconButton>
            <IconButton label="Edit" className="btn icon ghost" onClick={() => setEditing(c.id)}>
              <Pencil size={14} />
            </IconButton>
            <IconButton
              label="Delete"
              className="btn icon ghost"
              onClick={() =>
                confirm("Delete command?", c.command, () => {
                  void api.deleteCommand(c.id).then(onChange).catch((e) => toast(formatError(e), "error"));
                }, true)
              }
            >
              <Trash2 size={14} />
            </IconButton>
          </div>
        ),
      )}
      {adding ? (
        <CommandForm initial={blank} onSave={save} onCancel={() => setAdding(false)} />
      ) : (
        <button className="btn" type="button" onClick={() => setAdding(true)}>Add command</button>
      )}
    </div>
  );
}
