import { useEffect, useState } from "react";
import { api } from "../../lib/ipc";
import { TrophyRow } from "./TrophyRow";
import type { Unlock } from "../../lib/types";

export function AchievementsPage() {
  const [items, setItems] = useState<Unlock[]>([]);
  const [progress, setProgress] = useState<{ xp: number; level: number; ratio: number } | null>(null);
  useEffect(() => {
    void api.listAchievements().then(setItems);
    void api.globalProgress().then((r) => setProgress(r as { xp: number; level: number; ratio: number }));
  }, []);
  return (
    <div>
      <h1 className="h1">Achievements</h1>
      {progress ? (
        <div className="stat-grid">
          <div className="stat">Level<b>{progress.level}</b></div>
          <div className="stat">XP<b>{progress.xp.toLocaleString()}</b></div>
        </div>
      ) : null}
      {progress ? <div className="progress" aria-hidden="true"><span style={{ width: `${Math.round(progress.ratio * 100)}%` }} /></div> : null}
      <div className="section"><TrophyRow items={items} /></div>
    </div>
  );
}
