import { useCallback, useState } from "react";
import type { ProjectCard } from "../../lib/types";
import { needsAttention } from "./GitSummary";

export type SortKey = "recent" | "name" | "time" | "attention";
export type FilterKey = "all" | "attention" | "favorites" | "active" | "paused" | "completed";
export type LayoutKey = "grid" | "list";

export type ProjectView = { query: string; sort: SortKey; filter: FilterKey; layout: LayoutKey };

export const DEFAULT_VIEW: ProjectView = { query: "", sort: "recent", filter: "all", layout: "grid" };

const storageKey = (categoryId: string) => `bluephoenix.projectView.${categoryId}`;

function load(categoryId: string): ProjectView {
  try {
    const raw = window.localStorage.getItem(storageKey(categoryId));
    return raw ? { ...DEFAULT_VIEW, ...(JSON.parse(raw) as Partial<ProjectView>), query: "" } : DEFAULT_VIEW;
  } catch {
    return DEFAULT_VIEW;
  }
}

/** Per-workspace search/sort/filter/layout, remembered between visits (except the search text). */
export function useProjectView(categoryId: string) {
  const [view, setView] = useState<ProjectView>(() => load(categoryId));
  const [forCategory, setForCategory] = useState(categoryId);
  if (forCategory !== categoryId) {
    setForCategory(categoryId);
    setView(load(categoryId));
  }
  const update = useCallback(
    (patch: Partial<ProjectView>) => {
      setView((current) => {
        const next = { ...current, ...patch };
        try {
          window.localStorage.setItem(storageKey(categoryId), JSON.stringify({ ...next, query: "" }));
        } catch {
          /* preferences are a convenience only */
        }
        return next;
      });
    },
    [categoryId],
  );
  return [view, update] as const;
}

export function attentionScore(p: ProjectCard) {
  const g = p.git;
  if (!g || g.pending || !g.isRepo) return 0;
  return g.behind * 3 + (g.dirty ? 2 : 0) + g.ahead;
}

/** Applies search, filter and sort; favorites always come first within the chosen order. */
export function applyView(projects: ProjectCard[], view: ProjectView): ProjectCard[] {
  const q = view.query.trim().toLowerCase();
  const filtered = projects.filter((p) => {
    if (q) {
      const haystack = [p.name, p.description, p.git?.branch, ...p.tags.map((t) => t.name)].join(" ").toLowerCase();
      if (!haystack.includes(q)) return false;
    }
    switch (view.filter) {
      case "attention":
        return needsAttention(p.git);
      case "favorites":
        return p.favorite;
      case "active":
      case "paused":
      case "completed":
        return p.status === view.filter;
      default:
        return true;
    }
  });
  const compare: Record<SortKey, (a: ProjectCard, b: ProjectCard) => number> = {
    recent: (a, b) => (b.lastActivity ?? "").localeCompare(a.lastActivity ?? ""),
    name: (a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: "base" }),
    time: (a, b) => b.totalTrackedSeconds - a.totalTrackedSeconds,
    attention: (a, b) => attentionScore(b) - attentionScore(a),
  };
  return [...filtered].sort((a, b) => Number(b.favorite) - Number(a.favorite) || compare[view.sort](a, b));
}
