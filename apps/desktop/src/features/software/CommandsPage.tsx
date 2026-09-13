import { useEffect, useState } from "react";
import { Play } from "lucide-react";
import type { Category, ProjectCard } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { EmptyState } from "../../components/ui/EmptyState";
import { PageHeader } from "../../components/PageHeader";
import { useUi } from "../../stores/ui";

export function CommandsPage({ category, refreshKey = 0 }: { category: Category; refreshKey?: number }) {
  const [projects, setProjects] = useState<ProjectCard[]>([]);
  const [ready, setReady] = useState(false);
  const toast = useUi((s) => s.showToast);
  const confirm = useUi((s) => s.askConfirm);
  useEffect(() => {
    setReady(false);
    void api.listProjects(category.id).then(setProjects).finally(() => setReady(true));
  }, [category.id, refreshKey]);
  const pinned = projects.filter((p) => p.pinnedCommand);
  return (
    <div>
      <PageHeader title="Commands" kicker={category.name} />
      {!ready ? (
        <p className="muted">Loading…</p>
      ) : pinned.length === 0 ? (
        <EmptyState title="No pinned commands" body="Pin a command on a software project to see it here." />
      ) : (
        pinned.map((p) => {
          const cmd = p.pinnedCommand!;
          const run = (confirmed?: boolean) =>
            api.runCommand({ projectId: p.id, command: cmd.command, workingDirectory: p.localPath, confirmed })
              .then(() => toast(`Ran ${cmd.name}`))
              .catch((e) => toast(formatError(e), "error"));
          return (
            <div key={p.id} className="list-row">
              <span className="list-row-title">{cmd.name}</span>
              <span className="list-row-meta">{p.name}</span>
              <button
                className="btn"
                type="button"
                onClick={() => {
                  if (cmd.dangerous) confirm("Run dangerous command?", cmd.command, () => void run(true), true);
                  else void run();
                }}
              >
                <Play size={14} /> Run
              </button>
            </div>
          );
        })
      )}
    </div>
  );
}
