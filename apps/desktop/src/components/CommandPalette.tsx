import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import {
  Award,
  CornerDownLeft,
  Download,
  ExternalLink,
  FolderOpen,
  FolderSearch,
  GitPullRequestArrow,
  Globe,
  LayoutDashboard,
  Play,
  RefreshCw,
  Settings as SettingsIcon,
  Square,
  Terminal,
  Upload,
  Code2,
  Plus,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";
import type { Bootstrap, CommandDto, SearchHit } from "../lib/types";
import { hasCap } from "../lib/types";
import { api, formatError } from "../lib/ipc";
import { useUi } from "../stores/ui";
import type { Route } from "./router";
import { Overlay } from "./ui/Overlay";
import { formatCombo } from "../lib/shortcuts";
import type { ProjectWithCategory } from "../lib/queries";
import {
  fetchAll,
  hasLocalFolder,
  openInEditor,
  openSystemTerminal,
  runGit,
  startTimer,
  stopTimer,
} from "../lib/projectActions";
import { openEmbeddedTerminal, runInEmbeddedTerminal } from "../lib/terminalActions";
import { useRunningTimer } from "../lib/queries";

type Item = {
  key: string;
  title: string;
  subtitle?: string;
  group: string;
  hint?: string;
  icon?: ReactNode;
  run: () => void | Promise<void>;
  /** Opens the per-project action list (Tab / →). */
  drill?: () => void;
};

const ICON = 15;

function matches(text: string, q: string) {
  return text.toLowerCase().includes(q);
}

export function CommandPalette({
  bootstrap,
  allProjects,
  currentProject,
  onNavigate,
  onCreate,
}: {
  bootstrap: Bootstrap;
  allProjects: ProjectWithCategory[];
  currentProject?: ProjectWithCategory;
  onNavigate: (route: Route) => void;
  onCreate: () => void;
}) {
  const open = useUi((s) => s.paletteOpen);
  const mode = useUi((s) => s.paletteMode);
  const setPalette = useUi((s) => s.setPalette);
  const recentIds = useUi((s) => s.recentProjectIds);
  const toast = useUi((s) => s.showToast);
  const { data: timer } = useRunningTimer();
  const [q, setQ] = useState("");
  const [active, setActive] = useState(0);
  const [scope, setScope] = useState<ProjectWithCategory | null>(null);
  const [remote, setRemote] = useState<SearchHit[]>([]);
  const [aiNote, setAiNote] = useState<string | null>(null);
  const [aiBusy, setAiBusy] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);

  // Reset whenever the palette is (re)opened.
  useEffect(() => {
    if (open) {
      setQ("");
      setActive(0);
      setScope(null);
      setRemote([]);
      setAiNote(null);
    }
  }, [open, mode]);

  const close = () => setPalette(false);
  const enabled = bootstrap.categories.filter((c) => c.enabled);

  const openProject = (p: ProjectWithCategory) =>
    onNavigate({ name: "workspace", categoryId: p.category.id, page: "items", projectId: p.project.id });

  const runningHere =
    timer && timer.isLocalDevice && !timer.entry.endedAt ? timer.entry.projectId : null;

  // Saved commands of the project the palette is acting on (drilled-in project, else the current one).
  const commandsTarget = scope ?? currentProject;
  const commandsQuery = useQuery({
    queryKey: ["commands", commandsTarget?.project.id],
    queryFn: () => api.listCommands(commandsTarget!.project.id),
    enabled: open && Boolean(commandsTarget) && hasCap(commandsTarget!.category, "developmentCommands"),
  });
  const commands: CommandDto[] = commandsQuery.data ?? [];

  const projectActions = (pc: ProjectWithCategory, isCurrent: boolean, cmds: CommandDto[] = []): Item[] => {
    const { project: p, category } = pc;
    const group = `Actions · ${p.name}`;
    const local = hasLocalFolder(p);
    const items: Item[] = [
      {
        key: `${p.id}:open`,
        title: `Open ${p.name}`,
        group,
        icon: <LayoutDashboard size={ICON} />,
        run: () => openProject(pc),
      },
    ];
    if (hasCap(category, "vscode") && local) {
      items.push({
        key: `${p.id}:editor`,
        title: "Open in editor",
        subtitle: p.localPath ?? undefined,
        group,
        hint: isCurrent ? formatCombo("mod+e") : undefined,
        icon: <Code2 size={ICON} />,
        run: () => openInEditor(p),
      });
    }
    if (hasCap(category, "terminal") && local) {
      items.push(
        {
          key: `${p.id}:terminal`,
          title: "Open terminal here",
          subtitle: "Embedded terminal panel",
          group,
          hint: isCurrent ? formatCombo("mod+j") : undefined,
          icon: <Terminal size={ICON} />,
          run: () => openEmbeddedTerminal(p),
        },
        {
          key: `${p.id}:systerminal`,
          title: "Open in system terminal",
          group,
          icon: <Terminal size={ICON} />,
          run: () => openSystemTerminal(p),
        },
      );
    }
    if (local) {
      cmds.forEach((c) =>
        items.push({
          key: `${p.id}:cmd:${c.id}`,
          title: `Run ${c.name}`,
          subtitle: c.command,
          group,
          icon: <Play size={ICON} />,
          run: () => runInEmbeddedTerminal(p, c),
        }),
      );
    }
    if (local) {
      items.push({
        key: `${p.id}:reveal`,
        title: "Reveal folder",
        group,
        icon: <FolderOpen size={ICON} />,
        run: () => void api.revealPath(p.localPath as string).catch((e) => toast(formatError(e), "error")),
      });
    }
    if (hasCap(category, "git") && local) {
      items.push(
        { key: `${p.id}:fetch`, title: "Git fetch", group, icon: <RefreshCw size={ICON} />, run: () => runGit(p, "fetch") },
        { key: `${p.id}:pull`, title: "Git pull", group, icon: <Download size={ICON} />, run: () => runGit(p, "pull") },
        { key: `${p.id}:push`, title: "Git push", group, icon: <Upload size={ICON} />, run: () => runGit(p, "push") },
      );
    }
    if (hasCap(category, "timeTracking")) {
      items.push(
        runningHere === p.id
          ? { key: `${p.id}:timer`, title: "Stop timer", group, hint: isCurrent ? formatCombo("mod+shift+t") : undefined, icon: <Square size={ICON} />, run: () => stopTimer() }
          : {
              key: `${p.id}:timer`,
              title: runningHere ? "Switch timer to this project" : "Start timer",
              group,
              hint: isCurrent ? formatCombo("mod+shift+t") : undefined,
              icon: <Play size={ICON} />,
              run: () => startTimer(p),
            },
      );
    }
    if (p.githubUrl) {
      items.push({ key: `${p.id}:gh`, title: "Open on GitHub", group, icon: <GitPullRequestArrow size={ICON} />, run: () => void api.openUrl(p.githubUrl as string) });
    }
    if (p.websiteUrl) {
      items.push({ key: `${p.id}:web`, title: "Open website", group, icon: <Globe size={ICON} />, run: () => void api.openUrl(p.websiteUrl as string) });
    }
    return items;
  };

  const globalActions = useMemo<Item[]>(() => {
    const items: Item[] = [
      { key: "g:new", title: "New project", group: "Create", hint: formatCombo("mod+n"), icon: <Plus size={ICON} />, run: onCreate },
    ];
    enabled.forEach((c, i) => {
      items.push({
        key: `g:ws:${c.id}`,
        title: `Open ${c.name}`,
        subtitle: "Workspace",
        group: "Go to",
        hint: i < 9 ? formatCombo(`mod+${i + 1}`) : undefined,
        icon: <LayoutDashboard size={ICON} />,
        run: () => onNavigate({ name: "workspace", categoryId: c.id, page: "overview" }),
      });
    });
    items.push(
      { key: "g:settings", title: "Open Settings", group: "Go to", hint: formatCombo("mod+,"), icon: <SettingsIcon size={ICON} />, run: () => onNavigate({ name: "settings", tab: "appearance" }) },
      { key: "g:ach", title: "Open Achievements", group: "Go to", icon: <Award size={ICON} />, run: () => onNavigate({ name: "achievements" }) },
      {
        key: "g:sync",
        title: "Sync now",
        group: "Go to",
        icon: <RefreshCw size={ICON} />,
        run: () => void api.pushSync().then(() => toast("Sync complete")).catch((e) => toast(formatError(e), "error")),
      },
      { key: "g:fetchall", title: "Fetch all repositories", subtitle: "Updates every tracked git project", group: "Git", icon: <FolderSearch size={ICON} />, run: () => void fetchAll() },
    );
    return items;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [bootstrap.categories, onNavigate, onCreate, toast]);

  const projectItem = (pc: ProjectWithCategory, group: string): Item => ({
    key: `p:${pc.project.id}`,
    title: pc.project.name,
    subtitle: [pc.category.name, pc.project.git?.branch, pc.project.description].filter(Boolean).join(" · "),
    group,
    icon: <ExternalLink size={ICON} />,
    run: () => openProject(pc),
    drill: () => {
      setScope(pc);
      setQ("");
      setActive(0);
    },
  });

  const query = q.trim().toLowerCase();

  // Full-text hits for other entity types (TODOs, files, …).
  useEffect(() => {
    if (!open || scope || mode === "projects" || query.length < 2) {
      setRemote([]);
      return;
    }
    let cancelled = false;
    const id = window.setTimeout(() => {
      api
        .search(q)
        .then((hits) => !cancelled && setRemote(hits))
        .catch(() => !cancelled && setRemote([]));
    }, 120);
    return () => {
      cancelled = true;
      window.clearTimeout(id);
    };
  }, [q, query.length, open, scope, mode]);

  const items = useMemo<Item[]>(() => {
    if (scope) {
      const acts = projectActions(scope, currentProject?.project.id === scope.project.id, commands);
      return query ? acts.filter((a) => matches(a.title, query)) : acts;
    }
    const byRecency = [...allProjects].sort((a, b) => {
      const ra = recentIds.indexOf(a.project.id);
      const rb = recentIds.indexOf(b.project.id);
      return (ra === -1 ? 99 : ra) - (rb === -1 ? 99 : rb);
    });
    if (mode === "projects") {
      const list = query
        ? byRecency.filter((pc) => matches(pc.project.name, query) || pc.project.tags.some((t) => matches(t.name, query)))
        : byRecency;
      return list.map((pc, i) => projectItem(pc, recentIds.includes(pc.project.id) && !query ? "Recent" : i === 0 ? "Projects" : "Projects"));
    }
    if (!query) {
      const out: Item[] = [];
      if (currentProject) out.push(...projectActions(currentProject, true, commands));
      byRecency
        .filter((pc) => recentIds.includes(pc.project.id) && pc.project.id !== currentProject?.project.id)
        .slice(0, 5)
        .forEach((pc) => out.push(projectItem(pc, "Recent")));
      out.push(...globalActions);
      return out;
    }
    const out: Item[] = [];
    byRecency
      .filter((pc) => matches(pc.project.name, query) || pc.project.tags.some((t) => matches(t.name, query)))
      .slice(0, 8)
      .forEach((pc) => out.push(projectItem(pc, "Projects")));
    if (currentProject) {
      out.push(...projectActions(currentProject, true, commands).filter((a) => matches(a.title, query) || matches(a.subtitle ?? "", query)));
    }
    out.push(...globalActions.filter((a) => matches(a.title, query) || matches(a.subtitle ?? "", query)));
    const knownProjectIds = new Set(allProjects.map((pc) => pc.project.id));
    remote
      .filter((h) => !(h.entityType === "project" && knownProjectIds.has(h.entityId)))
      .forEach((hit, i) =>
        out.push({
          key: `r:${hit.entityType}:${hit.entityId}:${i}`,
          title: hit.title,
          subtitle: hit.subtitle || hit.entityType,
          group: hit.entityType === "todo" ? "TODOs" : "Results",
          run: () => {
            const categoryId = hit.categoryId ?? allProjects.find((pc) => pc.project.id === hit.projectId)?.category.id;
            const projectId = hit.entityType === "project" ? hit.entityId : hit.projectId;
            if (categoryId && projectId) onNavigate({ name: "workspace", categoryId, page: "items", projectId });
          },
        }),
      );
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scope, query, mode, allProjects, recentIds, currentProject, globalActions, remote, runningHere, commands]);

  useEffect(() => {
    setActive((a) => Math.min(a, Math.max(0, items.length - 1)));
  }, [items.length]);

  useEffect(() => {
    listRef.current?.querySelector<HTMLElement>(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
  }, [active]);

  const runItem = (item: Item | undefined) => {
    if (!item) return;
    close();
    void item.run();
  };

  const askAi = async () => {
    setAiBusy(true);
    try {
      const res = await api.aiInterpretSearch(q);
      const ids = res.interpretation?.projectIds ?? [];
      const found = ids
        .map((id) => allProjects.find((pc) => pc.project.id === id))
        .filter((pc): pc is ProjectWithCategory => Boolean(pc));
      setAiNote(res.interpretation?.explanation ?? (found.length ? null : "No matches found."));
      if (found.length === 1) {
        close();
        openProject(found[0]);
      } else if (found.length > 1) {
        setScope(null);
        setRemote(
          found.map((pc) => ({
            title: pc.project.name,
            subtitle: res.interpretation?.explanation || pc.category.name,
            entityType: "project",
            entityId: pc.project.id,
            categoryId: pc.category.id,
            projectId: pc.project.id,
          })),
        );
      }
    } catch (e) {
      setAiNote(formatError(e));
    } finally {
      setAiBusy(false);
    }
  };

  if (!open) return null;

  const sentenceLike = query.length >= 12 && query.includes(" ");
  let lastGroup = "";
  return (
    <Overlay onDismiss={close} label="Command palette">
      <div className="palette menu-pop">
        <div className="palette-input-row">
          {scope ? (
            <button type="button" className="palette-scope" onClick={() => { setScope(null); setQ(""); }} title="Back (Backspace)">
              {scope.project.name}
            </button>
          ) : null}
          <input
            autoFocus
            aria-label="Search"
            placeholder={
              scope
                ? "Choose an action…"
                : mode === "projects"
                  ? "Jump to project…"
                  : "Search projects, TODOs, actions…"
            }
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setActive(0);
              setAiNote(null);
            }}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                setActive((a) => Math.min(items.length - 1, a + 1));
              } else if (e.key === "ArrowUp") {
                e.preventDefault();
                setActive((a) => Math.max(0, a - 1));
              } else if (e.key === "Tab" || (e.key === "ArrowRight" && e.currentTarget.selectionStart === q.length)) {
                const drill = items[active]?.drill;
                if (drill) {
                  e.preventDefault();
                  drill();
                } else if (e.key === "Tab") {
                  e.preventDefault();
                }
              } else if (e.key === "Backspace" && q === "" && scope) {
                e.preventDefault();
                setScope(null);
              } else if (e.key === "Enter") {
                e.preventDefault();
                if (items.length === 0 && sentenceLike && !aiBusy) void askAi();
                else runItem(items[active]);
              }
            }}
          />
        </div>
        {aiNote ? <div className="palette-note muted">{aiNote}</div> : null}
        <div className="scroll palette-list" ref={listRef} role="listbox">
          {items.map((item, i) => {
            const heading = item.group !== lastGroup ? item.group : null;
            lastGroup = item.group;
            return (
              <div key={item.key}>
                {heading ? <div className="palette-group">{heading}</div> : null}
                <button
                  type="button"
                  role="option"
                  aria-selected={i === active}
                  data-index={i}
                  className={`palette-item ${i === active ? "active" : ""}`}
                  onMouseMove={() => i !== active && setActive(i)}
                  onClick={() => runItem(item)}
                >
                  {item.icon ? <span className="palette-icon">{item.icon}</span> : null}
                  <span className="palette-text">
                    <span className="palette-title">{item.title}</span>
                    {item.subtitle ? <span className="palette-sub muted">{item.subtitle}</span> : null}
                  </span>
                  {item.hint ? <kbd className="kbd">{item.hint}</kbd> : null}
                  {item.drill && i === active ? <kbd className="kbd">Tab</kbd> : null}
                </button>
              </div>
            );
          })}
          {items.length === 0 ? (
            <div className="palette-empty muted">
              {sentenceLike ? (aiBusy ? "Asking AI…" : "No matches. Press Enter to ask AI.") : "No matches."}
            </div>
          ) : null}
        </div>
        <div className="palette-footer muted">
          <span><kbd className="kbd">↑↓</kbd> navigate</span>
          <span><kbd className="kbd"><CornerDownLeft size={10} /></kbd> run</span>
          <span><kbd className="kbd">Tab</kbd> project actions</span>
          <span><kbd className="kbd">Esc</kbd> close</span>
        </div>
      </div>
    </Overlay>
  );
}
