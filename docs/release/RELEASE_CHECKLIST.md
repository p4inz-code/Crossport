# Release Checklist

The pass to run before tagging. It matches what the repository actually does —
Windows artifacts built locally, no publishing automation — so nothing here
describes a workflow that does not exist.

`docs/release/RELEASE_1.1.1.md` is the release document this checklist verifies
for the current line (with `RELEASE_1.1.0.md` and `RELEASE_1.0.md` kept as the
records for those releases): version, platform, installation, functionality,
verification, recovery, security model, artifacts, limitations, and what is
deliberately not supported. Keep them in step.

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
      --nocapture --test-threads=1` against `docs/development/PERFORMANCE.md`
      and update anything that moved materially
- [ ] Confirm no horizontal overflow and no clipped primary controls at
      1024×720, 1280×720, 1366×768, 1440×900, 1920×1080, a maximized window, a
      restored window, and a narrow window, in both light and dark themes

## Security model (spot-check)

- [ ] `apps/desktop/src-tauri/capabilities/default.json` still grants only
      `core:default` — no fs, dialog, shell, or http permission for the webview
- [ ] The production CSP in `tauri.conf.json` is unchanged, or a change is
      justified: no `unsafe-eval`, no remote origins, `object-src 'none'`,
      `form-action 'none'`, `frame-ancestors 'none'`
- [ ] No new dependency, command, or plugin was added that performs network
      access, telemetry, or update checking
- [ ] `cargo tree` shows no HTTP client in the Windows build
- [ ] Every new Tauri command validates its inputs in Rust and returns
      `{ code, message }` via `AppResult`; nothing security-relevant is decided
      in the frontend
- [ ] Logs contain no file contents and no credentials
      (`grep -rn "log::" apps/desktop/src-tauri/src`)

## Tag and publish

- [ ] Commit the version bump and the changelog entry
- [ ] `git tag vX.Y.Z`, then push the tag when publishing is intended
- [ ] Release notes written from `RELEASE_TEMPLATE.md`, listing the artifact
      names with their SHA-256 digests from `checksums.txt`

## Legal and branding metadata

- [ ] `LICENSE` and `NOTICE` are at the repository root and describe the current
      license (`docs/legal/LICENSING.md`)
- [ ] `license` is consistent across `package.json`, `apps/desktop/package.json`,
      and `apps/desktop/src-tauri/Cargo.toml`
- [ ] The Tauri bundle `publisher` and `copyright` name P4inz Interactive Labs,
      not a personal name and not an earlier studio name
- [ ] The bundled license file (`bundle.licenseFile`) still points at the
      repository `LICENSE`

## Known gaps

- Artifacts are unsigned: Windows shows an unknown-publisher warning until a
  certificate is configured (`docs/development/RELEASE_PROCESS.md`).
- Only Windows artifacts are built. No macOS or Linux bundle exists yet.
- A version upgrade was exercised (1.0.0 → 1.1.0). A downgrade is **not**
  refused: `allowDowngrades` is `false`, but it is only enforced on the
  interactive reinstall page, and a silent install replaces the newer build.
  Re-check both at the next version bump.
