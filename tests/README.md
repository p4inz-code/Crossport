# Tests

CrossPort ships with real, runnable tests for both halves of the stack.

## Frontend (Vitest)

- Location: `apps/desktop/src/**/*.test.{ts,tsx}` (next to the code)
- Command: `pnpm test`
- Docs: `docs/development/TESTING_GUIDE.md`, `docs/architecture/TESTING_STRATEGY.md`

## Backend (cargo test)

- Location: `#[cfg(test)] mod tests` inside `apps/desktop/src-tauri/src/**`
- Command: `cd apps/desktop/src-tauri && cargo test`

## Guarantees

- The frontend suite fails when zero test files are found — a green result
  always means tests ran.
- CI (`ci.yml`) runs both suites plus lint, format, build, and version checks.
