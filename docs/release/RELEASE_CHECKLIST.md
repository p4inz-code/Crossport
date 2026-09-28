# Release Checklist

The pass to run before tagging. It matches what the repository actually does —
Windows artifacts built locally, no publishing automation — so nothing here
describes a workflow that does not exist.

## Before tagging

- [ ] Versions in sync: `bash scripts/check-versions.sh`
- [ ] `pnpm lint` and `pnpm check` pass
- [ ] `pnpm --filter desktop build` passes (typecheck + production bundle)
- [ ] `pnpm test` passes, and the performance budgets in it are green
- [ ] `cargo fmt --check`, `cargo check --all-targets`,
      `cargo clippy --all-targets`, `cargo test` pass
- [ ] `CHANGELOG.md` updated, with the version and date
- [ ] Docs reviewed for drift (`docs/`), especially `README.md` limitations

## Build the artifacts

- [ ] `bash scripts/release.sh` produces the NSIS installer and the MSI under
      `apps/desktop/src-tauri/target/release/bundle/`, and `checksums.txt`
- [ ] `sha256sum -c checksums.txt` passes from the bundle directory

## Verify the artifacts on Windows

- [ ] `cargo test --test artifact_smoke -- --nocapture`: the built
      `CrossPort.exe` starts from an isolated directory with no development
      tools in its environment, writes its startup line, shows a window, and
      exits cleanly when the window is closed
- [ ] Install the NSIS installer: program files in `%LOCALAPPDATA%\CrossPort`,
      an **Apps → Installed apps** entry, a Start-menu entry, a desktop shortcut
- [ ] Launch the installed `CrossPort.exe` and confirm a fresh startup line in
      `%LOCALAPPDATA%\com.crossport.app\logs`
- [ ] Re-run the installer over the same version: the program files are replaced
      and user data is kept
- [ ] Uninstall: program files, shortcuts, and the registry entry are gone, and
      `%APPDATA%\com.crossport.app` and `%LOCALAPPDATA%\com.crossport.app` are
      still there
- [ ] Record the numbers from `cargo test --lib measure -- --ignored
      --nocapture` against `docs/development/PERFORMANCE.md` and update anything
      that moved materially

## Tag and publish

- [ ] Commit the version bump and the changelog entry
- [ ] `git tag vX.Y.Z`, then push the tag when publishing is intended
- [ ] Release notes written from `RELEASE_TEMPLATE.md`, listing the artifact
      names with their SHA-256 digests from `checksums.txt`

## Known gaps

- Artifacts are unsigned: Windows shows an unknown-publisher warning until a
  certificate is configured (`docs/development/RELEASE_PROCESS.md`).
- Only Windows artifacts are built. No macOS or Linux bundle exists yet.
- A version upgrade and a refused downgrade have not been exercised, because
  only one version exists. Re-check both at the first real version bump.
