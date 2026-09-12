import { useEffect, useState } from "react";
import type { Category } from "../../lib/types";
import { api } from "../../lib/ipc";
import { formatDate } from "../../lib/format";
import { EmptyState } from "../../components/ui/EmptyState";

export function ExamsPage({ category }: { category: Category }) {
  const [exams, setExams] = useState<{ id: string; projectName: string; date?: string; status: string; grade?: number }[]>([]);
  useEffect(() => {
    void api.listExams({ categoryId: category.id }).then((r) => setExams(r as never));
  }, [category.id]);
  return (
    <div>
      <h1 className="h1">Exams</h1>
      {exams.length === 0 ? (
        <EmptyState title="No exam attempts" body="Record attempts from a course page. Failed attempts stay out of GPA unless you change that in Settings." />
      ) : (
        exams.map((e) => (
          <div key={e.id} className="muted" style={{ padding: "8px 0" }}>
            {e.projectName} · {formatDate(e.date)} · {e.status} {e.grade != null ? `· ${e.grade}/30` : ""}
          </div>
        ))
      )}
    </div>
  );
}
