# Testing Guide

## Running the suites

```bash
pnpm test                       # frontend (Vitest)
cd apps/desktop/src-tauri && cargo test   # backend
```

CI runs both; the frontend suite fails when zero test files are found, so a
"green" result always means tests actually ran.

## Frontend conventions

- Test files live next to the code: `src/**/*.test.{ts,tsx}`.
- Environment: jsdom (configured in `vite.config.ts`); setup in
  `src/test/setup.ts` (jest-dom matchers, Testing Library cleanup between tests
  — required because globals are off — and a `matchMedia` polyfill).
- Import `describe/it/expect` from `vitest` explicitly — no globals.
- Service tests mock the Tauri IPC module (`vi.mock("@tauri-apps/api/core")`)
  and set `isTauri()` per test to cover both runtime modes.
- Store tests mock the service module, so store logic is exercised without IPC.
- Page tests mock services and drive the real store; query by role and label so
  the assertions describe what a user sees.

## Backend conventions

- Unit tests live in `#[cfg(test)] mod tests` blocks inside the module they
  cover.
- Focus on validation rules and serialization contracts (the IPC boundary).
- Filesystem tests use `crate::filesystem::test_support::unique_temp_dir` and
  clean up after themselves; no test touches a path outside the temp directory.
- Platform-specific branches are unit-tested on every host where they compile;
  `#[cfg(not(windows))]` tests run in CI on Linux.

## Coverage expectations

- New services: test every public function and every failure fallback.
- New schemas: test accept + reject cases per rule.
- New stores: test hydration, setters, and failure fallbacks.
- New UI primitives: smoke-test rendering and variant classes.
