# CrossPort Desktop

The Tauri 2 desktop application for CrossPort, a file-transfer utility for
moving and verifying files across mounted volumes. Windows is the current
packaged and tested target; Linux and macOS are application foundation only
(see [`../../docs/product/PLATFORM_SUPPORT.md`](../../docs/product/PLATFORM_SUPPORT.md)).

## Stack

- Tauri 2 + Rust backend (`src-tauri/`)
- React 19 + TypeScript + Vite
- Zustand (state), Zod (validation), Lucide (icons), React Router (routing)
- Vitest + Testing Library (frontend tests), `cargo test` (backend tests)

## Frontend architecture (`src/`)

| Path | Purpose |
| --- | --- |
| `src/app/` | Application shell, providers, root routing |
| `src/components/` | Reusable UI primitives (Button, Card, EmptyState…) |
| `src/features/` | Feature folders: `drives` (volume rail + directory browser), `transfers` (composer, queue, dialog), `history`, `recovery`, `settings`, `notifications` |
| `src/hooks/` | Shared hooks (theme mode, keyboard shortcuts) |
| `src/layouts/` | AppShell, TopBar, Sidebar, StatusBar, PageContainer, close guard |
| `src/lib/` | Framework-agnostic utilities, constants, formatters |
| `src/services/` | IPC transport, error contract, and one service per backend domain |
| `src/stores/` | Zustand stores (app, system, drives, browser, settings, transfer, history, recovery, notification) |
| `src/styles/` | Design tokens and base styles |
| `src/test/` | Vitest setup and fixtures |
| `src/types/` | Shared domain types and zod schemas mirroring the Rust payloads |

Dependency direction is inward: features → stores → services → types.

## Backend architecture (`src-tauri/src/`)

| Module | Responsibility |
| --- | --- |
| `commands/` | Tauri command layer; filesystem and transfer work runs off the UI thread |
| `platform/` | Platform abstraction: OS identity, app directories, volume model and probing |
| `filesystem/` | Path normalization/validation, metadata inspection, single-directory listing, breadcrumb ancestors |
| `transfer/` | The copy/move engine: planning, queue, workers, conflict strategies, safety checks, progress |
| `verification/` | Post-transfer verification (size, SHA-256) and the job verdict |
| `persistence/` | The atomic, versioned document layer shared by state and history |
| `recovery/` | Classification of interrupted jobs and the user's restart / discard / confirm decision |
| `history/` | Durable, bounded transfer records |
| `settings/` | The single owner of user preferences; validation and file persistence |
| `errors/` | `AppError` with structured `code`/`message` serialization |
| `state/` | Managed application state |
| `logging.rs` | Log plugin configuration (stdout + rotating log file; never file contents) |

### Command surface

| Command | Purpose |
| --- | --- |
| `get_settings` / `update_settings` | Read and persist user preferences |
| `get_system_info` | Platform, OS, arch, and family of the host |
| `list_drives` | Storage volumes with kind, filesystem, capacity, read-only flag, and mounted status |
| `list_directory` | Validate a path and list one directory: entries with size, modification time, and kind |
| `list_ancestors` | A path and its parents, for the breadcrumb trail |
| `inspect_path` | Normalize a path and report its metadata |
| `pick_directory` | Native folder picker; returns a validated path or `null` |
| `plan_transfer` | Read-only dry run of a transfer request |
| `start_transfer` | Plan and queue a transfer |
| `list_transfers` / `get_transfer` | Read the queue and one job |
| `pause_transfer` / `resume_transfer` / `cancel_transfer` | Control a job |
| `remove_transfer` / `clear_finished_transfers` | Drop finished jobs |
| `list_history` / `get_archive_status` / `delete_history_record` / `clear_history` | Transfer history |
| `get_recovery_candidates` / recovery decisions | Interrupted-work recovery |

Transfer progress arrives as `transfer:update` events carrying a snapshot per
job, so the queue stays live without polling.

## Security configuration

- Production CSP: `default-src 'self'` with no `unsafe-eval`, and `object-src`,
  `form-action`, and `frame-ancestors` all locked to `'none'`; a separate dev
  CSP adds only what Vite HMR needs.
- Capabilities: `capabilities/default.json` grants `core:default` to the main
  window and nothing else — no fs, shell, or http permission.

## Development

```bash
pnpm install
pnpm dev                 # Vite dev server (browser mode: localStorage settings only)
pnpm build               # TypeScript + Vite production build
pnpm lint                # ESLint
pnpm test                # Vitest
pnpm preview             # Serve the production build
```

To run inside Tauri:

```bash
pnpm --filter desktop exec tauri dev
```

Backend checks:

```bash
cd src-tauri && cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test
```

## Design tokens

All visual decisions flow from the CSS custom properties in
`src/styles/tokens/`. Never hardcode colors, spacing, radius, typography,
elevation, or transitions in components — use the tokens.

## Testing

Unit, service, store, component, and page tests live next to the code
(`src/**/*.test.{ts,tsx}`); backend tests live in the modules they cover and
exercise real files. The suite fails when zero test files are found, so a green
`pnpm test` means tests actually ran. See
[`../../docs/development/TESTING_GUIDE.md`](../../docs/development/TESTING_GUIDE.md).
