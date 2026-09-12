# AI

BluePhoenix AI is optional. Every existing flow works with no OpenRouter key and with AI turned off.

## Provider

OpenRouter Chat Completions (`https://openrouter.ai/api/v1/chat/completions`). Model identifiers are user-editable (for example `provider/model-name:free`). Request logic never hardcodes a model.

Up to eight slots live in `app_settings` (`ai.models`). Only the first four **non-empty** slots are attempted. Auth/config failures (`401`/`403`/missing key) stop. Rate limits, 404/unavailable, 5xx, timeouts, network errors, and malformed JSON fall through to the next slot.

Chat uses SSE streaming. Structured tools (search, TODOs, changelog, commit messages) use non-streaming JSON and are validated before any write.

## Secrets

- **Local:** OS keychain service `BluePhoenix`, account `openrouter_api_key`. The UI shows a masked key until you reveal it.
- **Cloud (signed in):** AES-256-GCM envelope in `encrypted_secrets` (`kind = openrouter_api_key`). The wrap key is Argon2id(account password + email) and stays on the device. Neon never receives plaintext or the wrap key. If you are not signed in, the key stays local-only.

## Surfaces

- Settings → AI: enable, key, test connection, eight model slots, commit-style toggle
- First-launch popup after the chrome is ready (skippable; never nags again)
- Titlebar toggle for the right-hand chat rail (local SQLite history)
- Command palette: local FTS on each keystroke; natural-language interpretation only on Enter
- Software project actions: prioritize TODOs, changelog helpers, coding-agent prompt, commit-message suggestion (copy only — never auto-commits)

## Documents

For non-software projects, attaching a file queues `index_documents`. Parsers (PDF, DOCX, TXT, MD, HTML) extract text, split into chunks, and index with FTS5. Embeddings are reserved and unused. Chat retrieves chunks and cites document/page/section when present. If retrieval is empty, the model is told not to invent content. Original files stay on disk.
