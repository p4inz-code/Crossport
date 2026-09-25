# Code Style

Formatting and import order are enforced by Biome (2-space indent, trailing
commas, organized imports). ESLint enforces React rules and unused-variable
errors (`^_` prefix to opt out of unused-parameter errors). Rust uses
`rustfmt` defaults.

## Commands

```bash
pnpm check          # Biome check (no fixes)
pnpm exec biome check --write .   # auto-fix formatting/imports
pnpm lint           # ESLint
cargo fmt           # Rust formatting
```

## TypeScript conventions

- Strict mode; `noUnusedLocals`, `noUnusedParameters`, `verbatimModuleSyntax`,
  `erasableSyntaxOnly` are on.
- No `any`. Prefer `unknown` + narrowing for untyped IPC payloads.
- Type-only imports use `import type`.
- Barrel files exist per directory (`index.ts`) and export only what is used.
- Domain values crossing the IPC boundary are validated with zod schemas
  defined in `src/types`.

## Rust conventions

- Modules mirror domains (settings, platform, filesystem, errors, state,
  commands).
- Commands return `AppResult<T>`; errors never serialize to bare strings.
- No `unwrap`/`expect` outside tests; poisoned locks map to `AppError`.

## Naming

- Files: `kebab-case` (`.ts`, `.tsx`, `.rs`).
- CSS classes: BEM (`block__element--modifier`).
- CSS custom properties: `--namespace-name` (e.g. `--color-primary`,
  `--space-4`).
