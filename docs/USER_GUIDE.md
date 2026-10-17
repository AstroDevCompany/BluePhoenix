# Using BluePhoenix

BluePhoenix is a desktop command center. It is not a generic list of mixed projects. You switch **workspaces** (Software, University, and any other enabled categories) and only see tools that belong there.

## First launch

1. Create an account, sign in, or **Continue locally**.
2. Enable the categories you actually use. Disabled categories disappear from navigation.
3. Pick a default workspace. If you only use University, disable Software and BluePhoenix opens into courses.

You can redo this later in Settings → Workspace, including custom generic categories.

After the main window is ready, a **skippable AI popup** offers an OpenRouter key. Skip stores `ai.setupDismissed` and never nags again. Configure models, test the connection, and manage the key later under Settings → AI.

Export and import JSON from Settings → Data. The desktop app never needs Neon credentials.

The titlebar **AI** button opens a right-hand chat. It uses the open project when you are on a detail page. No key means the rest of the app still works.

## Switching workspaces and projects

Use the switcher at the top of the sidebar, or `Ctrl+1` / `Cmd+1` through `9` for enabled workspaces in order.

The sidebar lists your favorite and most recently opened projects with a live status dot (uncommitted changes, behind or ahead of the remote, timer running). Click one, or press `Alt+1` through `9`.

### Command palette

`Ctrl+K` / `Cmd+K` opens the command palette. It searches projects, TODOs and actions as you type. Press **Tab** on a project to see its actions: open in editor, open a terminal, fetch / pull / push, start or switch the timer, run any saved command. On a project page the palette offers that project's actions first. Press Enter on a full sentence with no match to optionally interpret it with AI.

`Ctrl+P` / `Cmd+P` is the same palette limited to jumping between projects (most recent first). Press `?` anywhere to see every shortcut.

| Shortcut | Action |
| --- | --- |
| `Cmd+K` | Command palette |
| `Cmd+P` | Jump to project |
| `Cmd+N` | New project / item |
| `Cmd+,` | Settings |
| `Cmd+E` | Open the current project in your editor |
| `Cmd+J` | Show or hide the terminal panel |
| `Cmd+Shift+T` | Start or stop the timer on the current project |
| `Esc` | Back from a project to the project list |

## Git, editors and terminals

Software project cards show live repository state: branch, commits ahead/behind, number of changed files, stash entries and the last commit. Status is read in the background (at start, when the window regains focus, and every minute and a half while it is focused), so the app never waits on git while you browse.

Each card and project page keeps the main actions within reach and tucks the rest away:

- **Editor**: opens the folder in your preferred editor. The **⋯** menu lists every supported editor found on your machine (VS Code, Cursor, Zed, Sublime Text, JetBrains IDEs, Android Studio, Xcode) and remembers your choice, and lists your system terminals (Terminal, iTerm, Warp, Ghostty or kitty on macOS; Windows Terminal on Windows).
- **Terminal** (project page): opens a shell in the project folder inside BluePhoenix.
- **Run**: runs the pinned command. Click the main button to run it in the embedded terminal, or use the caret to run it in a system terminal or pick another saved command.
- **Pull / Push**: appears on a card only when there is something to pull or push. Fetch, stash and the rest are in the **⋯** menu. Failures name their cause: merge conflict, authentication, network, timeout, or local changes that block the operation.

The star and **⋯** menu on a card appear when you hover it.

The **Overview** dashboard of the Software workspace shows today's and this week's tracked time, repositories that need attention (with one-click actions and **Fetch all**), your next TODOs and anything still running in a terminal.

### The terminal panel

Commands and shells run in your login shell, so your PATH, version managers and aliases behave as in your own terminal. The panel stays open while you navigate, so a dev server keeps running. Use **Stop** to end a process, **Rerun** to run it again, and drag the top edge to resize. Everything is stopped when you quit BluePhoenix. Commands that look destructive (for example `rm -rf`, `git reset --hard`) ask for confirmation first.

### Git tab

The project page has a Git tab with a branch switcher, the list of changed files, recent commits, stash and the same fetch / pull / push actions. Committing and diffing stay in your editor.

## Themes

Settings → Appearance offers Dark glass, Light glass, or Match system. The terminal panel stays dark in every theme.

## Creating something

**New** asks what you are creating first. Software, University, and generic forms are different on purpose.

- Software: folder, GitHub, website, languages, frameworks
- University: course name, optional university/professor/CFU/folder
- Generic: name, folder, optional primary file

## Project cards vs detail pages

A card is a summary. Clicking the card opens the workspace page. Action buttons (editor, terminal, run, git, folder, links, trophies) do **not** navigate. Use the star to pin a project to the top of lists and the sidebar, and the search, filter chips and sort menu on the Projects page to find things quickly.

On software projects, AI actions sit in the TODOs / Versions / Git sections: they propose text and wait for confirm. Commit messages are copy-only.

## Custom titlebar

The window uses a custom titlebar. On Windows, minimize/maximize/close are on the right. On macOS, traffic lights stay on the left; the rest of the bar is draggable.
