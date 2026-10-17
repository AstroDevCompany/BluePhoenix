import { ArrowUpDown, LayoutGrid, List, Search, X } from "lucide-react";
import type { ProjectCard } from "../../lib/types";
import { Menu, MenuContent, MenuItem, MenuTrigger } from "../../components/ui/Menu";
import { needsAttention } from "./GitSummary";
import type { FilterKey, ProjectView, SortKey } from "./projectView";

const SORTS: { id: SortKey; label: string }[] = [
  { id: "recent", label: "Recent activity" },
  { id: "name", label: "Name" },
  { id: "time", label: "Time tracked" },
  { id: "attention", label: "Needs attention" },
];

export function ProjectsToolbar({
  projects,
  view,
  onChange,
  isSoftware,
}: {
  projects: ProjectCard[];
  view: ProjectView;
  onChange: (patch: Partial<ProjectView>) => void;
  isSoftware: boolean;
}) {
  const attentionCount = projects.filter((p) => needsAttention(p.git)).length;
  const chips: { id: FilterKey; label: string; count?: number; show: boolean }[] = [
    { id: "all", label: "All", count: projects.length, show: true },
    { id: "attention", label: "Needs attention", count: attentionCount, show: isSoftware },
    { id: "favorites", label: "Favorites", count: projects.filter((p) => p.favorite).length, show: true },
    { id: "active", label: "Active", show: true },
    { id: "paused", label: "Paused", show: true },
    { id: "completed", label: "Done", show: true },
  ];
  const sort = SORTS.find((s) => s.id === view.sort) ?? SORTS[0];
  return (
    <div className="projects-toolbar">
      <label className="toolbar-search">
        <Search size={14} aria-hidden />
        <input
          type="search"
          placeholder="Filter by name, tag or branch…"
          aria-label="Filter projects"
          value={view.query}
          onChange={(e) => onChange({ query: e.target.value })}
        />
        {view.query ? (
          <button type="button" aria-label="Clear filter" onClick={() => onChange({ query: "" })}>
            <X size={13} />
          </button>
        ) : null}
      </label>
      <div className="toolbar-chips" role="group" aria-label="Filter">
        {chips
          .filter((c) => c.show)
          .map((c) => (
            <button
              key={c.id}
              type="button"
              className={`chip-btn ${view.filter === c.id ? "active" : ""}`}
              aria-pressed={view.filter === c.id}
              onClick={() => onChange({ filter: c.id })}
            >
              {c.label}
              {c.count ? <span className="chip-count">{c.count}</span> : null}
            </button>
          ))}
      </div>
      <div className="toolbar-end">
        <Menu>
          <MenuTrigger asChild>
            <button type="button" className="btn" aria-label={`Sort: ${sort.label}`}>
              <ArrowUpDown size={14} /> {sort.label}
            </button>
          </MenuTrigger>
          <MenuContent align="end">
            {SORTS.filter((s) => isSoftware || s.id !== "attention").map((s) => (
              <MenuItem key={s.id} onSelect={() => onChange({ sort: s.id })}>
                {s.label}
              </MenuItem>
            ))}
          </MenuContent>
        </Menu>
        <div className="seg" role="group" aria-label="Layout">
          <button type="button" className={view.layout === "grid" ? "active" : ""} aria-pressed={view.layout === "grid"} aria-label="Grid view" onClick={() => onChange({ layout: "grid" })}>
            <LayoutGrid size={14} />
          </button>
          <button type="button" className={view.layout === "list" ? "active" : ""} aria-pressed={view.layout === "list"} aria-label="List view" onClick={() => onChange({ layout: "list" })}>
            <List size={14} />
          </button>
        </div>
      </div>
    </div>
  );
}
