# Updates

Two independent checks:

1. **RAW JSON** — Settings → Updates → RAW version URL (default points at `version.json`). The app fetches JSON, parses a semantic version with the `semver` crate, and compares it to the installed version. Versions are never compared as strings.
2. **Tauri updater** — download/install when you ship signed artifacts and a valid updater public key.

Failures (offline, timeout, malformed JSON, invalid semver, HTTP 429) surface in the UI.

Configure automatic checks, channel, and the RAW URL in Settings. Replace the placeholder updater pubkey in `apps/desktop/src-tauri/tauri.conf.json` before production.
