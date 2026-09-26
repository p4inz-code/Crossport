# FAQ

## How do I run the app?

```bash
pnpm install
pnpm --filter desktop exec tauri dev
```

## How do I run the tests?

`pnpm test` (frontend) and `cargo test` in `apps/desktop/src-tauri` (backend),
or `./scripts/test.sh` for both.

## Where are settings stored?

In Tauri builds, the Rust backend stores `settings.json` in the platform
app-config directory. In browser dev mode, the validated localStorage fallback
is used so `pnpm dev` works without the backend.

## Why does the frontend not call filesystem APIs?

All filesystem, drive, and dialog work happens in the Rust backend by design.
The webview is granted only `core:default` and CrossPort's own commands, so the
attack surface stays minimal.

## What error format does the backend use?

`{ "code", "message" }` objects with stable codes: `invalid_input`,
`path_not_found`, `path_not_directory`, `permission_denied`, `io`, and
`internal`. The frontend normalizes them to `IpcError` and switches on `code`,
never on the message. See `docs/architecture/ERROR_HANDLING.md`.

## Where are the backend logs?

stdout and a rotating `crossport`-named file (5 MiB × 3) in the platform
app-log directory. Startup logs print the resolved config and log directories,
and the user-facing path is listed in `docs/development/SETUP.md`.

## Why does a volume show no capacity?

Because the platform did not report any. An empty optical drive, a disconnected
network share, and a mount point CrossPort cannot classify all answer with
"unknown" instead of a guessed number, and a volume whose media is not ready is
listed as *Not available*. Facts the OS cannot provide are never invented.

## Which commands does the backend expose today?

`get_settings`, `update_settings`, `get_system_info`, `list_drives`,
`list_directory`, `inspect_path`, and `pick_directory` — see
`apps/desktop/README.md`.
