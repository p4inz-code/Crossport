# Release Checklist

## Before tagging

- [ ] Versions in sync: `bash scripts/check-versions.sh`
- [ ] `pnpm lint` and `pnpm check` pass
- [ ] `pnpm --filter desktop build` passes
- [ ] `pnpm test` passes (real tests, not zero)
- [ ] `cargo fmt --check`, `cargo check`, `cargo test` pass
- [ ] `CHANGELOG.md` updated
- [ ] Docs reviewed for drift (`docs/`)

## After tagging

- [ ] Release workflow produced Windows, macOS, and Linux bundles
- [ ] Smoke-tested each platform artifact (settings persist across restart)
- [ ] Draft release notes written from `RELEASE_TEMPLATE.md`
- [ ] Published release
