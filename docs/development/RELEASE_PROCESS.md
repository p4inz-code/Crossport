# Release Process

CrossPort releases are built locally. There is no publishing automation, no
update server, and no code signing in this repository yet; the process below
produces the installers, checksums them, and leaves them in the build
directory.

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

Plus the checks that need a built artifact (run after `scripts/release.sh`, so
the binary exists):

```bash
cd apps/desktop/src-tauri
cargo test --test artifact_smoke -- --nocapture    # starts the built app
cargo test --lib measure -- --ignored --nocapture  # performance numbers
```

The artifact smoke test launches `target/release/CrossPort.exe` from a
temporary directory with a stripped environment (no Node, no pnpm, no Cargo, no
repository) and asserts that it writes its startup line, creates a visible
window, and exits cleanly when the window is closed. Set `CROSSPORT_APP_EXE` to
check a different binary. Without a built artifact it reports that it skipped.

## Producing the artifacts

```bash
bash scripts/release.sh
```

What it does, in order:

1. asserts every manifest agrees on the version;
2. builds the frontend (`pnpm --filter desktop build`);
3. runs `tauri build -- --locked`, so cargo build fails rather than resolving a
   dependency the committed `Cargo.lock` does not pin;
4. writes `apps/desktop/src-tauri/target/release/bundle/checksums.txt` with a
   SHA-256 digest for every `.exe`, `.msi`, and `.zip` under that bundle
   directory.

Artifacts land in `apps/desktop/src-tauri/target/release/bundle/`:

| Artifact | Bundle | Install scope |
| --- | --- | --- |
| `nsis/CrossPort_<version>_x64-setup.exe` | NSIS | current user, into `%LOCALAPPDATA%\CrossPort` |
| `msi/CrossPort_<version>_x64_en-US.msi` | WiX | per-machine, needs elevation |

The version in the file name comes from `tauri.conf.json`, so a mismatch is
caught by `scripts/check-versions.sh` before the build starts.

### Verifying a checksum

```bash
cd apps/desktop/src-tauri/target/release/bundle
sha256sum -c checksums.txt
```

### What the installers do

- They install the application, its icons, a Start-menu entry, and (NSIS) a
  desktop shortcut; both register an entry in **Apps → Installed apps** for
  uninstalling. The installed executable is `CrossPort.exe`: `mainBinaryName`
  in `tauri.conf.json` renames cargo's `crossport` binary before bundling, so
  the file a user sees matches the product rather than the crate.
- They do not touch `%APPDATA%\com.crossport.app` or
  `%LOCALAPPDATA%\com.crossport.app` — the settings, history, recovery state,
  logs, and WebView2 profile a previous install created. Both installers can be
  run over an existing installation of the same version: the program files are
  replaced and the user's data is kept.
- Installing an older version over a newer one is **not** blocked.
  `bundle > windows > allowDowngrades` is `false`, but the NSIS template that
  reads it only enforces it on the interactive reinstall page, where a
  downgrade requires uninstalling the newer version first; a silent install
  (`/S`) skips that page and replaces the newer build outright. A downgraded
  build does not corrupt user data — a newer history document is refused and
  left untouched rather than read — but only install a build you mean to run.
- Uninstalling removes the program files, the shortcuts, and the uninstall
  entry. It leaves the user data directories in place; deleting them is the
  user's decision.

### What has and has not been exercised

Run against the built installer on Windows: a fresh install, launching the
installed `CrossPort.exe` on an isolated desktop, a same-version reinstall over
it, and an uninstall. Each step was checked for its result, not just its exit
code: the install directory, the `HKCU\...\Uninstall\CrossPort` entry, the
shortcut, and — after the uninstall — that
`%APPDATA%\com.crossport.app` and `%LOCALAPPDATA%\com.crossport.app` are still
there.

Exercised since against the two shipped versions: an **upgrade** from 1.0.0 to
1.1.0 (the program files are replaced while the registry entry, shortcuts, and
user data are kept) and a **downgrade**, which is not refused (see above). The
MSI was built and its per-machine scope is recorded above, but installing it
requires elevation, which was not available here.

### WebView2

`bundle > windows > webviewInstallMode` is `downloadBootstrapper` with
`silent: true`: on a machine without the WebView2 runtime, the installer
downloads and installs it. A machine that is offline during installation needs
the runtime already present (Windows 11 and current Windows 10 ship it).

## Signing

Artifacts are unsigned, so Windows shows an unknown-publisher warning on first
install. To sign a build, set `bundle > windows > certificateThumbprint` (and
`digestAlgorithm`/`timestampUrl` if the certificate requires them) in
`tauri.conf.json`; the bundler then signs the installers and the executable with
the certificate in the current user's certificate store. Nothing in the
repository assumes a certificate exists.

## Tag and release

1. Commit the version bump and the changelog entry.
2. Tag it (`git tag vX.Y.Z`), then push the tag when you intend to publish.
3. Build the artifacts locally with `scripts/release.sh`.
4. Run the pre-release checks from this document against the artifacts.
5. Write the release notes from `docs/release/RELEASE_TEMPLATE.md`, list the
   artifact names with their SHA-256 digests from `checksums.txt`, and publish
   them wherever the project publishes downloads.

`docs/release/RELEASE_CHECKLIST.md` is the pass to run before tagging.
