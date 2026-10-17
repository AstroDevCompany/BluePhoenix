import { useState, type ReactElement } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Check,
  Code2,
  Download,
  FolderOpen,
  GitBranch,
  Loader2,
  Play,
  RefreshCw,
  Terminal as TerminalIcon,
  Upload,
  Archive,
} from "lucide-react";
import type { CommandDto, GitAction, ProjectCard } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { useEditors, useTerminalOptions } from "../../lib/queries";
import { hasLocalFolder, openInEditor, runGit } from "../../lib/projectActions";
import { openEmbeddedTerminal, runInEmbeddedTerminal, runInSystemTerminal } from "../../lib/terminalActions";
import { SplitButton } from "../../components/ui/SplitButton";
import { MenuItem, MenuLabel, MenuSeparator } from "../../components/ui/Menu";
import { useShell } from "../../lib/shellContext";
import { useUi } from "../../stores/ui";
import { formatCombo } from "../../lib/shortcuts";

const ICON = 14;

/** Opens the folder in the preferred editor; the caret picks another installed editor (and remembers it). */
export function EditorButton({ project, compact }: { project: ProjectCard; compact?: boolean }) {
  const { bootstrap, reload } = useShell();
  const { data: editors = [] } = useEditors();
  const preferred = bootstrap.settings.preferredEditor;
  const current = editors.find((e) => e.id === preferred) ?? editors[0];
  const local = hasLocalFolder(project);

  const choose = async (id: string) => {
    await openInEditor(project, id);
    if (id !== preferred) {
      await api.saveSettings({ ...bootstrap.settings, preferredEditor: id }).then(reload).catch(() => undefined);
    }
  };

  return (
    <SplitButton
      compact={compact}
      icon={<Code2 size={ICON} />}
      label={compact ? undefined : "Editor"}
      title={current ? `Open in ${current.name} (${formatCombo("mod+e")})` : "Open in editor"}
      disabled={!local}
      onClick={() => void openInEditor(project, current?.id)}
      menuLabel="Choose editor"
    >
      <MenuLabel>Open in</MenuLabel>
      {editors.length === 0 ? <MenuItem disabled>No supported editor found</MenuItem> : null}
      {editors.map((e) => (
        <MenuItem key={e.id} icon={e.id === current?.id ? <Check size={ICON} /> : undefined} onSelect={() => void choose(e.id)}>
          {e.name}
        </MenuItem>
      ))}
      <MenuSeparator />
      <MenuItem icon={<FolderOpen size={ICON} />} onSelect={() => local && void api.revealPath(project.localPath)}>
        Reveal folder
      </MenuItem>
    </SplitButton>
  );
}

/** Embedded terminal by default; the caret offers the system terminal apps. */
export function TerminalButton({ project, compact }: { project: ProjectCard; compact?: boolean }) {
  const { data: terminals = [] } = useTerminalOptions();
  const toast = useUi((s) => s.showToast);
  const local = hasLocalFolder(project);
  return (
    <SplitButton
      compact={compact}
      icon={<TerminalIcon size={ICON} />}
      label={compact ? undefined : "Terminal"}
      title={`Terminal in ${project.name} (${formatCombo("mod+j")})`}
      disabled={!local}
      onClick={() => void openEmbeddedTerminal(project)}
      menuLabel="Terminal options"
    >
      <MenuItem icon={<TerminalIcon size={ICON} />} onSelect={() => void openEmbeddedTerminal(project)}>
        Embedded terminal
      </MenuItem>
      <MenuSeparator />
      <MenuLabel>System terminal</MenuLabel>
      {terminals.map((t) => (
        <MenuItem
          key={t.id}
          onSelect={() =>
            local && void api.openTerminal(project.localPath, { terminal: t.id }).catch((e) => toast(formatError(e), "error"))
          }
        >
          {t.name}
        </MenuItem>
      ))}
    </SplitButton>
  );
}

