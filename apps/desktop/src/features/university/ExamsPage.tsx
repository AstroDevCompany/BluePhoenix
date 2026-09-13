import { useEffect, useState } from "react";
import type { Category } from "../../lib/types";
import { api } from "../../lib/ipc";
import { formatDate, formatExamStatus } from "../../lib/format";
import { EmptyState } from "../../components/ui/EmptyState";
import { PageHeader } from "../../components/PageHeader";

export function ExamsPage({ category, refreshKey = 0 }: { category: Category; refreshKey?: number }) {
  const [exams, setExams] = useState<{ id: string; projectName: string; date?: string; status: string; grade?: number }[]>([]);
  const [ready, setReady] = useState(false);
  useEffect(() => {
    setReady(false);
    void api.listExams({ categoryId: category.id }).then((r) => setExams(r as never)).finally(() => setReady(true));
  }, [category.id, refreshKey]);
  return (
    <div>
      <PageHeader title="Exams" kicker={category.name} />
      {!ready ? (
        <p className="muted">Loading…</p>
      ) : exams.length === 0 ? (
        <EmptyState title="No exam attempts" body="Record attempts from a course page. Failed attempts stay out of GPA unless you change that in Settings." />
      ) : (
        exams.map((e) => (
          <div key={e.id} className="list-row">
            <span className="list-row-title">{e.projectName}</span>
            <span className="list-row-meta">
              {formatDate(e.date)} · {formatExamStatus(e.status)}{e.grade != null ? ` · ${e.grade}/30` : ""}
            </span>
          </div>
        ))
      )}
    </div>
  );
}
