# Architecture

Monorepo:

- `apps/desktop` — Tauri v2 + React
- `apps/api` — Axum + Neon/Postgres
- `crates/domain` — GPA, XP, time, achievements, capabilities, semver
- `crates/sync-protocol` — metadata vs encrypted-secret DTOs
- `crates/ai` — OpenRouter client, fallback, schemas, document parsers, prompt builders

SQLite is the runtime source of truth. Cloud sync has two collections: portable metadata and opaque encrypted secrets.

Desktop Rust modules of note: `ai` (IPC), `secrets` (keychain + AES-GCM wrap), `documents` (background parsers), `ai_store` (local conversations and chunks), and the developer-workflow modules below.

### Developer workflow (git, terminals, launching)

- `git` shells out to the system `git` with `GIT_TERMINAL_PROMPT=0`, a hard timeout and English messages. Status comes from a single `status --porcelain=v2 --branch --show-stash` call; failures are classified into typed errors (`conflict:`, `auth:`, `network:`, `timeout:`, `dirty:`) that the UI renders.
- `git_cache` keeps the last status per folder. Project cards read only from this cache and never spawn git. A background thread refreshes every tracked repository at start, on window focus and periodically while focused, with bounded parallelism, and emits `git-status-updated` events that the frontend patches into its query cache. Git actions (`git_run`, `git_fetch_all`, `git_refresh`) are async commands that never hold the database lock.
- `terminal` owns embedded terminal sessions (`portable-pty`). It spawns the user's login shell, streams output as `term-data` / `term-exit` events, keeps a bounded backlog for re-attaching, and terminates the whole process group on stop and on exit. A command's working directory is confined to the project folder.
- `native` detects installed editors and terminals, opens folders in them, and builds the platform-specific launch commands.

### Desktop frontend

- `src/lib/queries.ts` holds TanStack Query hooks (`useProjects`, `useRunningTimer`, ...); `useGitStatusEvents` patches git updates into every cached project list.
- `src/stores` (zustand) hold UI state only: palette, toasts, recent projects, terminal sessions and the output buffers that outlive xterm instances.
- `src/lib/shortcuts.ts` is the single place global shortcuts are defined; the shortcut sheet and the palette hints read from it.
- Menus, dialogs, tooltips and tabs are Radix primitives wrapped in `src/components/ui`.
- Styles live in `src/styles/*.css`, imported in order from `index.css` (tokens, base, layout, surfaces, controls, overlays, ai, motion, platform, pages, menus, chrome, terminal, projects, theme-light). `backdrop-filter` is reserved for large surfaces (titlebar, sidebar, dock, overlays, menus, tooltips, toasts), not for repeated controls.

See the project README for run commands.
