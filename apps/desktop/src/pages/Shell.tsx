import { useCallback, useEffect, useMemo } from "react";
import { Outlet, useNavigate, useRouterState } from "@tanstack/react-router";
import {
  Award,
  BookOpen,
  Code2,
  LayoutDashboard,
  ListTodo,
  Plus,
  Settings as SettingsIcon,
  Terminal,
} from "lucide-react";
import type { Category } from "../lib/types";
import { useShell } from "../lib/shellContext";
import { Titlebar } from "../components/Titlebar";
import { CommandPalette } from "../components/CommandPalette";
import { ShortcutSheet } from "../components/ShortcutSheet";
import { ToastStack } from "../components/ToastStack";
import { WorkspaceSwitcher } from "../components/WorkspaceSwitcher";
import { SidebarProjects, pickSidebarProjects } from "../components/SidebarProjects";
import { TimerBadge, TimerWidget } from "../components/TimerWidget";
import { Onboarding } from "./Onboarding";
import { fabIntentFor, fabLabelFor, parsePath, pathFor, type Route, type WorkspacePage } from "../components/router";
import { useUi } from "../stores/ui";
import { CreateProjectDialog } from "../features/projects/CreateProjectDialog";
import { TabCreateDialog } from "../components/TabCreateDialog";
import { AiChatPanel } from "../features/ai/AiChatPanel";
import { AiSetupPopup } from "../features/ai/AiSetupPopup";
import { applyWorkspaceColor } from "../lib/theme";
import { Overlay } from "../components/ui/Overlay";
import { formatCombo, digitIndex, isTypingTarget, matchesCombo } from "../lib/shortcuts";
import {
  useAllProjects,
  useGitStatusEvents,
  useProject,
  useProjects,
  useRunningTimer,
  type ProjectWithCategory,
} from "../lib/queries";
import { openInEditor, startTimer, stopTimer } from "../lib/projectActions";
import { TerminalDock } from "../components/TerminalDock";
import { initTerminals, openEmbeddedTerminal } from "../lib/terminalActions";
import { useTerminals } from "../stores/terminals";

const NAV_ICONS: Record<string, typeof Code2> = {
  overview: LayoutDashboard,
  todos: ListTodo,
  commands: Terminal,
  exams: BookOpen,
};

