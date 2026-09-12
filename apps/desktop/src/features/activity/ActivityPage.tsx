import { useEffect, useState } from "react";
import { api } from "../../lib/ipc";
import { formatDate } from "../../lib/format";
import { EmptyState } from "../../components/ui/EmptyState";
import { VirtualList } from "../../components/ui/VirtualList";
import type { ActivityEvent } from "../../lib/types";

export function ActivityPage() {
  const [rows, setRows] = useState<ActivityEvent[]>([]);
  useEffect(() => {
    void api.listActivity({}).then((r) => setRows(r as ActivityEvent[]));
  }, []);
  return (
    <div>
      <h1 className="h1">Activity</h1>
      {rows.length === 0 ? (
        <EmptyState title="Quiet so far" body="Creates, timers, exams, and TODOs will show up here." />
      ) : rows.length > 30 ? (
        <VirtualList
          items={rows}
          render={(r) => (
            <div className="muted" style={{ padding: "8px 0" }}>
              {r.projectName ? `${r.projectName} · ` : ""}{r.eventType} · {formatDate(r.createdAt)}
            </div>
          )}
        />
      ) : (
        rows.map((r) => (
          <div key={r.id} className="muted" style={{ padding: "8px 0" }}>
            {r.projectName ? `${r.projectName} · ` : ""}{r.eventType} · {formatDate(r.createdAt)}
          </div>
        ))
      )}
    </div>
  );
}
