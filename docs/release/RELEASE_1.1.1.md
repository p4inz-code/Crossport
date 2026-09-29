# CrossPort 1.1.1 — release document

Status: built and validated on Windows. Not code-signed. Windows only.

This is a **corrective patch** on the Windows 1.1 line. It restores the product
documented in `docs/release/RELEASE_1.1.0.md`, which remains the document for
that release. The `v1.0.0` and `v1.1.0` tags and their artifacts are untouched.

## Version

| Manifest | Version |
| --- | --- |
| `package.json` (root, canonical) | `1.1.1` |
| `apps/desktop/package.json` | `1.1.1` |
| `apps/desktop/src-tauri/tauri.conf.json` | `1.1.1` |
| `apps/desktop/src-tauri/Cargo.toml` | `1.1.1` |

`scripts/check-versions.sh` enforces that all four move together.

## What 1.1.1 fixes

**A wire-contract defect that broke the transfer queue and the start-transfer
flow.**

The live transfer snapshot (`TransferSnapshot`, which embeds
`VerificationSummary`) did not serialize the rendered verification verdict line
that the frontend contract requires (`verificationSummarySchema.verdict` in
`apps/desktop/src/types/verification.ts`). Rust exposed the verdict only as a
method, not as a serialized field, while the history record
(`HistoryVerification`) already carried it.

Because `invokeTyped` validates every payload it is handed, the missing field
made every `start_transfer`, `list_transfers`, and `get_transfer` call fail
frontend validation:

- starting a transfer in the composer reported *"unexpected payload"*, and
- the Transfers page reported the queue as **unavailable**.

The engine was never at fault: it planned, queued, copied, and verified
correctly. Only the value the UI reads was absent.

The fix serializes `verdict` as part of `VerificationSummary`, keeping the
rendered line and the serialized line the same value. A regression test
(`a_resumed_summary_keeps_the_digest_algorithm_on_the_wire` in
`apps/desktop/src-tauri/src/verification/tests.rs`) now asserts that the
serialized summary carries its verdict, so the field cannot silently disappear
again.

Also included (documentation only): the corrected downgrade claim from
`RELEASE_1.1.0.md` — a downgrade is permitted, not refused.

No engine, persistence, security, or IPC-shape behaviour changed beyond that
added field.

## Supported platform

Unchanged: **Windows 10 1607+ or Windows 11, 64-bit** is the only packaged and
tested target, with the Microsoft Edge WebView2 runtime. Paths over 260
characters need Windows long-path support enabled.

macOS and Linux are **not packaged, not built, and not tested**. CrossPort is an
application, not a filesystem driver. See
`docs/product/PLATFORM_SUPPORT.md` and `docs/product/PARAGON_CAPABILITY_GAP.md`.

## Verification performed

| Check | Result |
| --- | --- |
| `pnpm check` (Biome) | clean |
| `pnpm lint` (ESLint) | clean |
| `pnpm --filter desktop build` (tsc + Vite) | built |
| `pnpm test` (Vitest) | passed |
| `cargo fmt --check` | clean |
| `cargo check --all-targets` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo test` | 398 passed, 0 failed, 8 ignored, plus the release-artifact smoke test |
| `bash scripts/check-versions.sh` | all versions in sync: 1.1.1 |

### Live defect verification (the reason for this release)

The built 1.1.1 application was launched and driven directly (WebView2 remote
debugging), then a real copy was executed through the application's own backend
and the interface inspected:

- the Transfers page rendered the job: status **Completed**, 11.4 MB of
  11.4 MB, 2 of 2 files, and the verdict
  **"SHA-256 verification — verified (size_and_checksum, 2 files, 12000000 bytes)"**;
- a completion notification carried the same verdict;
- the version footer reported **v1.1.1**.

## Artifacts and checksums

`bash scripts/release.sh` writes, under
`apps/desktop/src-tauri/target/release/bundle/`:

| Artifact | Path |
| --- | --- |
| NSIS installer | `nsis/CrossPort_1.1.1_x64-setup.exe` |
| WiX MSI | `msi/CrossPort_1.1.1_x64_en-US.msi` |
| Checksums | `bundle/checksums.txt` (SHA-256 per artifact) |

Verify with:

```bash
cd apps/desktop/src-tauri/target/release/bundle
sha256sum -c checksums.txt
```

Digest values are not recorded here on purpose: they change with every build,
and `checksums.txt` is the authoritative list that ships beside the artifacts.

## Known limitations

All limitations from 1.1.0 still apply: Windows only, not code-signed, one
transfer at a time, bounded history, verification proves only the claims it
lists, no byte-offset resume, lexical path containment, long paths needing
Windows support, and a downgrade being permitted rather than refused.

The MSI is built but not installed here (it requires elevation); the NSIS
install and lifecycle evidence for this line comes from the artifact smoke test
and the interactive pass recorded under "Release 1.1" in `ROADMAP.md`.

## Release procedure

1. `bash scripts/check-versions.sh`
2. The full gate (see the table above).
3. `bash scripts/release.sh` — builds the frontend, builds with `--locked`, and
   writes `checksums.txt`.
4. `cargo test --test artifact_smoke -- --nocapture`
5. `sha256sum -c checksums.txt` from the bundle directory.
6. Tag `v1.1.1` at the validated commit and attach the installers plus
   `checksums.txt`. No workflow builds or publishes anything.

## Not supported (deliberately out of scope)

Unchanged from 1.0.0 and 1.1.0: folder synchronization, scheduling, cloud
storage and accounts, telemetry, update checking, remote filesystem access,
macOS/Linux packaging, and code signing. See `NON_GOALS.md` and
`docs/product/FEATURE_SPECIFICATION.md`.
