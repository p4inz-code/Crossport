# CrossPort 1.1.2

A Windows file-transfer utility that moves files between your drives — with a
review of what will happen before it starts, and proof of what landed when it
finishes.

CrossPort is free and offline: no account, no telemetry, no ads, and no
background synchronization.

---

## What's new

This is a **corrective release**. Nothing new was added; two defects were fixed.

- **Fixed: a second preserved copy could erase the first.** If CrossPort could
  not use a settings or history file, it sets that file aside so you can inspect
  it. If two files were set aside within the same millisecond, they were given
  the same name and the second one silently replaced the first — so the evidence
  you would need to diagnose the problem could disappear. Preserved files now
  always get their own name.
- **Fixed: the same input could be classified differently on different
  platforms.** A path that runs through a file (for example `C:\notes.txt\sub`)
  cannot exist. CrossPort now recognises that the same way everywhere instead of
  relying on a Windows-only error signal.

Also in this release:

- **CrossPort is now licensed under Apache-2.0.** CrossPort 1.1.1 and earlier
  releases were published under the MIT License and remain under it — their
  tags, sources, and installers are unchanged. The history is recorded in
  [`docs/legal/LICENSING.md`](https://github.com/p4inz-code/Crossport/blob/main/docs/legal/LICENSING.md).
- **The publisher is now P4inz Interactive Labs**, in the installers and the
  package metadata.

## Core capabilities

- Browse mounted volumes and the folders inside them, with real volume metadata
  (kind, filesystem, capacity, read-only state).
- Copy and move files and folders between volumes, one transfer at a time, with
  the plan shown before anything moves.
- Conflict strategies: `replace`, `skip` (the default), and `rename`.
- Post-transfer verification: `none`, `size`, or a streaming SHA-256 comparison,
  reported as a plain-language verdict on the job.
- Pause, resume, and cancel; a cancelled job keeps what it already committed and
  discards everything else.
- Durable, bounded history of finished transfers, and recovery for anything
  interrupted by a crash or a forced shutdown.
- An offline application: no network access, no telemetry, no update checker.

## Validation

| Check | Result |
| --- | --- |
| Frontend lint, format check, type build | clean |
| Frontend tests | 43 files, 455 tests passed |
| Rust format, check, clippy (`-D warnings`) | clean |
| Rust tests | 400 passed, 0 failed, 8 ignored, plus the release-artifact smoke test |

The 1.1.2 installer was then installed on a clean machine state and the
installed application was exercised end to end: browsing, a checksum-verified
copy, a skipped conflict, a move, a pause/resume of a 61-file transfer, a
cancel that kept its committed files and left no temporary files behind,
history, an empty recovery list, a clean shutdown, and an uninstall that left
your settings and history in place.

## Platform status

- **Windows 10 1607+ / Windows 11, 64-bit** — production supported. This is the
  only packaged and tested platform.
- **macOS and Linux** — application foundation only. Not packaged, not built,
  not tested. They are not supported, and they are not described as supported.

CrossPort is an application, not a filesystem driver.

## Known limitations

- The installers are **not code-signed**, so Windows shows an unknown-publisher
  warning (SmartScreen). Verify a download against `checksums.txt` before
  running it.
- The MSI is built and its metadata and payload are validated, but it was not
  installed here because that requires elevation.
- One transfer runs at a time; a queue of parallel jobs is not implemented.
- Verification proves exactly what it says it proved: with `size`, only the byte
  count is compared; only the `checksum` policy compares content.
- A pause resumes from the start of the file it was in the middle of; there is
  no byte-offset resume.
- A same-volume move is a single rename that writes no bytes, so it reports
  `not verified` rather than a pass.
- A destination inside its own source is refused, and path containment is
  decided lexically rather than by resolving links and junctions.
- Paths over 260 characters need Windows long-path support enabled.
- A downgrade is permitted rather than refused; there is no auto-updater.

## Installation

1. Download **`CrossPort_1.1.2_x64-setup.exe`** (recommended) or
   **`CrossPort_1.1.2_x64_en-US.msi`** from the release assets.
2. Run it. The NSIS installer installs for the current user into
   `%LOCALAPPDATA%\CrossPort` and adds a Start-menu entry and a desktop
   shortcut. No account and no setup step are involved.
3. Start CrossPort from the Start menu.

Windows will warn about an unknown publisher because the installers are not
signed. To check a download first:

```bash
sha256sum -c checksums.txt
```

To uninstall, use **Apps → Installed apps → CrossPort → Uninstall**. Your
settings, history, and logs are kept under `%APPDATA%\com.crossport.app` and
`%LOCALAPPDATA%\com.crossport.app` and are never removed for you.

---

CrossPort is free and offline: no account, no telemetry, no background
synchronization. Licensed under the Apache License 2.0.
