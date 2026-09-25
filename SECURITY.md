# Security Policy

## Reporting a vulnerability

Please report suspected security vulnerabilities privately to the maintainers
rather than opening a public issue. Include:

- The affected version
- Steps to reproduce
- A description of the impact

Do not exploit the vulnerability beyond demonstrating it. We aim to respond
within 72 hours.

## Hardening model

- **Content Security Policy** — a restrictive CSP is enforced in production
  builds (`apps/desktop/src-tauri/tauri.conf.json`); a separate, looser dev
  CSP allows Vite HMR.
- **Minimal webview capability surface** — the frontend is granted only
  `core:default`. All filesystem and configuration access happens inside the
  Rust backend, never in the webview.
- **Backend-owned persistence** — user settings are stored by the Rust backend
  in the platform app-config directory; the webview localStorage fallback is
  used only in browser dev mode and is schema-validated.
- **Validated inputs** — settings received over IPC are validated on both
  sides of the boundary (zod + Rust validation) before use.
- **No secrets** — the application collects no credentials and stores no
  secrets. Do not introduce secret handling without review.

## Dependency updates

Dependency updates are tracked by Dependabot. Keep the dependency set small;
every dependency must justify its existence (see
`docs/development/CODING_RULES.md`).
