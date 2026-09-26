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
| `services/drives-service` | Volume payload parsing (including unknown facts), empty lists, contract violations, browser unavailability |
| `services/system-service` | Platform parsing, unknown platform rejection |
| `services/filesystem-service` | Path inspection, directory listings, cancelled dialogs, structured failures (`path_not_found`, `path_not_directory`, `permission_denied`), contract violations |
| `stores/settings-store` | Hydration, local validation, persisted writes, failure surfacing |
| `stores/drives-store` | Refresh, empty results, unreported volume metadata, structured failures, recovery |
| `stores/browser-store` | Opening locations, backend-normalized paths, back/up/refresh, stale-response protection, error surfacing, recovery, leaving the browser |
| `stores/system-store` | Hydration and browser-mode failure |
| `services/transfer-service` | Command payloads, argument forwarding, contract rejection, structured failures, event subscription (validated payloads, skipped garbage payloads, browser no-op) |
| `stores/transfer-store` | Queue loading and ordering, snapshot merging by identifier, control calls and their failures, dismissals that ignore late events, clearing finished jobs, following the event feed |
| `types/*` | Schema acceptance and rejection per rule |
| `lib/*` | Formatters and class composition |
| `features/**/*.test.tsx` | Volume metadata rendering, capacity meters, entry tables, navigation controls, multi-select and transfer actions, the loading/empty/error states of the drives page and the settings page, the transfer composer's dry run and conflict strategies, and the queue surface's progress, issues, and controls |

Run with `pnpm test` (or `pnpm --filter desktop test`).

## Backend (cargo test)

| Area | Coverage |
| --- | --- |
| `errors` | Serialization (`code`/`message`), stable and unique codes, `io::Error` mapping |
| `settings` | Validation rules, JSON round-tripping, forward/backward-compatible loading |
| `state` | Default state, settings replacement under the lock |
| `platform` | OS mapping, identifier stability, `SystemInfo` payload shape |
| `platform::paths` | Config file resolution inside the config directory |
| `platform::volume` | Drive-type classification, capacity derivation (underflow, inconsistent, missing values), label fallback, ordering, camelCase payload |
| `platform::drives` | Dedupe/sort stability, host enumeration, Windows probing (kind, filesystem, capacity consistency, mounted status) |
| `filesystem::directory` | Ordering (directories first), file/directory metadata, listing limits and truncation, symlink and broken-link handling, path validation, read-error mapping |
| `filesystem::path` | Empty/null/relative rejection, lexical cleanup, root-escape rejection, directory validation |
| `filesystem::metadata` | File/dir/root descriptions, missing paths, camelCase payload |
| `platform::drives` (space) | Free-space probing, volume roots, volume identity comparison |
| `transfer::model` | Stable identifiers, terminal/live states, percent and ETA rules, request defaults and rejection, serialized progress payload |
| `transfer::plan` | Recursive planning, ordering, collisions, skip strategies, item budget, space checks |
| `transfer::safety` | Source inspection, destination validation, unsafe relationships, chain creation and empty-directory cleanup, disk-full mapping |
| `transfer::conflict` | Collision detection, rename candidates, exhaustion |
| `transfer::copy` | Streaming with a bounded buffer, temp-file commits, move semantics, failure isolation |
| `transfer` (engine) | Queue order, one active job at a time, pause/resume/cancel on queued and running jobs, prune rules, concurrent control calls, progress accounting, throttled snapshots |
| `transfer::sanity` | Real end-to-end runs on disk: nested tree copy, move, all three conflict strategies, pause/resume/cancel cleanliness, a 192 MiB file streamed under a memory ceiling, unsafe requests leaving the disk untouched, queue order |

Run with `cargo test` in `apps/desktop/src-tauri`.

## CI

The CI pipeline runs both suites plus format, lint, build, and version-sync
checks on every push and pull request. See `docs/development/CI_CD.md`.
