# Release Process

## Version bump

1. Bump `version` in `package.json` (root).
2. Update the same version in `apps/desktop/package.json`,
   `apps/desktop/src-tauri/tauri.conf.json`, and
   `apps/desktop/src-tauri/Cargo.toml`.
3. Add a `CHANGELOG.md` entry.
4. Run `bash scripts/check-versions.sh` — CI enforces this too.

## Pre-release verification

```bash
./scripts/lint.sh
./scripts/test.sh
pnpm --filter desktop build
bash scripts/check-versions.sh
```

## Tag and release

1. Create and push a `vX.Y.Z` tag; the release workflow builds Windows, macOS,
   and Linux bundles and opens a draft GitHub release.
2. Smoke-test each platform artifact.
3. Publish the draft release with the changelog entry.

## Release template

Use `docs/release/RELEASE_TEMPLATE.md` for release notes and
`docs/release/RELEASE_CHECKLIST.md` for the final pass.
