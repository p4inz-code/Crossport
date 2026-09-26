# Module Architecture

Version: 3.0
Status: Approved

This document describes the module structure that exists today. The repository
is a single Tauri application under `apps/desktop/`; there is no multi-package
workspace. Modules are organized within the app, not as separate packages.

## Frontend (`apps/desktop/src`)

| Module | Responsibility | Dependency rules |
| --- | --- | --- |
| `app/` | Application shell, providers, root routing | Depends on layouts, components, stores, features |
| `features/` | Implemented feature folders: `drives` (volume list, directory browser, presentation helpers), `settings` | Each feature owns its pages and components; no cross-feature imports |
| `components/ui/` | Reusable UI primitives (Button, Card, EmptyState, …) | No business logic; depends only on `lib`, tokens |
| `layouts/` | AppShell, TopBar, Sidebar, StatusBar, PageContainer | Depends on components, stores, lib |
| `stores/` | Zustand stores (app, system, drives, browser, settings) | Owns client state; talks to services |
| `services/` | IPC transport (`ipc.ts`), per-domain services, validated storage | The only module allowed to call `invoke` |
| `hooks/` | Shared hooks (e.g. `useThemeMode`) | Depends on stores, types |
| `lib/` | Framework-agnostic utilities, constants, formatters | No dependencies on the rest of the app |
| `types/` | Domain types and zod schemas mirroring Rust payloads | No runtime dependencies |
| `styles/` | Design tokens and base styles | Global by design |

Dependency direction is always inward: features → stores → services → types.
Components never import stores directly; they receive data through props.

Feature folders exist only for implemented behavior. Transfers and history are
Phase 3 work and have no scaffolding.

## Backend (`apps/desktop/src-tauri/src`)

| Module | Responsibility |
| --- | --- |
| `commands/` | Tauri command layer, one submodule per domain (`settings`, `system`, `drives`, `filesystem`, `dialog`); blocking filesystem work runs on the blocking pool (`run_blocking`) |
| `platform/` | Platform abstraction: `Platform`/`SystemInfo`, app directories (`AppPaths`), the volume model (`volume.rs`), and volume detection with Windows probing (`drives.rs`) |
| `filesystem/` | Path normalization and directory validation (`path.rs`), metadata inspection (`metadata.rs`), single-directory listing (`directory.rs`) |
| `settings/` | The single source of truth for user preferences; validation and file persistence |
| `errors/` | `AppError` with structured `code`/`message` serialization |
| `state/` | Managed `AppState` (config, platform, current settings) |
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
- Circular imports are forbidden. Every import crosses at most one layer
  inward.
