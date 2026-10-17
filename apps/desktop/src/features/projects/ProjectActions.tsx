import { useState, type ReactElement, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Archive,
  Check,
  Code2,
  Download,
  FolderOpen,
  Github,
  Globe,
  Loader2,
  MoreHorizontal,
  Play,
  RefreshCw,
  Terminal as TerminalIcon,
  Upload,
} from "lucide-react";
import type { CommandDto, GitAction, ProjectCard } from "../../lib/types";
import { api, formatError } from "../../lib/ipc";
import { useEditors, useTerminalOptions } from "../../lib/queries";
import { hasLocalFolder, openInEditor, runGit } from "../../lib/projectActions";
import { openEmbeddedTerminal, runInEmbeddedTerminal, runInSystemTerminal } from "../../lib/terminalActions";
import { SplitButton } from "../../components/ui/SplitButton";
import { Menu, MenuContent, MenuItem, MenuLabel, MenuSeparator, MenuTrigger } from "../../components/ui/Menu";
import { useShell } from "../../lib/shellContext";
import { useUi } from "../../stores/ui";
import { formatCombo } from "../../lib/shortcuts";

const ICON = 14;

/** Opens the folder in the preferred editor. Other editors live in the project's "more" menu. */
export function EditorButton({ project, small, quiet }: { project: ProjectCard; small?: boolean; quiet?: boolean }) {
  const { bootstrap } = useShell();
  const { data: editors = [] } = useEditors();
  const current = editors.find((e) => e.id === bootstrap.settings.preferredEditor) ?? editors[0];
  return (
    <button
      type="button"
      className={`btn ${small ? "sm" : ""} ${quiet ? "quiet" : ""}`}
      disabled={!hasLocalFolder(project)}
      title={current ? `Open in ${current.name} (${formatCombo("mod+e")})` : "Open in editor"}
      onClick={() => void openInEditor(project, current?.id)}
    >
      <Code2 size={ICON} /> Editor
    </button>
  );
}

export function TerminalButton({ project, small, quiet }: { project: ProjectCard; small?: boolean; quiet?: boolean }) {
  return (
    <button
      type="button"
      className={`btn ${small ? "sm" : ""} ${quiet ? "quiet" : ""}`}
      disabled={!hasLocalFolder(project)}
      title={`Terminal in ${project.name} (${formatCombo("mod+j")})`}
      onClick={() => void openEmbeddedTerminal(project)}
    >
      <TerminalIcon size={ICON} /> Terminal
    </button>
  );
}

/** Runs a command in the embedded terminal; the caret chooses the system terminal or another command. */
export function RunButton({
  project,
  command,
  variant = "default",
  small,
}: {
  project: ProjectCard;
  command: Pick<CommandDto, "id" | "name" | "command" | "dangerous">;
  variant?: "default" | "primary" | "quiet";
  small?: boolean;
}) {
  const [menuOpen, setMenuOpen] = useState(false);
  const others = useQuery({
    queryKey: ["commands", project.id],
    queryFn: () => api.listCommands(project.id),
    enabled: menuOpen,
  });
  const extra = (others.data ?? []).filter((c) => c.id !== command.id);
  return (
    <SplitButton
      variant={variant}
      small={small}
      icon={<Play size={ICON} />}
      label={command.name}
      title={`Run \`${command.command}\` in the embedded terminal`}
      disabled={!hasLocalFolder(project)}
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
      {extra.length > 0 ? (
        <>
          <MenuSeparator />
          <MenuLabel>Other commands</MenuLabel>
          {extra.map((c) => (
            <MenuItem key={c.id} icon={<Play size={ICON} />} onSelect={() => void runInEmbeddedTerminal(project, c)}>
              {c.name}
            </MenuItem>
          ))}
        </>
      ) : null}
    </SplitButton>
  );
}

