import { useEffect, useState } from "react";
import type { Category } from "../../lib/types";
import { api } from "../../lib/ipc";
import { formatDate, formatExamStatus } from "../../lib/format";
import { EmptyState } from "../../components/ui/EmptyState";

export function ExamsPage({ category, refreshKey = 0 }: { category: Category; refreshKey?: number }) {
  const [exams, setExams] = useState<{ id: string; projectName: string; date?: string; status: string; grade?: number }[]>([]);
  useEffect(() => {
    void api.listExams({ categoryId: category.id }).then((r) => setExams(r as never));
  }, [category.id, refreshKey]);
  return (
    <div>
      <h1 className="h1">Exams</h1>
      {exams.length === 0 ? (
        <EmptyState title="No exam attempts" body="Record attempts from a course page. Failed attempts stay out of GPA unless you change that in Settings." />
      ) : (
        exams.map((e) => (
          <div key={e.id} className="list-row muted">
            {e.projectName} · {formatDate(e.date)} · {formatExamStatus(e.status)} {e.grade != null ? `· ${e.grade}/30` : ""}
          </div>
        ))
      )}
    </div>
  );
}
