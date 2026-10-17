import { useEffect } from "react";
import { useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { api } from "./ipc";
import type { Category, GitStatusUpdate, ProjectCard } from "./types";

export const keys = {
  projects: (categoryId: string) => ["projects", categoryId] as const,
  project: (id: string) => ["project", id] as const,
  todos: (categoryId: string) => ["todos", categoryId] as const,
  exams: (categoryId: string) => ["exams", categoryId] as const,
  timer: ["running-timer"] as const,
  editors: ["launch", "editors"] as const,
  terminals: ["launch", "terminals"] as const,
};

export function useProjects(categoryId: string) {
  return useQuery({ queryKey: keys.projects(categoryId), queryFn: () => api.listProjects(categoryId) });
}

export function useProject(id: string | undefined) {
  return useQuery({
    queryKey: keys.project(id ?? ""),
    queryFn: () => api.getProject(id as string),
    enabled: Boolean(id),
  });
}

export type ProjectWithCategory = { project: ProjectCard; category: Category };

/** Projects of every enabled workspace, flattened (used by the palette and quick switcher). */
export function useAllProjects(categories: Category[]): ProjectWithCategory[] {
  const results = useQueries({
    queries: categories.map((c) => ({
      queryKey: keys.projects(c.id),
      queryFn: () => api.listProjects(c.id),
    })),
  });
  return results.flatMap((r, i) => (r.data ?? []).map((project) => ({ project, category: categories[i] })));
}

export function useRunningTimer() {
  return useQuery({
    queryKey: keys.timer,
    queryFn: () => api.runningTimer(),
    refetchInterval: 5_000,
  });
}

export function useEditors() {
  return useQuery({ queryKey: keys.editors, queryFn: () => api.listEditors(), staleTime: Infinity });
}

export function useTerminalOptions() {
  return useQuery({ queryKey: keys.terminals, queryFn: () => api.listTerminals(), staleTime: Infinity });
}

/** Pushes backend git status updates into every cached project list. */
export function useGitStatusEvents() {
  const client = useQueryClient();
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void listen<GitStatusUpdate>("git-status-updated", ({ payload }) => {
      const patch = (p: ProjectCard) => (p.id === payload.projectId ? { ...p, git: payload.git } : p);
      client.setQueriesData<ProjectCard[]>({ queryKey: ["projects"] }, (old) => old?.map(patch));
      client.setQueryData<ProjectCard>(keys.project(payload.projectId), (old) => (old ? patch(old) : old));
    }).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [client]);
}