/** The git action that matters right now (Pull if behind, Push if ahead, else Fetch). */
export function GitButton({
  project,
  onlyWhenNeeded,
  small,
}: {
  project: ProjectCard;
  /** Render nothing unless there is something to pull, push or commit. */
  onlyWhenNeeded?: boolean;
  small?: boolean;
}) {
  const [busy, setBusy] = useState<GitAction | null>(null);
  const git = project.git;
  const ready = Boolean(git && git.isRepo && !git.pending);
  if (onlyWhenNeeded && !(ready && (git!.behind > 0 || git!.ahead > 0))) return null;
  const action: GitAction = ready && git!.behind > 0 ? "pull" : ready && git!.ahead > 0 ? "push" : "fetch";
  const count = action === "pull" ? git?.behind : action === "push" ? git?.ahead : 0;
  const labels: Record<string, string> = { pull: "Pull", push: "Push", fetch: "Fetch" };
  const icons: Record<string, ReactElement> = {
    pull: <Download size={ICON} />,
    push: <Upload size={ICON} />,
    fetch: <RefreshCw size={ICON} />,
  };
  const run = async () => {
    if (busy) return;
    setBusy(action);
    try {
      await runGit(project, action);
    } finally {
      setBusy(null);
    }
  };
  return (
    <button
      type="button"
      className={`btn ${small ? "sm" : ""}`}
      disabled={!hasLocalFolder(project) || !ready || busy !== null}
      title={`git ${action} · ${project.name}`}
      onClick={() => void run()}
    >
      {busy ? <Loader2 size={ICON} className="spin" /> : icons[action]}
      {labels[action]}
      {count ? ` ${count}` : ""}
    </button>
  );
}

/**
 * Everything that is not the main action: other editors and terminals, git extras, links.
 * `children` adds page-specific items (for example Remove on a card).
 */
export function ProjectMoreMenu({ project, extra }: { project: ProjectCard; extra?: ReactNode }) {
  const { bootstrap, reload } = useShell();
  const { data: editors = [] } = useEditors();
  const { data: terminals = [] } = useTerminalOptions();
  const toast = useUi((s) => s.showToast);
  const local = hasLocalFolder(project);
  const git = project.git;
  const gitReady = Boolean(git && git.isRepo && !git.pending);
  const preferred = bootstrap.settings.preferredEditor || editors[0]?.id;

  const openWith = async (id: string) => {
    await openInEditor(project, id);
    if (id !== bootstrap.settings.preferredEditor) {
      await api.saveSettings({ ...bootstrap.settings, preferredEditor: id }).then(reload).catch(() => undefined);
    }
  };

  return (
    <Menu>
      <MenuTrigger asChild>
        <button type="button" className="btn icon quiet" aria-label={`More actions for ${project.name}`}>
          <MoreHorizontal size={16} />
        </button>
      </MenuTrigger>
      <MenuContent align="end">
        {local && editors.length > 0 ? (
          <>
            <MenuLabel>Open in</MenuLabel>
            {editors.map((e) => (
              <MenuItem key={e.id} icon={e.id === preferred ? <Check size={ICON} /> : <span />} onSelect={() => void openWith(e.id)}>
                {e.name}
              </MenuItem>
            ))}
            <MenuSeparator />
            <MenuLabel>Terminal</MenuLabel>
            {terminals.map((t) => (
              <MenuItem
                key={t.id}
                icon={<TerminalIcon size={ICON} />}
                onSelect={() => void api.openTerminal(project.localPath, { terminal: t.id }).catch((e) => toast(formatError(e), "error"))}
              >
                {t.name}
              </MenuItem>
            ))}
            <MenuSeparator />
          </>
        ) : null}
        {local ? (
          <MenuItem icon={<FolderOpen size={ICON} />} onSelect={() => void api.revealPath(project.localPath)}>
            Reveal folder
          </MenuItem>
        ) : null}
        {gitReady ? (
          <>
            <MenuItem icon={<RefreshCw size={ICON} />} onSelect={() => void runGit(project, "fetch")}>Fetch</MenuItem>
            <MenuItem icon={<Download size={ICON} />} onSelect={() => void runGit(project, "pull")}>Pull</MenuItem>
            <MenuItem icon={<Upload size={ICON} />} onSelect={() => void runGit(project, "push")}>Push</MenuItem>
            <MenuItem icon={<Archive size={ICON} />} disabled={!git!.dirty} onSelect={() => void runGit(project, "stash")}>
              Stash changes
            </MenuItem>
            <MenuItem icon={<Archive size={ICON} />} disabled={git!.stash === 0} onSelect={() => void runGit(project, "stash_pop")}>
              Apply last stash
            </MenuItem>
          </>
        ) : null}
        {project.githubUrl ? (
          <MenuItem icon={<Github size={ICON} />} onSelect={() => void api.openUrl(project.githubUrl!)}>GitHub</MenuItem>
        ) : null}
        {project.websiteUrl ? (
          <MenuItem icon={<Globe size={ICON} />} onSelect={() => void api.openUrl(project.websiteUrl!)}>Website</MenuItem>
        ) : null}
        {extra}
      </MenuContent>
    </Menu>
  );
}
