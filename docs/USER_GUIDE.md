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

`Ctrl+K` / `Cmd+K` searches locally as you type. Press Enter on a full sentence to optionally interpret it with AI and merge those project hits with FTS results.

## Switching workspaces

Use the switcher at the top of the sidebar, or `Ctrl+1` / `Cmd+1` through `9` for enabled workspaces in order.

`Ctrl+K` / `Cmd+K` opens the command palette: projects, courses, TODOs, and actions.

## Creating something

**New** asks what you are creating first. Software, University, and generic forms are different on purpose.

- Software: folder, GitHub, website, languages, frameworks
- University: course name, optional university/professor/CFU/folder
- Generic: name, folder, optional primary file

## Project cards vs detail pages

A card is a summary. Clicking the card opens the workspace page. Action buttons (Pull, VS Code, Open folder, Open primary file, trophies) do **not** navigate.

On software projects, AI actions sit in the TODOs / Versions / Git sections: they propose text and wait for confirm. Commit messages are copy-only.

## Custom titlebar

The window uses a custom titlebar. On Windows, minimize/maximize/close are on the right. On macOS, traffic lights stay on the left; the rest of the bar is draggable.
