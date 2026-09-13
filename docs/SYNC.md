# Sync and accounts

BluePhoenix is **local-first**. SQLite is what the UI reads. The cloud is optional.

## What syncs

Portable metadata: projects, categories, TODOs, time records, versions, links, university records, achievements, activity.

## What stays on the device

Absolute folder paths, primary files, Git/VS Code binaries, file-index cache, auth tokens (OS keychain).

Mac and Windows keep their own path bindings so one machine does not overwrite the other.

## Encrypted secrets

API keys that should follow your account (OpenRouter) are **not** ordinary sync rows.

- Unwrap material stays in the OS keychain (`BluePhoenix` / `openrouter_api_key` for plaintext, plus a local wrap-state blob).
- The wrap key is derived with Argon2id from the account password and email at sign-in. It is never stored on Neon.
- Sync pushes only AES-256-GCM ciphertext, nonce, and public wrap parameters (algorithm, memory, salt).
- A new device re-derives the wrap key on login and unwraps the envelope into the local keychain.
- Without an account, the OpenRouter key is local-only. Do not Base64-encode a key and call it encryption.

Auth session tokens never use that path.

## Status

Synced · Syncing… · Offline · Changes pending · Sync error

Sign in under Settings → Account. The desktop client talks to the app-owned `/v1` host. `claim-local` uploads this device's pending changes after login. Polling retries every 30 seconds if the websocket is down.
