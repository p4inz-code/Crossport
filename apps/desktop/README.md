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
| `src/features/` | Implemented feature folders (`drives`, `settings`) |
| `src/hooks/` | Shared hooks (e.g. `useThemeMode`) |
| `src/layouts/` | AppShell, TopBar, Sidebar, StatusBar, PageContainer |
| `src/lib/` | Framework-agnostic utilities, constants, formatters |
| `src/services/` | IPC transport, error contract, and one service per backend domain |
| `src/stores/` | Zustand stores (app, system, drives, settings) |
| `src/styles/` | Design tokens and base styles |
| `src/test/` | Vitest setup |
| `src/types/` | Shared domain types and zod schemas mirroring the Rust payloads |

Dependency direction is inward: features → stores → services → types.

## Backend architecture (`src-tauri/src/`)

| Module | Responsibility |
| --- | --- |
| `commands/` | Tauri command layer (`settings`, `system`, `drives`, `filesystem`, `dialog`) |
| `platform/` | Platform abstraction: OS identity, app directories, drive roots |
| `filesystem/` | Path normalization/validation and metadata inspection |
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
| `list_drives` | Storage roots the user can currently reach |
| `inspect_path` | Normalize a path and report its metadata |
| `pick_directory` | Native folder picker; returns a validated path or `null` |

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

Unit, service, store, and page tests live next to the code
(`src/**/*.test.{ts,tsx}`). The suite fails when zero test files are found, so
a green `pnpm test` means tests actually ran. See
`docs/development/TESTING_GUIDE.md`.
