# Settings Architecture

Version: 1.0
Status: Approved

## Ownership

The Rust backend owns user settings. The frontend store (`settings-store.ts`)
is a projection: it mirrors the backend state for rendering and sends changes
through the settings service.

## Path

| Mode | Source of truth | Persistence |
| --- | --- | --- |
| Tauri (desktop) | Rust `AppState` + `settings.json` | JSON file in the platform app-config directory |
| Browser dev | localStorage (validated) | `crossport:settings` key |

The fallback exists purely so `pnpm dev` works in a browser. It is
schema-validated with zod and documented as a dev convenience, never as a
second source of truth.

## Schema

- `theme`: `"light" | "dark" | "system"` (validated by zod and by Rust)
- `locale`: 2–16 characters
- `verification`: `"none" | "size" | "checksum"` — how thoroughly a new transfer
  verifies what it writes. Default `size`; see
  `docs/architecture/VERIFICATION.md`
- `historyLimit`: integer 20–2000 — how many finished transfers history keeps
  (`DEFAULT_HISTORY_LIMIT` 200, `MIN_HISTORY_LIMIT` 20, `MAX_HISTORY_LIMIT`
  2000). The same three constants live in `history/mod.rs` and in
  `src/types/settings.ts`

Both sides enforce the same rules; the Rust side is authoritative.

## When a setting takes effect

`verification` is **folded into a job's request** when it is planned or started,
so a queued or running job keeps the policy it was shown at the time. Changing
the setting affects the next job, never one already accepted — and history
records the policy a job actually ran under.

`historyLimit` applies immediately: `update_settings` propagates it to the open
archive, which prunes the oldest records down to the new limit, writes the
trimmed list, and reports how many records were removed. A history document that
is read-only (written by a newer schema) or degraded is left untouched rather
than replaced, and the new limit still governs what happens in memory.

## Command surface

- `get_settings` → `AppSettings`
- `update_settings(AppSettings)` → `()` (validates, persists, updates state, and
  applies the history retention limit to the open archive)

Errors are returned as `{ code, message }` and normalized to `IpcError` in the
frontend; see `docs/architecture/ERROR_HANDLING.md`.

## File lifecycle

- Missing file → defaults (created on first save).
- Field added by a newer build or absent from an older file → filled from the
  struct defaults; unknown fields are ignored (`serde(default)`), so a settings
  file written by another version still loads.
- Corrupt or invalid file → logged loudly, defaults used, file rewritten on next
  save.
- Writes go through a temp file + rename so a crash cannot truncate the file.

## Perceived state

The settings store keeps the write result in state (`saveStatus`, `error`), so
the page can confirm a save or show the structured failure instead of assuming
success. A change that fails to persist stays visible in the UI and reports the
backend message.

## Security

The settings file is written with default OS permissions in the user's own
app-config directory. No credentials are stored.
