# Versioning

CrossPort follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
(Major.Minor.Patch).

## Single version, four manifests

The version must be kept identical in every manifest:

1. `package.json` (root) — the canonical release version
2. `apps/desktop/package.json`
3. `apps/desktop/src-tauri/tauri.conf.json` — drives the bundle version
4. `apps/desktop/src-tauri/Cargo.toml` — the Rust crate version

`scripts/check-versions.sh` asserts all four agree and runs in CI, so a bump
can never drift across manifests. Always run `bash scripts/check-versions.sh`
after any version change.

## 1.0 and beyond

CrossPort 1.0.0 is the first production release and the frozen development
baseline (`git tag v1.0.0`). While the baseline is frozen, only critical
security fixes, critical production bugs, and release-blocking corrections may
change it — each as a patch release (`1.0.x`). New features, architectural
expansion, and V2 work do not enter this line.

## Release process

Follow `docs/development/RELEASE_PROCESS.md`. Releases are built locally on
Windows with `scripts/release.sh`; the repository ships no publishing
automation, no update server, and no macOS or Linux bundle. CI (`.github/workflows/ci.yml`)
runs lint, tests, and the version check only — it never builds installers.

A tagged `v*` push does not trigger any workflow. Tagging is a record of what
was released, not a build trigger; the artifacts are attached to the release by
hand, with their `checksums.txt` digests.
