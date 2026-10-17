import { useQuery } from "@tanstack/react-query";
import type { Category } from "../../lib/types";
import { api } from "../../lib/ipc";
import { keys } from "../../lib/queries";
import { ErrorNote } from "../../components/ui/ErrorNote";
import { SkeletonRows } from "../../components/ui/Skeleton";
import { formatDate, formatExamStatus } from "../../lib/format";
import { EmptyState } from "../../components/ui/EmptyState";
import { PageHeader } from "../../components/PageHeader";

export function ExamsPage({ category }: { category: Category }) {
  const query = useQuery({
    queryKey: keys.exams(category.id),
    queryFn: async () =>
      (await api.listExams({ categoryId: category.id })) as unknown as {
        id: string;
        projectName: string;
        date?: string;
        status: string;
        grade?: number;
      }[],
  });
  const exams = query.data ?? [];
  return (
    <div>
      <PageHeader title="Exams" kicker={category.name} />
      {query.isError ? (
        <ErrorNote error={query.error} onRetry={() => void query.refetch()} />
      ) : query.isPending ? (
        <SkeletonRows />
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