export function RootChrome() {
  const { bootstrap, reload, bump } = useShell();
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const confirm = useUi((s) => s.confirm);
  const closeConfirm = useUi((s) => s.closeConfirm);
  const createOpen = useUi((s) => s.createOpen);
  const closeCreate = useUi((s) => s.closeCreate);
  const openCreate = useUi((s) => s.openCreate);
  const openPalette = useUi((s) => s.openPalette);
  const setShortcuts = useUi((s) => s.setShortcuts);
  const aiPanelOpen = useUi((s) => s.aiPanelOpen);
  const recentIds = useUi((s) => s.recentProjectIds);

  const enabled = useMemo(() => bootstrap.categories.filter((c) => c.enabled), [bootstrap.categories]);
  const fallback = bootstrap.settings.defaultWorkspace ?? enabled[0]?.id ?? "";
  const route = parsePath(pathname, fallback);
  const currentCategory: Category | undefined =
    route.name === "workspace" ? enabled.find((c) => c.id === route.categoryId) ?? enabled[0] : enabled[0];
  const routeProjectId = route.name === "workspace" ? route.projectId : undefined;

  useGitStatusEvents();
  useEffect(() => initTerminals(), []);
  const allProjects = useAllProjects(enabled);
  const { data: categoryProjects = [] } = useProjects(currentCategory?.id ?? "");
  const { data: timer } = useRunningTimer();
  const runningProjectId = timer && timer.isLocalDevice && !timer.entry.endedAt ? timer.entry.projectId : null;
  const { data: routeProjectCard } = useProject(routeProjectId);

  const currentProject: ProjectWithCategory | undefined = useMemo(() => {
    if (!routeProjectId) return undefined;
    const found = allProjects.find((pc) => pc.project.id === routeProjectId);
    if (found) return found;
    return routeProjectCard && currentCategory ? { project: routeProjectCard, category: currentCategory } : undefined;
  }, [routeProjectId, allProjects, routeProjectCard, currentCategory]);

  const sidebarProjects = useMemo(
    () => (currentCategory?.kind === "university" ? [] : pickSidebarProjects(categoryProjects, recentIds)),
    [categoryProjects, recentIds, currentCategory?.kind],
  );

  useEffect(() => {
    applyWorkspaceColor(currentCategory?.accent);
  }, [currentCategory?.accent]);

  useEffect(() => {
    if (routeProjectId) useUi.getState().touchProject(routeProjectId);
  }, [routeProjectId]);

  const go = useCallback(
    (next: Route) => {
      void navigate({ to: pathFor(next) as never });
    },
    [navigate],
  );

  const openProjectById = useCallback(
    (projectId: string) => {
      const pc = allProjects.find((p) => p.project.id === projectId);
      if (pc) go({ name: "workspace", categoryId: pc.category.id, page: "items", projectId });
    },
    [allProjects, go],
  );

  const intent = route.name === "workspace" ? fabIntentFor(route) : null;
  const createIntent = intent ?? "project";

  // One place for every global shortcut.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!bootstrap.onboardingComplete) return;
      const wsIdx = digitIndex(e, "mod");
      if (wsIdx !== null && enabled[wsIdx]) {
        e.preventDefault();
        go({ name: "workspace", categoryId: enabled[wsIdx].id, page: "overview" });
        return;
      }
      const jumpIdx = digitIndex(e, "alt");
      if (jumpIdx !== null && sidebarProjects[jumpIdx] && currentCategory) {
        e.preventDefault();
        go({ name: "workspace", categoryId: currentCategory.id, page: "items", projectId: sidebarProjects[jumpIdx].id });
        return;
      }
      if (matchesCombo(e, "mod+k")) {
        e.preventDefault();
        const st = useUi.getState();
        if (st.paletteOpen) st.setPalette(false);
        else openPalette("all");
      } else if (matchesCombo(e, "mod+p")) {
        e.preventDefault();
        openPalette("projects");
      } else if (matchesCombo(e, "mod+n")) {
        e.preventDefault();
        openCreate();
      } else if (matchesCombo(e, "mod+,")) {
        e.preventDefault();
        go({ name: "settings", tab: "appearance" });
      } else if (matchesCombo(e, "mod+shift+t")) {
        e.preventDefault();
        if (currentProject && runningProjectId !== currentProject.project.id) void startTimer(currentProject.project);
        else if (runningProjectId) void stopTimer();
      } else if (matchesCombo(e, "mod+e")) {
        if (currentProject) {
          e.preventDefault();
          void openInEditor(currentProject.project);
        }
      } else if (matchesCombo(e, "mod+j")) {
        e.preventDefault();
        const term = useTerminals.getState();
        if (term.sessions.length > 0) term.setDockOpen(!term.dockOpen);
        else if (currentProject) void openEmbeddedTerminal(currentProject.project);
        else useUi.getState().showToast("Open a project to start a terminal", "error");
      } else if (matchesCombo(e, "?") && !isTypingTarget(e.target)) {
        e.preventDefault();
        setShortcuts(true);
      } else if (
        e.key === "Escape" &&
        !e.defaultPrevented &&
        !isTypingTarget(e.target) &&
        routeProjectId &&
        currentCategory
      ) {
        go({ name: "workspace", categoryId: currentCategory.id, page: "items" });
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    bootstrap.onboardingComplete,
    enabled,
    sidebarProjects,
    currentCategory,
    currentProject,
    runningProjectId,
    routeProjectId,
    go,
    openPalette,
    openCreate,
    setShortcuts,
  ]);

  if (!bootstrap.onboardingComplete || route.name === "onboarding") {
    return (
      <div className="shell onboard">
        <Titlebar showSearch={false} />
        <main className="main">
          <Onboarding
            bootstrap={bootstrap}
            onDone={(id) => {
              reload();
              go({ name: "workspace", categoryId: id, page: "overview" });
            }}
          />
        </main>
        <ToastStack />
      </div>
    );
  }

  if (!currentCategory) {
    return <div className="main">Enable a workspace in Settings.</div>;
  }

  return (
    <div className={`shell ${aiPanelOpen ? "ai-open" : ""}`}>
      <Titlebar extra={<TimerBadge onOpenProject={openProjectById} />} />
      <a className="skip-link btn" href="#main">Skip to content</a>
      <aside className="sidebar">
        <WorkspaceSwitcher
          categories={enabled}
          current={currentCategory}
          onSelect={(id) => go({ name: "workspace", categoryId: id, page: "overview" })}
        />
        <button className="btn primary sidebar-new" type="button" onClick={openCreate}>
          <Plus size={15} />
          <span>{fabLabelFor(createIntent, currentCategory.terminology.itemSingular)}</span>
          <kbd className="kbd">{formatCombo("mod+n")}</kbd>
        </button>
        <div className="sidebar-nav">
          <div className="nav-section">{currentCategory.name.toUpperCase()}</div>
          {currentCategory.nav.filter((item) => item.id !== "activity").map((item) => {
            const page = item.id as WorkspacePage;
            const active = route.name === "workspace" && route.page === page;
            const Icon = NAV_ICONS[item.id] ?? Code2;
            return (
              <button key={item.id} className={`nav-btn ${active ? "active" : ""}`} type="button" onClick={() => go({ name: "workspace", categoryId: currentCategory.id, page })}>
                <Icon size={15} /> {item.label}
              </button>
            );
          })}
        </div>
        <SidebarProjects
          category={currentCategory}
          projects={sidebarProjects}
          activeId={routeProjectId}
          runningProjectId={runningProjectId}
          onOpen={(id) => go({ name: "workspace", categoryId: currentCategory.id, page: "items", projectId: id })}
          onAll={() => go({ name: "workspace", categoryId: currentCategory.id, page: "items" })}
        />
        <div className="sidebar-spacer" />
        <TimerWidget onOpenProject={openProjectById} />
        <button className="nav-btn" type="button" onClick={() => go({ name: "achievements" })}><Award size={15} /> Achievements</button>
        <button className="nav-btn" type="button" onClick={() => go({ name: "settings", tab: "appearance" })}><SettingsIcon size={15} /> Settings</button>
        <div className="sidebar-sync muted">
          <span className={`sync-dot ${bootstrap.sync.status === "sync_error" ? "error" : bootstrap.sync.pending ? "pending" : ""}`} />
          {bootstrap.sync.label}
        </div>
      </aside>
      <main className="main" id="main">
        <Outlet />
      </main>
      <TerminalDock currentProject={currentProject?.project} />
      <AiChatPanel projectId={routeProjectId} />
      <AiSetupPopup />
      {createOpen && createIntent === "project" ? (
        <CreateProjectDialog
          bootstrap={bootstrap}
          initialCategory={currentCategory.id}
          onClose={closeCreate}
          onCreated={(id, categoryId) => {
            closeCreate();
            bump();
            go({ name: "workspace", categoryId, page: "items", projectId: id });
          }}
        />
      ) : null}
      {createOpen && createIntent !== "project" ? (
        <TabCreateDialog
          kind={createIntent}
          category={currentCategory}
          onClose={closeCreate}
          onCreated={() => {
            closeCreate();
            bump();
          }}
        />
      ) : null}
      <CommandPalette
        bootstrap={bootstrap}
        allProjects={allProjects}
        currentProject={currentProject}
        onNavigate={go}
        onCreate={openCreate}
      />
      <ShortcutSheet />
      <ToastStack />
      {confirm ? (
        <Overlay onDismiss={closeConfirm} label={confirm.title}>
          <div className="dialog">
            <h2 className="h2">{confirm.title}</h2>
            <p className="muted">{confirm.body}</p>
            <div className="row actions-end">
              <button className="btn" type="button" onClick={closeConfirm}>Cancel</button>
              <button className={`btn ${confirm.danger ? "danger" : "primary"}`} type="button" onClick={() => { confirm.onConfirm(); closeConfirm(); }}>Confirm</button>
            </div>
          </div>
        </Overlay>
      ) : null}
    </div>
  );
}