/** Runs a command in the embedded terminal; the caret chooses the system terminal or another command. */
export function RunButton({
  project,
  command,
  primary = true,
}: {
  project: ProjectCard;
  command: Pick<CommandDto, "id" | "name" | "command" | "dangerous">;
  primary?: boolean;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  const others = useQuery({
    queryKey: ["commands", project.id],
    queryFn: () => api.listCommands(project.id),
    enabled: menuOpen,
  });
  const local = hasLocalFolder(project);
  return (
    <SplitButton
      primary={primary}
      icon={<Play size={ICON} />}
      label={command.name}
      title={`Run \`${command.command}\` in the embedded terminal`}
      disabled={!local}
      onClick={() => void runInEmbeddedTerminal(project, command)}
      onMenuOpenChange={setMenuOpen}
      menuLabel="Run options"
    >
      <MenuItem icon={<TerminalIcon size={ICON} />} onSelect={() => void runInEmbeddedTerminal(project, command)}>
        Run in embedded terminal
      </MenuItem>
      <MenuItem icon={<TerminalIcon size={ICON} />} onSelect={() => void runInSystemTerminal(project, command)}>
        Run in system terminal
      </MenuItem>
      {(others.data ?? []).filter((c) => c.id !== command.id).length > 0 ? (
        <>
          <MenuSeparator />
          <MenuLabel>Other commands</MenuLabel>
          {(others.data ?? [])
            .filter((c) => c.id !== command.id)
            .map((c) => (
              <MenuItem key={c.id} icon={<Play size={ICON} />} onSelect={() => void runInEmbeddedTerminal(project, c)}>
                {c.name}
              </MenuItem>
            ))}
        </>
      ) : null}
    </SplitButton>
  );
}

/** The git action that matters most right now (Pull if behind, Push if ahead, else Fetch), plus the rest. */
export function GitButton({ project }: { project: ProjectCard }) {
  const [busy, setBusy] = useState<GitAction | null>(null);
  const git = project.git;
  const local = hasLocalFolder(project);
  const ready = Boolean(git && git.isRepo && !git.pending);
  const primaryAction: GitAction = ready && git!.behind > 0 ? "pull" : ready && git!.ahead > 0 ? "push" : "fetch";
  const labels: Record<string, string> = { pull: "Pull", push: "Push", fetch: "Fetch" };
  const icons: Record<string, ReactElement> = {
    pull: <Download size={ICON} />,
    push: <Upload size={ICON} />,
    fetch: <RefreshCw size={ICON} />,
  };
  const run = async (action: GitAction) => {
    if (busy) return;
    setBusy(action);
    try {
      await runGit(project, action);
    } finally {
      setBusy(null);
    }
  };
  const count = primaryAction === "pull" ? git?.behind : primaryAction === "push" ? git?.ahead : 0;
  return (
    <SplitButton
      icon={busy ? <Loader2 size={ICON} className="spin" /> : icons[primaryAction]}
      label={`${labels[primaryAction]}${count ? ` ${count}` : ""}`}
      title={`git ${primaryAction} · ${project.name}`}
      disabled={!local || !ready || busy !== null}
      onClick={() => void run(primaryAction)}
      menuLabel="Git actions"
    >
      <MenuItem icon={<RefreshCw size={ICON} />} onSelect={() => void run("fetch")}>Fetch</MenuItem>
      <MenuItem icon={<Download size={ICON} />} onSelect={() => void run("pull")}>
        Pull{git && git.behind > 0 ? ` (${git.behind} behind)` : ""}
      </MenuItem>
      <MenuItem icon={<Upload size={ICON} />} onSelect={() => void run("push")}>
        Push{git && git.ahead > 0 ? ` (${git.ahead} ahead)` : ""}
      </MenuItem>
      <MenuSeparator />
      <MenuItem icon={<Archive size={ICON} />} disabled={!git?.dirty} onSelect={() => void run("stash")}>
        Stash changes
      </MenuItem>
      <MenuItem icon={<GitBranch size={ICON} />} disabled={!git || git.stash === 0} onSelect={() => void run("stash_pop")}>
        Apply last stash{git && git.stash > 0 ? ` (${git.stash})` : ""}
      </MenuItem>
    </SplitButton>
  );
}
