export type WorkspacePage = "overview" | "items" | "todos" | "commands" | "exams";

export type Route =
  | { name: "onboarding" }
  | { name: "workspace"; categoryId: string; page: WorkspacePage; projectId?: string }
  | { name: "achievements" }
  | { name: "activity" }
  | { name: "settings"; tab: string };

export function pathFor(route: Route) {
  if (route.name === "onboarding") return "/onboarding";
  if (route.name === "achievements") return "/achievements";
  if (route.name === "activity") return "/activity";
  if (route.name === "settings") return `/settings/${route.tab}`;
  const base = `/workspace/${route.categoryId}/${route.page}`;
  return route.projectId ? `${base}/${route.projectId}` : base;
}

export function parsePath(pathname: string, fallbackCategory: string): Route {
  const parts = pathname.split("/").filter(Boolean);
  if (parts[0] === "onboarding") return { name: "onboarding" };
  if (parts[0] === "achievements") return { name: "achievements" };
  if (parts[0] === "activity") return { name: "activity" };
  if (parts[0] === "settings") return { name: "settings", tab: parts[1] ?? "appearance" };
  if (parts[0] === "workspace" && parts[1]) {
    const page = (parts[2] as WorkspacePage) || "overview";
    return { name: "workspace", categoryId: parts[1], page, projectId: parts[3] };
  }
  return { name: "workspace", categoryId: fallbackCategory, page: "overview" };
}
