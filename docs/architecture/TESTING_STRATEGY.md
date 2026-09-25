# Testing Strategy

Version: 2.0
Status: Approved

## Principles

- Tests must actually run: the suite fails when zero tests are found.
- Test behavior at boundaries: schema validation, IPC error normalization,
  store hydration/persistence, platform enumeration rules, path validation, and
  serialization contracts.
- Unit tests live next to the code they cover and use real temporary
  directories instead of mocks where the filesystem is the subject.
- Never weaken an assertion or delete a test to make a command pass.

## Frontend (Vitest + Testing Library)

| Area | Coverage |
| --- | --- |
| `services/ipc` | Error contract, code mapping, unknown codes, argument forwarding, schema rejection, runtime detection |
| `services/settings-service` | Browser fallback, validation before/after IPC, backend error passthrough |
| `services/drives-service` | Payload parsing, empty lists, contract violations, browser unavailability |
| `services/system-service` | Platform parsing, unknown platform rejection |
| `services/filesystem-service` | Path inspection, cancelled dialogs, contract violations |
| `stores/settings-store` | Hydration, local validation, persisted writes, failure surfacing |
| `stores/drives-store` | Refresh, empty results, structured failures, recovery |
| `stores/system-store` | Hydration and browser-mode failure |
| `types/*` | Schema acceptance and rejection per rule |
| `lib/*` | Formatters and class composition |
| `features/**/*.test.tsx` | Page rendering for the drives and settings pages, including error states |

Run with `pnpm test` (or `pnpm --filter desktop test`).

## Backend (cargo test)

| Area | Coverage |
| --- | --- |
| `errors` | Serialization (`code`/`message`), stable and unique codes, `io::Error` mapping |
| `settings` | Validation rules, JSON round-tripping, forward/backward-compatible loading |
| `state` | Default state, settings replacement under the lock |
| `platform` | OS mapping, identifier stability, `SystemInfo` payload shape |
| `platform::paths` | Config file resolution inside the config directory |
| `platform::drives` | Label derivation, filtering non-directories, dedupe/sort stability, host enumeration |
| `filesystem::path` | Empty/null/relative rejection, lexical cleanup, root-escape rejection, directory validation |
| `filesystem::metadata` | File/dir/root descriptions, missing paths, camelCase payload |

Run with `cargo test` in `apps/desktop/src-tauri`.

## CI

The CI pipeline runs both suites plus format, lint, build, and version-sync
checks on every push and pull request. See `docs/development/CI_CD.md`.
