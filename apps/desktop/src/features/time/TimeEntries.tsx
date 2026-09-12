import { useEffect, useState } from "react";
import type { TimeEntry } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { formatDate, formatDuration } from "../../lib/format";
import { useUi } from "../../stores/ui";
import { EmptyState } from "../../components/ui/EmptyState";

export function TimeEntries({ projectId, onChanged }: { projectId: string; onChanged: () => void }) {
  const [rows, setRows] = useState<TimeEntry[]>([]);
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [notes, setNotes] = useState("");
  const toast = useUi((s) => s.showToast);

  const load = () => {
    void api.listTimeEntries(projectId).then(setRows);
  };
  useEffect(() => {
    load();
  }, [projectId]);

  return (
    <div className="section">
      <h2 className="h2">Time entries</h2>
      <p className="muted">Elapsed time is stored as start and end timestamps, not a running JavaScript counter.</p>
      {rows.length === 0 ? (
        <EmptyState title="No entries yet" body="Start the timer or add a manual range." />
      ) : (
        rows.slice(0, 40).map((row) => (
          <div key={row.id} className="list-row spread">
            <span>
              {formatDate(row.startedAt)} · {formatDuration(row.seconds)}
              {row.topicTitle ? ` · ${row.topicTitle}` : ""}
              {row.notes ? ` · ${row.notes}` : ""}
            </span>
            <button
              className="btn ghost"
              type="button"
              onClick={() => api.updateTimeEntry({ id: row.id, notes: "" }).then(() => { load(); onChanged(); }).catch((e) => toast(formatError(e), "error"))}
            >
              Clear note
            </button>
          </div>
        ))
      )}
      <div className="row" style={{ marginTop: 10 }}>
        <input className="input" type="datetime-local" value={start} onChange={(e) => setStart(e.target.value)} aria-label="Start" />
        <input className="input" type="datetime-local" value={end} onChange={(e) => setEnd(e.target.value)} aria-label="End" />
        <input className="input" placeholder="Notes" value={notes} onChange={(e) => setNotes(e.target.value)} />
        <button
          className="btn"
          type="button"
          onClick={() => {
            if (!start || !end) return;
            void api
              .addManualEntry({
                projectId,
                startedAt: new Date(start).toISOString(),
                endedAt: new Date(end).toISOString(),
                notes,
              })
              .then(() => {
                setNotes("");
                load();
                onChanged();
              })
              .catch((e) => toast(formatError(e), "error"));
          }}
        >
          Add range
        </button>
      </div>
    </div>
  );
}
