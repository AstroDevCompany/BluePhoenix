import {
  RouterProvider,
  createRootRoute,
  createRoute,
  createRouter,
  redirect,
  useNavigate,
  useParams,
} from "@tanstack/react-router";
import type { WorkspacePage } from "./components/router";
import { WorkspaceHome } from "./pages/WorkspacePages";
import { SettingsPage } from "./pages/SettingsPage";
import { AchievementsPage } from "./features/achievements/AchievementsPage";
import { ActivityPage } from "./features/activity/ActivityPage";
import { RootChrome, useShell } from "./pages/Shell";

const rootRoute = createRootRoute({
  component: RootChrome,
});

const onboardingRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/onboarding",
  component: function OnboardingPlaceholder() {
    return null;
  },
});

const workspaceRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/workspace/$categoryId/$page",
  component: function WorkspaceRoute() {
    const { categoryId, page } = useParams({ from: "/workspace/$categoryId/$page" });
    const { bootstrap, refreshKey, bump } = useShell();
    const navigate = useNavigate();
    const category = bootstrap.categories.find((c) => c.id === categoryId && c.enabled)
      ?? bootstrap.categories.find((c) => c.enabled);
    if (!category) return <div className="muted">Workspace unavailable.</div>;
    return (
      <WorkspaceHome
        category={category}
        page={(page as WorkspacePage) || "overview"}
        onOpenProject={(id) => {
          void navigate({
            to: "/workspace/$categoryId/items/$projectId",
            params: { categoryId: category.id, projectId: id },
          });
        }}
        onCreate={bump}
        refreshKey={refreshKey}
      />
    );
  },
});

const projectRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/workspace/$categoryId/items/$projectId",
  component: function ProjectRoute() {
    const { categoryId, projectId } = useParams({ from: "/workspace/$categoryId/items/$projectId" });
    const { bootstrap, refreshKey, bump } = useShell();
    const navigate = useNavigate();
    const category = bootstrap.categories.find((c) => c.id === categoryId && c.enabled)
      ?? bootstrap.categories.find((c) => c.enabled);
    if (!category) return <div className="muted">Workspace unavailable.</div>;
    return (
      <WorkspaceHome
        category={category}
        page="items"
        projectId={projectId}
        onOpenProject={(id) => {
          void navigate({
            to: "/workspace/$categoryId/items/$projectId",
            params: { categoryId: category.id, projectId: id },
          });
        }}
        onCreate={bump}
        refreshKey={refreshKey}
      />
    );
  },
});

const achievementsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/achievements",
  component: AchievementsPage,
});

const activityRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/activity",
  component: ActivityPage,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings/$tab",
  component: function SettingsRoute() {
    const { tab } = useParams({ from: "/settings/$tab" });
    const { bootstrap, reload } = useShell();
    const navigate = useNavigate();
    return (
      <SettingsPage
        bootstrap={bootstrap}
        tab={tab}
        onTab={(next) => void navigate({ to: "/settings/$tab", params: { tab: next } })}
        onReload={reload}
      />
    );
  },
});

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/onboarding" });
  },
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  onboardingRoute,
  projectRoute,
  workspaceRoute,
  achievementsRoute,
  activityRoute,
  settingsRoute,
]);

export const router = createRouter({
  routeTree,
  defaultViewTransition: true,
});

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

export function AppRouter() {
  return <RouterProvider router={router} />;
}
