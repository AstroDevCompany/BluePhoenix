import { createContext, useContext, useState } from "react";
import { Outlet, useNavigate, useRouterState } from "@tanstack/react-router";
import {
  Activity,
  Award,
  BookOpen,
  Code2,
  LayoutDashboard,
  ListTodo,
  Plus,
  Settings as SettingsIcon,
  Terminal,
} from "lucide-react";
import type { Bootstrap, Category } from "../lib/types";
import { Titlebar } from "../components/Titlebar";
import { CommandPalette } from "../components/CommandPalette";
import { WorkspaceSwitcher } from "../components/WorkspaceSwitcher";
import { Onboarding } from "./Onboarding";
import { parsePath, pathFor, type Route, type WorkspacePage } from "../components/router";
import { useUi } from "../stores/ui";
import { LiveTimerBadge } from "../features/time/TimerControls";
import { CreateProjectDialog } from "../features/projects/CreateProjectDialog";
import { AiChatPanel } from "../features/ai/AiChatPanel";
import { AiSetupPopup } from "../features/ai/AiSetupPopup";

export type ShellApi = {
  bootstrap: Bootstrap;
  reload: () => void;
  refreshKey: number;
  bump: () => void;
};

const Ctx = createContext<ShellApi | null>(null);

export function useShell() {
  const value = useContext(Ctx);
  if (!value) throw new Error("Shell context missing");
  return value;
}

export function ShellProvider({ value, children }: { value: ShellApi; children: React.ReactNode }) {
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function RootChrome() {
  const { bootstrap, reload, bump } = useShell();
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const toast = useUi((s) => s.toast);
  const confirm = useUi((s) => s.confirm);
  const closeConfirm = useUi((s) => s.closeConfirm);
  const [creating, setCreating] = useState(false);
  const createOpen = useUi((s) => s.createOpen);
  const closeCreate = useUi((s) => s.closeCreate);
  const openCreate = useUi((s) => s.openCreate);
  const aiPanelOpen = useUi((s) => s.aiPanelOpen);

  const enabled = bootstrap.categories.filter((c) => c.enabled);
  const fallback = bootstrap.settings.defaultWorkspace ?? enabled[0]?.id ?? "";
  const route = parsePath(pathname, fallback);
  const currentCategory: Category | undefined =
    route.name === "workspace" ? enabled.find((c) => c.id === route.categoryId) ?? enabled[0] : enabled[0];

  const go = (next: Route) => {
    void navigate({ to: pathFor(next) as never });
  };

  if (!bootstrap.onboardingComplete || route.name === "onboarding") {
    return (
      <div className="shell onboard">
        <Titlebar extra={<LiveTimerBadge />} />
        <main className="main">
          <Onboarding
            bootstrap={bootstrap}
            onDone={(id) => {
              reload();
              go({ name: "workspace", categoryId: id, page: "overview" });
            }}
          />
        </main>
      </div>
    );
  }

  if (!currentCategory) {
    return <div className="main">Enable a workspace in Settings.</div>;
  }

  return (
    <div className={`shell ${aiPanelOpen ? "ai-open" : ""}`}>
      <Titlebar extra={<LiveTimerBadge />} />
      <a className="skip-link btn" href="#main">Skip to content</a>
      <aside className="sidebar">
        <WorkspaceSwitcher
          categories={enabled}
          current={currentCategory}
          onSelect={(id) => go({ name: "workspace", categoryId: id, page: "overview" })}
        />
        <div className="sidebar-nav" key={currentCategory.id}>
          <div className="muted" style={{ fontSize: 11, letterSpacing: "0.12em", marginTop: 12 }}>
            {currentCategory.name.toUpperCase()}
          </div>
          {currentCategory.nav.map((item) => {
            const page = (item.id === "overview" ? "overview" : item.id === "items" ? "items" : item.id) as WorkspacePage;
            const active = route.name === "workspace" && (route.page === page || (item.id === "items" && route.page === "items"));
            const Icon = item.id === "overview" ? LayoutDashboard : item.id === "todos" ? ListTodo : item.id === "commands" ? Terminal : item.id === "exams" ? BookOpen : Code2;
            return (
              <button key={item.id} className={`nav-btn ${active ? "active" : ""}`} type="button" onClick={() => go({ name: "workspace", categoryId: currentCategory.id, page })}>
                <Icon size={15} /> {item.label}
              </button>
            );
          })}
        </div>
        <div style={{ flex: 1 }} />
        <button className="nav-btn" type="button" onClick={() => go({ name: "achievements" })}><Award size={15} /> Achievements</button>
        <button className="nav-btn" type="button" onClick={() => go({ name: "activity" })}><Activity size={15} /> Activity</button>
        <button className="nav-btn" type="button" onClick={() => go({ name: "settings", tab: "appearance" })}><SettingsIcon size={15} /> Settings</button>
        <div className="row muted" style={{ fontSize: 11, marginTop: 8 }}>
          <span className={`sync-dot ${bootstrap.sync.status === "sync_error" ? "error" : bootstrap.sync.pending ? "pending" : ""}`} />
          {bootstrap.sync.label}
        </div>
      </aside>
      <main className="main" id="main">
        <div key={pathname} className="page-enter">
          <Outlet />
        </div>
      </main>
      <AiChatPanel projectId={route.name === "workspace" ? route.projectId : undefined} />
      <AiSetupPopup />
      <button className="btn primary fab-new" type="button" onClick={openCreate}>
        <Plus size={16} /> New
      </button>
      {creating || createOpen ? (
        <CreateProjectDialog
          bootstrap={bootstrap}
          initialCategory={currentCategory.id}
          onClose={() => { setCreating(false); closeCreate(); }}
          onCreated={(id, categoryId) => {
            setCreating(false);
            closeCreate();
            bump();
            go({ name: "workspace", categoryId, page: "items", projectId: id });
          }}
        />
      ) : null}
      <CommandPalette bootstrap={bootstrap} onNavigate={go} />
      {toast ? <div className="toast" role="status">{toast.message}</div> : null}
      {confirm ? (
        <div className="overlay" onClick={closeConfirm}>
          <div className="dialog" role="dialog" aria-modal="true" onClick={(e) => e.stopPropagation()}>
            <h2 className="h2">{confirm.title}</h2>
            <p className="muted">{confirm.body}</p>
            <div className="row" style={{ justifyContent: "flex-end" }}>
              <button className="btn" type="button" onClick={closeConfirm}>Cancel</button>
              <button className="btn danger" type="button" onClick={() => { confirm.onConfirm(); closeConfirm(); }}>Confirm</button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );
}
