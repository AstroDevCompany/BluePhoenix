import { useEffect, useMemo, useState } from "react";
import type { Bootstrap } from "../lib/types";
import { api, formatError } from "../lib/ipc";
import { useUi } from "../stores/ui";
import type { Route } from "./router";

export function CommandPalette({
  bootstrap,
  onNavigate,
}: {
  bootstrap: Bootstrap;
  onNavigate: (route: Route) => void;
}) {
  const open = useUi((s) => s.paletteOpen);
  const setPalette = useUi((s) => s.setPalette);
  const toast = useUi((s) => s.showToast);
  const [q, setQ] = useState("");
  const [hits, setHits] = useState<{ title: string; subtitle: string; run: () => void }[]>([]);
  const [active, setActive] = useState(0);
  const [interpreted, setInterpreted] = useState(false);
  const [aiNote, setAiNote] = useState<string | null>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPalette(!open);
      }
      if (e.key === "Escape") setPalette(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, setPalette]);

  const staticActions = useMemo(() => {
    const enabled = bootstrap.categories.filter((c) => c.enabled);
    const actions = [
      { title: "Open Settings", subtitle: "Preferences", run: () => onNavigate({ name: "settings", tab: "appearance" }) },
      { title: "Open Achievements", subtitle: "Progress", run: () => onNavigate({ name: "achievements" }) },
      { title: "Sync now", subtitle: "Cloud", run: () => { void api.pushSync().then(() => toast("Sync complete")).catch((e) => toast(formatError(e), "error")); } },
    ];
    for (const c of enabled) {
      actions.push({
        title: `Open ${c.name}`,
        subtitle: "Workspace",
        run: () => onNavigate({ name: "workspace", categoryId: c.id, page: "overview" }),
      });
    }
    return actions;
  }, [bootstrap.categories, onNavigate, toast]);

  useEffect(() => {
    let cancelled = false;
    const run = async () => {
      const query = q.trim().toLowerCase();
      const local = staticActions.filter((a) => a.title.toLowerCase().includes(query) || a.subtitle.toLowerCase().includes(query));
      if (!query) {
        setHits(local);
        setInterpreted(false);
        setAiNote(null);
        return;
      }
      try {
        const remote = await api.search(q);
        const mapped = remote.map((hit) => ({
          title: hit.title,
          subtitle: hit.subtitle || hit.entityType,
          run: () => {
            if (hit.entityType === "project" && hit.categoryId) {
              onNavigate({ name: "workspace", categoryId: hit.categoryId, page: "items", projectId: hit.entityId });
            } else if (hit.projectId && hit.categoryId) {
              onNavigate({ name: "workspace", categoryId: hit.categoryId, page: "items", projectId: hit.projectId });
            }
          },
        }));
        if (!cancelled) setHits([...mapped, ...local].slice(0, 20));
      } catch {
        if (!cancelled) setHits(local);
      }
    };
    void run();
    return () => {
      cancelled = true;
    };
  }, [q, staticActions, onNavigate]);

  if (!open) return null;
  return (
    <div className="overlay" onClick={() => setPalette(false)}>
      <div className="palette menu-pop" onClick={(e) => e.stopPropagation()}>
        <input
          autoFocus
          placeholder="Search projects, courses, TODOs, actions…"
          value={q}
          onChange={(e) => { setQ(e.target.value); setActive(0); setInterpreted(false); }}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown") { e.preventDefault(); setActive((a) => Math.min(hits.length - 1, a + 1)); }
            if (e.key === "ArrowUp") { e.preventDefault(); setActive((a) => Math.max(0, a - 1)); }
            if (e.key === "Enter") {
              e.preventDefault();
              const sentence = q.trim().length >= 12 && q.includes(" ") && !q.trim().startsWith("/");
              if (sentence && !interpreted) {
                void (async () => {
                  try {
                    const res = await api.aiInterpretSearch(q);
                    const extra = (res.interpretation?.projectIds ?? []).map((id) => {
                      const hit = res.hits.find((h) => h.entityId === id || h.projectId === id);
                      return {
                        title: hit?.title ?? id,
                        subtitle: res.interpretation?.explanation || hit?.subtitle || "AI match",
                        run: () => {
                          const categoryId = hit?.categoryId;
                          const projectId = hit?.projectId ?? hit?.entityId;
                          if (categoryId && projectId) {
                            onNavigate({ name: "workspace", categoryId, page: "items", projectId });
                          }
                        },
                      };
                    });
                    setHits([...extra, ...hits].slice(0, 20));
                    setInterpreted(true);
                    setAiNote(res.interpretation?.explanation ?? null);
                    setActive(0);
                  } catch {
                    if (hits[active]) { hits[active].run(); setPalette(false); }
                  }
                })();
                return;
              }
              if (hits[active]) {
                hits[active].run();
                setPalette(false);
              }
            }
          }}
        />
        {aiNote ? <div className="muted" style={{ padding: "8px 16px", fontSize: 12 }}>{aiNote}</div> : null}
        <div className="scroll" style={{ maxHeight: 360 }}>
          {hits.map((hit, i) => (
            <button
              key={`${hit.title}-${i}`}
              className={`palette-item ${i === active ? "active" : ""}`}
              type="button"
              onClick={() => { hit.run(); setPalette(false); }}
            >
              <div>{hit.title}</div>
              <div className="muted" style={{ fontSize: 12 }}>{hit.subtitle}</div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
