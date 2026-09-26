# CrossPort Desktop

The Tauri 2 desktop application for CrossPort — a fast, reliable,
cross-platform file transfer utility.

## Stack

- Tauri 2 + Rust backend (`src-tauri/`)
- React 19 + TypeScript + Vite
- Zustand (state), Zod (validation), Lucide (icons), React Router (routing)
- Vitest + Testing Library (frontend tests), cargo test (backend tests)

## Frontend architecture (`src/`)

| Path | Purpose |
| --- | --- |
| `src/app/` | Application shell, providers, root routing |
| `src/components/` | Reusable UI primitives (Button, Card, EmptyState…) |
| `src/features/` | Implemented feature folders (`drives`: volume list + directory browser, `settings`) |
| `src/hooks/` | Shared hooks (e.g. `useThemeMode`) |
| `src/layouts/` | AppShell, TopBar, Sidebar, StatusBar, PageContainer |
| `src/lib/` | Framework-agnostic utilities, constants, formatters |
| `src/services/` | IPC transport, error contract, and one service per backend domain |
| `src/stores/` | Zustand stores (app, system, drives, browser, settings) |
| `src/styles/` | Design tokens and base styles |
| `src/test/` | Vitest setup |
| `src/types/` | Shared domain types and zod schemas mirroring the Rust payloads |

Dependency direction is inward: features → stores → services → types.

## Backend architecture (`src-tauri/src/`)

| Module | Responsibility |
| --- | --- |
| `commands/` | Tauri command layer (`settings`, `system`, `drives`, `filesystem`, `dialog`); filesystem work runs on the blocking pool |
| `platform/` | Platform abstraction: OS identity, app directories, volume model (`volume.rs`), volume probing (`drives.rs`) |
| `filesystem/` | Path normalization/validation, metadata inspection, single-directory listing (`directory.rs`) |
| `settings/` | The single owner of user preferences; validation and file persistence |
| `errors/` | `AppError` with structured `code`/`message` serialization |
| `state/` | Managed `AppState` (config, platform, current settings) |
| `config.rs` | Immutable application configuration |
| `logging.rs` | Log plugin configuration (stdout + rotating log file) |

### Command surface

| Command | Purpose |
| --- | --- |
| `get_settings` / `update_settings` | Read and persist user preferences |
| `get_system_info` | Platform, OS, arch, and family of the host |
| `list_drives` | Storage volumes with kind, filesystem, capacity, read-only flag, and mounted status |
| `list_directory` | Validate a path and list one directory: entries with size, modification time, and read-only flag, directories first |
| `inspect_path` | Normalize a path and report its metadata (used by transfer milestones for single paths) |
| `pick_directory` | Native folder picker; returns a validated path or `null` |

### Storage browser

`src/features/drives/` turns the `list_drives` and `list_directory` commands into
a desktop-style storage surface: a volume rail (kind icon, label, filesystem,
capacity meter, status notes) next to a browser with back/up/refresh controls,
the current location, an entry table, and loading, empty, and recovery states.
Navigation state lives in `src/stores/browser-store.ts`, which only ever opens
paths the backend produced and drops a listing that arrives after a newer
navigation started.

Errors cross the boundary as `{ code, message }`; codes are mirrored in
`src/services/ipc.ts` and the Rust `AppError`. The webview is granted only
`core:default` — filesystem, drive, and dialog work all happens in Rust, so no
plugin permissions are needed on the frontend.

## Security configuration

- Production CSP: `default-src 'self'` with no `object`, `frame`, or `form`
  escapes; a separate dev CSP adds only what Vite HMR needs.
- Capabilities: `capabilities/default.json` grants `core:default` to the main
  window and nothing else.

## Development

```bash
pnpm install
pnpm dev                 # Vite dev server (browser mode: localStorage settings only)
pnpm build               # TypeScript + Vite production build
pnpm lint                # ESLint
pnpm test                # Vitest
```

To run inside Tauri:

```bash
pnpm --filter desktop exec tauri dev
```

Backend checks:

```bash
cd src-tauri && cargo fmt --check && cargo check && cargo test
```

## Design tokens

All visual decisions flow from the CSS custom properties in
`src/styles/tokens/`. Never hardcode colors, spacing, radius, typography,
elevation, or transitions in components — use the tokens.

## Testing

Unit, service, store, component, and page tests live next to the code
(`src/**/*.test.{ts,tsx}`); backend tests live in the modules they cover. The suite fails when zero test files are found, so
a green `pnpm test` means tests actually ran. See
`docs/development/TESTING_GUIDE.md`.
