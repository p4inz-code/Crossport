# Paragon Capability Gap

CrossPort is an **application**. Paragon's NTFS products are **filesystem
drivers**. The two look adjacent from a user's seat — both appear to "deal with
foreign volumes" — but they sit on opposite sides of a hard line: a driver
teaches the operating system to read and write a filesystem it does not
understand; an application can only use the filesystems the operating system has
already mounted, with the access it has already granted.

This document states that line plainly so no surface of CrossPort overpromises.
It compares capability *categories*, not feature lists, and notes where each
capability would have to be implemented.

## The two technologies

- **Driver-class products** (e.g. Microsoft NTFS for Mac by Paragon Software)
  install a kernel or system extension. Their advertised capability is to
  *mount, unmount, verify, format, and read/write* Microsoft NTFS volumes on
  macOS, including volumes macOS would otherwise mount read-only or not mount at
  all. That is possible only because the driver is privileged code in the
  filesystem stack.
- **CrossPort** is a user-space Tauri application. It copies and moves files on
  volumes the operating system has mounted, verifies what it wrote, records
  history, and recovers interrupted work. It never loads a driver, never mounts
  or unmounts a volume, and never formats or repairs a filesystem.

Some of Paragon's capabilities are impossible for CrossPort *in principle*
without becoming a driver product. That is a deliberate scope decision, not a
missing feature.

## Capability matrix

### SUPPORTED (CrossPort provides this today)

| Capability | Notes |
| --- | --- |
| Copy files and folders between mounted volumes | Recursive, with a reviewed dry run |
| Move files and folders between mounted volumes | Same-volume move is a single rename; cross-volume copies then verifies then deletes the source |
| Browse mounted volumes and directories | Rust-validated paths, no recursion during listing |
| Conflict handling | Replace / Skip (default) / Rename |
| Post-transfer verification | Size by default; optional SHA-256 computed while copying |
| Transfer history | Durable, bounded, per-record verdict |
| Crash recovery of interrupted transfers | Classified outcomes; never restarted automatically |
| Report a volume's read-only state | Reported from filesystem metadata; see below |

### PARTIALLY SUPPORTED

| Capability | What CrossPort does | What it cannot do |
| --- | --- | --- |
| Volume metadata | Reports kind, filesystem, capacity, read-only, mounted — but only where the platform actually reports it; unknown stays `null` | Invent facts a platform does not expose |
| Volume "verification" | Verifies files it wrote (size / SHA-256) | Verify filesystem structures (that is `fsck`-class work) |
| Read-only volumes | Detects and reports a read-only mount honestly | Make a read-only mount writable |
| Long paths, Unicode names | Handles them as far as the host filesystem allows | Exceed the host's own limits |
| Mounted-state changes | Reports a volume that is unavailable | Remount or reattach it |

### PLATFORM-SPECIFIC

| Behaviour | Difference |
| --- | --- |
| Volume roots | Drive letters on Windows; mount points on Unix |
| Links | Reparse points and junctions (Windows) vs symlinks (Unix) — reported, never followed |
| Permissions | Windows attributes vs POSIX mode bits |
| Case sensitivity | Insensitive on Windows, sensitive on typical Unix filesystems |

### NOT SUPPORTED (and not attempted)

These require privileged, driver-level, or OS-internal integration. CrossPort
adds no UI for any of them — a control that cannot work is worse than an absent
one.

- Native NTFS **write** access on macOS beyond what macOS mounts.
- Mounting or unmounting volumes; automount.
- Formatting or partitioning volumes.
- Filesystem integrity checks, repair, or verification of filesystem structures.
- Reading filesystems the operating system does not understand.
- Any kernel extension, system extension, or privileged helper.

### FUTURE DRIVER / PLATFORM WORK

Reaching those capabilities would be a new product surface, not a CrossPort
feature request: a signed driver or helper plus a privileged mount-management
service, installed with administrator rights and maintained per platform and per
filesystem. It is explicitly **out of scope** for CrossPort, and no roadmap item
claims it. Any future work in this area would be documented as such before it
shipped.

## What this means for the product

CrossPort's promise is narrower and honest: **safe, fast, simple file access and
transfer across the volumes your operating system has mounted** — with a review
before anything moves, proof of what landed, and a real recovery path when
something is interrupted. It does not promise a filesystem it cannot reach.

Related: `docs/product/PLATFORM_SUPPORT.md`, `NON_GOALS.md`,
`docs/product/VISION.md`.
