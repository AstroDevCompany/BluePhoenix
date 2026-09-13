export type WorkspacePage = "overview" | "items" | "todos" | "commands" | "exams";

const WORKSPACE_PAGES: WorkspacePage[] = ["overview", "items", "todos", "commands", "exams"];

export type Route =
  | { name: "onboarding" }
  | { name: "workspace"; categoryId: string; page: WorkspacePage; projectId?: string }
  | { name: "achievements" }
  | { name: "settings"; tab: string };

export function pathFor(route: Route) {
  if (route.name === "onboarding") return "/onboarding";
  if (route.name === "achievements") return "/achievements";
  if (route.name === "settings") return `/settings/${route.tab}`;
  const base = `/workspace/${route.categoryId}/${route.page}`;
  return route.projectId ? `${base}/${route.projectId}` : base;
}

export function parsePath(pathname: string, fallbackCategory: string): Route {
  const parts = pathname.split("/").filter(Boolean);
  if (parts[0] === "onboarding") return { name: "onboarding" };
  if (parts[0] === "achievements") return { name: "achievements" };
  if (parts[0] === "settings") return { name: "settings", tab: parts[1] ?? "appearance" };
  if (parts[0] === "workspace" && parts[1]) {
    const raw = parts[2] ?? "overview";
    const page = WORKSPACE_PAGES.includes(raw as WorkspacePage) ? (raw as WorkspacePage) : "overview";
    return { name: "workspace", categoryId: parts[1], page, projectId: parts[3] };
  }
  return { name: "workspace", categoryId: fallbackCategory, page: "overview" };
}

export type FabIntent = "project" | "todo" | "exam" | "command";

export function fabIntentFor(route: Route): FabIntent | null {
  if (route.name !== "workspace") return null;
  if (route.projectId) return null;
  if (route.page === "todos") return "todo";
  if (route.page === "exams") return "exam";
  if (route.page === "commands") return "command";
  return "project";
}

export function fabLabelFor(intent: FabIntent, itemSingular: string) {
  if (intent === "todo") return "New TODO";
  if (intent === "exam") return "New exam";
  if (intent === "command") return "New command";
  return `New ${itemSingular}`;
}
