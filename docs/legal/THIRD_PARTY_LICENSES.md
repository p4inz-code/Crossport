# Third-Party Licenses

CrossPort's current development line is distributed under the Apache License 2.0
(see `LICENSE`). Releases up to and including 1.1.1 were published under the MIT
License and remain under it (see `docs/legal/LICENSING.md`). CrossPort depends on
third-party libraries; their licenses are preserved as follows:

- **Frontend/JS** — dependency metadata is recorded in `package.json` /
  `pnpm-lock.yaml`. Each package carries its own license in
  `node_modules/<package>/LICENSE*`; run a license audit before each release
  (e.g. `pnpm licenses list`).
- **Rust** — dependencies are recorded in `apps/desktop/src-tauri/Cargo.toml`
  and locked in `Cargo.lock`. License metadata is available via
  `cargo metadata`; run `cargo deny` (or an equivalent audit) before each
  release.

All dependencies must carry permissive or otherwise acceptable licenses and
must justify their existence per `docs/development/CODING_RULES.md`.
