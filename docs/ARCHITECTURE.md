# Architecture

Monorepo:

- `apps/desktop` — Tauri v2 + React
- `apps/api` — Axum + Neon/Postgres
- `crates/domain` — GPA, XP, time, achievements, capabilities, semver
- `crates/sync-protocol` — metadata vs encrypted-secret DTOs
- `crates/ai` — OpenRouter client, fallback, schemas, document parsers, prompt builders

SQLite is the runtime source of truth. Cloud sync has two collections: portable metadata and opaque encrypted secrets.

Desktop Rust modules of note: `ai` (IPC), `secrets` (keychain + AES-GCM wrap), `documents` (background parsers), `git` (status/log/diff), `ai_store` (local conversations and chunks).

See the project README for run commands.
