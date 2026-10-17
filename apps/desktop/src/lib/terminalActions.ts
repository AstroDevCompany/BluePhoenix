import { listen } from "@tauri-apps/api/event";
import { api, formatError } from "./ipc";
import type { CommandDto, ProjectCard, TerminalDataEvent, TerminalExitEvent, TerminalSession } from "./types";
import { appendOutput, forgetBuffer, setBuffer, useTerminals } from "../stores/terminals";
import { useUi } from "../stores/ui";
import { hasLocalFolder } from "./projectActions";

const toast = (message: string, tone?: "ok" | "error") => useUi.getState().showToast(message, tone);

/** Subscribes to backend terminal events and restores sessions that outlived a UI reload. */
export function initTerminals(): () => void {
  let disposed = false;
  const unlisteners: (() => void)[] = [];
  const track = (p: Promise<() => void>) =>
    void p.then((fn) => (disposed ? fn() : unlisteners.push(fn)));

  track(listen<TerminalDataEvent>("term-data", ({ payload }) => appendOutput(payload.id, payload.data)));
  track(
    listen<TerminalExitEvent>("term-exit", ({ payload }) => {
      useTerminals.getState().markExit(payload.id, payload.code);
    }),
  );

  void api
    .termList()
    .then(async (sessions) => {
      for (const session of sessions) {
        if (useTerminals.getState().sessions.some((s) => s.id === session.id)) continue;
        setBuffer(session.id, await api.termBacklog(session.id).catch(() => ""));
        useTerminals.getState().upsert(session);
      }
    })
    .catch(() => undefined);

  return () => {
    disposed = true;
    unlisteners.forEach((fn) => fn());
  };
}

function show(session: TerminalSession) {
  const store = useTerminals.getState();
  store.upsert(session);
  store.setActive(session.id);
  store.setDockOpen(true);
}

export async function openEmbeddedTerminal(project: ProjectCard) {
  if (!hasLocalFolder(project)) return toast("Set a local folder for this project first", "error");
  try {
    show(await api.termSpawn({ projectId: project.id }));
  } catch (e) {
    toast(formatError(e), "error");
  }
}

type Runnable = Pick<CommandDto, "command" | "dangerous"> & Partial<Pick<CommandDto, "id" | "name">>;

async function spawnCommand(project: ProjectCard, cmd: Runnable, confirmed?: boolean) {
  try {
    show(
      await api.termSpawn({
        projectId: project.id,
        ...(cmd.id ? { commandId: cmd.id } : { command: cmd.command }),
        confirmed,
      }),
    );
  } catch (e) {
    toast(formatError(e), "error");
  }
}

/** Runs a command in the embedded terminal dock, asking first when it looks destructive. */
export async function runInEmbeddedTerminal(project: ProjectCard, cmd: Runnable) {
  if (!hasLocalFolder(project)) return toast("Set a local folder for this project first", "error");
  if (cmd.dangerous) {
    useUi.getState().askConfirm("Run dangerous command?", cmd.command, () => void spawnCommand(project, cmd, true), true);
    return;
  }
  await spawnCommand(project, cmd);
}

/** Runs a command in the user's external terminal app. */
export async function runInSystemTerminal(project: ProjectCard, cmd: Runnable) {
  if (!hasLocalFolder(project)) return toast("Set a local folder for this project first", "error");
  const go = (confirmed?: boolean) =>
    api.openTerminal(project.localPath, { command: cmd.command, confirmed }).catch((e) => toast(formatError(e), "error"));
  if (cmd.dangerous) {
    useUi.getState().askConfirm("Run dangerous command?", cmd.command, () => void go(true), true);
    return;
  }
  await go();
}

export async function rerunSession(session: TerminalSession) {
  if (!session.command && !session.commandId) return;
  try {
    const next = await api.termSpawn({
      projectId: session.projectId ?? undefined,
      ...(session.commandId ? { commandId: session.commandId } : { command: session.command ?? undefined }),
      confirmed: true, // it already ran once, after any confirmation
    });
    await closeSession(session.id);
    show(next);
  } catch (e) {
    toast(formatError(e), "error");
  }
}

export async function killSession(id: string) {
  await api.termKill(id).catch((e) => toast(formatError(e), "error"));
}

export async function closeSession(id: string) {
  await api.termClose(id).catch(() => undefined);
  forgetBuffer(id);
  useTerminals.getState().remove(id);
}
