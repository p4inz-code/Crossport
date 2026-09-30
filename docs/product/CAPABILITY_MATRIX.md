# Capability Matrix

What CrossPort does today, what it deliberately does not, and what is planned.
The three lists are kept apart on purpose: a future capability is never
described as a current one. Terminology matches the rest of the product
documentation.

Version: **1.1.2**. Platform: **Windows 10 1607+ / Windows 11, 64-bit**.

## CURRENT (built, packaged, and tested)

| Capability | Notes |
| --- | --- |
| Browse mounted volumes | Volume rail with kind, filesystem, capacity, read-only, and mounted state |
| Browse directories | Path validation in Rust; listings never recurse |
| Select files and folders | Checkbox selection; the selection is the transfer source |
| Copy | Recursive, with a reviewed plan before it starts |
| Move | Same-volume move is a rename; cross-volume copies, verifies, then deletes the source |
| Replace / Skip / Rename conflicts | Chosen per job and shown before it is queued |
| Dry-run planning | `plan_transfer` is read-only and reports what would happen |
| Size verification | Default policy: the written length is compared against the plan |
| SHA-256 verification | Optional, computed while copying; compares source bytes against the file on disk |
| Pause / Resume | Byte-level progress stops and continues on the same job |
| Cancel | Discards the partial output a job had written |
| Recovery handling | Interrupted jobs are classified into explicit outcomes; restart / discard / confirm is the user's decision |
| Transfer history | Durable, bounded, with per-record verification verdicts |
| Read-only / unavailable state reporting | Reported honestly; CrossPort cannot make a read-only volume writable |
| Notifications | One per terminal transition, each able to open the surface it is about |
| Responsive / fullscreen interface | Wide surfaces use the window; the navigation rail tightens before dropping labels |
| Light / dark / system themes | Drawn from one token set |
| Keyboard navigation | `Ctrl`/`Cmd`+`1`…`6` between pages; `Alt`+arrows in the browser |

## NOT CURRENT (deliberately out of scope — not attempted)

These require privileged, driver-level, or OS-internal integration. CrossPort
adds no UI for any of them: a control that cannot work is worse than an absent
one.

| Not current | Why |
| --- | --- |
| Native NTFS filesystem driver | CrossPort is an application, not a driver |
| Native NTFS write on macOS | Requires a driver; macOS mounts most NTFS volumes read-only and CrossPort can only report that |
| Mount / unmount | A driver-class or privileged operation |
| Automount | Same |
| Filesystem formatting | A driver-class or privileged operation |
| Filesystem repair | `fsck`-class work on filesystem structures, not on files CrossPort wrote |
| Kernel / system extension | Out of scope by design |
| Privileged filesystem helper | Out of scope by design |

See [`PARAGON_CAPABILITY_GAP.md`](PARAGON_CAPABILITY_GAP.md) for the full
comparison.

## FUTURE (not built; no dates promised)

Directions, not commitments. None of these exists yet.

| Future | Status |
| --- | --- |
| Linux / macOS production packaging and QA | Not started. The architecture is platform-neutral; no bundle is produced |
| Platform code signing | Not configured; there is no certificate in the repository |
| Update path / updater | Not built; there is no update checker and no update server |
| Folder synchronization, watched folders, batch operations, advanced transfer rules | Version 2 candidates in [`FEATURE_SPECIFICATION.md`](FEATURE_SPECIFICATION.md) |
| Wider verification coverage (metadata reapplication) | Not built. Today, modified times and attributes are not reapplied and every verdict says so |
| Byte-offset resume of a partial file | Deliberately refused today; would be a new engine capability |

## Platform-specific behaviour (already in the architecture)

These are not features to add; they are the differences the cross-platform
foundation already isolates behind explicit abstractions.

| Concern | Windows | Unix (Linux/macOS) |
| --- | --- | --- |
| Volume roots | Drive letters (`C:\`) | Conventional mount points (`/`, `/Volumes/...`) |
| Volume metadata | Win32 volume APIs | Mount-point discovery; unreported values stay `null` |
| Links | Reparse points and junctions reported, never followed | Symlinks reported, never followed |
| Permissions | Read-only attribute | POSIX mode bits |
| Case sensitivity | Insensitive comparison | Sensitive comparison |

Related: [`PLATFORM_SUPPORT.md`](PLATFORM_SUPPORT.md),
[`PARAGON_CAPABILITY_GAP.md`](PARAGON_CAPABILITY_GAP.md),
[`../../NON_GOALS.md`](../../NON_GOALS.md).
