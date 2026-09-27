# Module Architecture

Version: 4.0
Status: Approved

This document describes the module structure that exists today. The repository
is a single Tauri application under `apps/desktop/`; there is no multi-package
workspace. Modules are organized within the app, not as separate packages.

## Frontend (`apps/desktop/src`)

| Module | Responsibility | Dependency rules |
| --- | --- | --- |
| `app/` | Application shell, providers, root routing | Depends on layouts, components, stores, features |
| `features/` | Implemented feature folders: `drives` (volume list, directory browser, transfer composer), `transfers` (queue surface, composer dialog), `history`, `recovery`, `notifications`, `settings` — each with its own presentation helpers | Each feature owns its pages and components; no cross-feature imports. `drives` renders the `transfers` composer dialog, which is the one shared presentational component, not a state dependency |
| `components/ui/` | Reusable UI primitives (Button, Card, EmptyState, …) | No business logic; depends only on `lib`, tokens |
| `layouts/` | AppShell, TopBar, Sidebar, StatusBar, PageContainer | Depends on components, stores, lib |
| `stores/` | Zustand stores (app, system, drives, browser, transfers, settings, history, recovery, notifications) | Owns client state; talks to services. The transfer store merges job snapshots by identifier and never synthesizes progress; the notification store is bounded and deduplicated by job and event |
| `services/` | IPC transport (`ipc.ts`), per-domain services (`transfer`, `history`, `recovery`, …), validated storage | The only module allowed to call `invoke` |
| `hooks/` | Shared hooks (`useThemeMode`, `useTransferFeed`, `useRecoveryFeed`, `useTransferNotifications`) | Depends on stores, types. `useTransferFeed` and `useTransferNotifications` are mounted once by the shell, so a running transfer stays visible across pages and its outcome is announced exactly once |
| `lib/` | Framework-agnostic utilities, constants, formatters | No dependencies on the rest of the app |
| `types/` | Domain types and zod schemas mirroring Rust payloads | No runtime dependencies |
| `styles/` | Design tokens and base styles | Global by design |

Dependency direction is always inward: features → stores → services → types.
Pages wire stores; components receive data through props.

Feature folders exist only for implemented behavior; there is no scaffolding for
unimplemented work. Transfer, history, recovery, and notifications are all
implemented.

## Backend (`apps/desktop/src-tauri/src`)

| Module | Responsibility |
| --- | --- |
| `commands/` | Tauri command layer, one submodule per domain (`settings`, `system`, `drives`, `filesystem`, `dialog`, `transfer`, `history`, `recovery`); blocking filesystem work runs on the blocking pool (`run_blocking`) |
| `platform/` | Platform abstraction: `Platform`/`SystemInfo`, app directories (`AppPaths`), the volume model (`volume.rs`), and volume detection with Windows probing (`drives.rs`) |
| `filesystem/` | Path normalization and directory validation (`path.rs`), metadata inspection (`metadata.rs`), single-directory listing (`directory.rs`) |
| `transfer/` | The transfer engine: wire model (`model.rs`), planning (`plan.rs`), safety rules and cleanup (`safety.rs`), conflict strategies (`conflict.rs`), streaming copy, move, and per-item verification (`copy.rs`), queue and workers (`mod.rs`), engine tests (`tests.rs`) and real on-disk end-to-end runs (`sanity.rs`). Publishes progress through the `TransferPublisher` trait, publishes what happened through `TransferJournal`, and never depends on the Tauri windowing layer |
| `verification/` | Policies, per-file results, the job-wide verdict, and streaming SHA-256 (`mod.rs`, `hash.rs`) |
| `recovery/` | Crash recovery: the durable state store, classification of interrupted jobs, and artifact scanning (`mod.rs`, `artifacts.rs`) |
| `history/` | The durable record of finished and interrupted transfers, retention, and filters |
| `persistence/` | The document layer every transfer document goes through: atomic writes, `schemaVersion`, migration, corrupt-file preservation |
| `archive.rs` | The durable side of transfers: history, interrupted-job state, and recovery, behind the `TransferJournal` port |
| `settings/` | The single source of truth for user preferences (theme, locale, verification policy, history retention); validation and file persistence |
| `errors/` | `AppError` with structured `code`/`message` serialization |
| `state/` | Managed `AppState` (config, platform, current settings, the open archive) |
| `config.rs` | Immutable application configuration |
| `logging.rs` | Log plugin configuration (stdout + rotating log file) |

## Rules

- Business logic never lives in UI components.
- The frontend never touches the filesystem or the OS; all of it stays in
  Rust behind typed commands, so the webview capability surface stays minimal.
- Platform differences are isolated in `platform/`; no other module branches on
  `cfg(windows)` or reads OS constants.
- Settings have exactly one owner (the backend). The webview localStorage
  fallback exists only for browser dev mode and is schema-validated.
- Errors are structured on both sides: `AppError` in Rust, `IpcError` in the
  frontend, with the code list duplicated deliberately and asserted by tests.
- The transfer engine depends on `platform/`, `filesystem/`, and `errors/`, and
  nothing depends on it except the command layer, the event publisher, and the
  archive's journal port, so it stays testable without a window. Every number the
  UI shows — sizes, speeds, ETAs, collisions, check counts, verdicts — is
  produced by the backend, never by the frontend.
- The archive is the only module that knows both the live engine and the durable
  side. It is opened during setup, installed into `AppState`, and reached through
  commands; nothing else writes a transfer document.
- Circular imports are forbidden. Every import crosses at most one layer
  inward.
