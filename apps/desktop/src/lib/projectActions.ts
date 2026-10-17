import { queryClient } from "./queryClient";
import { keys } from "./queries";
import { api, formatError } from "./ipc";
import type { GitAction, ProjectCard } from "./types";
import { useUi, type ToastOptions, type ToastTone } from "../stores/ui";

export type GitErrorKind = "conflict" | "auth" | "network" | "timeout" | "dirty" | "other";

/** Splits a typed backend git error (`conflict: …`) into its kind and readable message. */
export function parseGitError(raw: string): { kind: GitErrorKind; message: string } {
  const match = /^(conflict|auth|network|timeout|dirty):\s*(.*)$/s.exec(raw);
  if (match) return { kind: match[1] as GitErrorKind, message: match[2] };
  return { kind: "other", message: raw };
}

const toast = (message: string, tone?: ToastTone, options?: ToastOptions) =>
  useUi.getState().showToast(message, tone, options);

function fail(e: unknown) {
  useUi.getState().showToast(parseGitError(formatError(e)).message, "error");
}

export function hasLocalFolder(p: ProjectCard): p is ProjectCard & { localPath: string } {
  return Boolean(p.localPath && p.localPath.trim());
}

export async function openInEditor(p: ProjectCard, editor?: string) {
  if (!hasLocalFolder(p)) return toast("Set a local folder for this project first", "error");
  try {
    await api.openInEditor(p.localPath, editor);
  } catch (e) {
    fail(e);
  }
}

export async function openSystemTerminal(p: ProjectCard, command?: string) {
  if (!hasLocalFolder(p)) return toast("Set a local folder for this project first", "error");
  try {
    await api.openTerminal(p.localPath, { command });
  } catch (e) {
    fail(e);
  }
}

const GIT_DONE: Record<GitAction, string> = {
  fetch: "Fetched",
  pull: "Pulled",
  push: "Pushed",
  checkout: "Switched branch",
  stash: "Stashed changes",
  stash_pop: "Applied stash",
};

/** Runs a git action; the backend refreshes the cached status and emits an update event. */
export async function runGit(p: ProjectCard, action: GitAction, arg?: string) {
  if (!hasLocalFolder(p)) return toast("Set a local folder for this project first", "error");
  try {
    await api.gitRun(p.id, action, arg);
    toast(`${GIT_DONE[action]} · ${p.name}`);
  } catch (e) {
    fail(e);
  }
}

export async function fetchAll(projectIds?: string[]) {
  try {
    const results = await api.gitFetchAll(projectIds);
    const failed = results.filter((r) => !r.ok);
    if (failed.length === 0) toast(`Fetched ${results.length} ${results.length === 1 ? "repository" : "repositories"}`);
    else toast(`${failed.length} of ${results.length} fetches failed`, "error");
    return results;
  } catch (e) {
    fail(e);
    return [];
  }
}

export async function refreshTimer() {
  await queryClient.invalidateQueries({ queryKey: keys.timer });
}

/** Starts the timer on a project, stopping a timer running elsewhere first (one-click switch). */
export async function startTimer(p: ProjectCard) {
  try {
    const running = await api.runningTimer().catch(() => null);
    const localRunning = running?.isLocalDevice && !running.entry.endedAt ? running : null;
    if (localRunning?.entry.projectId === p.id) return toast(`Timer already running · ${p.name}`);
    if (localRunning) await api.stopTimer();
    await api.startTimer(p.id);
    toast(localRunning ? `Timer switched to ${p.name}` : `Timer started · ${p.name}`);
  } catch (e) {
    toast(formatError(e), "error");
  }
  await queryClient.invalidateQueries({ queryKey: ["projects"] });
  await refreshTimer();
}

export async function stopTimer() {
  try {
    await api.stopTimer();
    toast("Timer stopped");
  } catch (e) {
    toast(formatError(e), "error");
  }
  await queryClient.invalidateQueries({ queryKey: ["projects"] });
  await refreshTimer();
}
