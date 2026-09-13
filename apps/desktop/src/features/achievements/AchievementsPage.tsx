import { useEffect, useState } from "react";
import { api } from "../../lib/ipc";
import { TrophyRow } from "./TrophyRow";
import type { Unlock } from "../../lib/types";
import { EmptyState } from "../../components/ui/EmptyState";
import { PageHeader } from "../../components/PageHeader";

export function AchievementsPage() {
  const [items, setItems] = useState<Unlock[]>([]);
  const [progress, setProgress] = useState<{ xp: number; level: number; ratio: number } | null>(null);
  const [ready, setReady] = useState(false);
  useEffect(() => {
    void Promise.all([
      api.listAchievements().then(setItems),
      api.globalProgress().then((r) => setProgress(r as { xp: number; level: number; ratio: number })),
    ]).finally(() => setReady(true));
  }, []);
  return (
    <div>
      <PageHeader title="Achievements" kicker="Progress" />
      {!ready ? <p className="muted">Loading…</p> : null}
      {progress ? (
        <div className="stat-grid">
          <div className="stat">Level<b>{progress.level}</b></div>
          <div className="stat">XP<b>{progress.xp.toLocaleString()}</b></div>
        </div>
      ) : null}
      {progress ? <div className="progress" aria-hidden="true"><span style={{ width: `${Math.round(progress.ratio * 100)}%` }} /></div> : null}
      <div className="section">
        {ready && items.length === 0 ? (
          <EmptyState title="No trophies yet" body="Finish TODOs, pass exams, and keep a streak to unlock the first ones." />
        ) : (
          <TrophyRow items={items} />
        )}
      </div>
    </div>
  );
}
