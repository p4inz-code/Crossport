# Platform Support

This document states exactly which platforms CrossPort ships on today and what
the application architecture is prepared for. It is deliberately conservative:
a platform is listed as supported only when a build actually exists and has been
tested.

## Current production platform

**Windows 10 1607+ or Windows 11, 64-bit.** This is the only packaged and tested
target. The release artifacts are the NSIS installer, the WiX MSI, and
`CrossPort.exe` (`docs/release/RELEASE_1.0.md`).

Requirements: the Microsoft Edge WebView2 runtime (included with Windows 11 and
current Windows 10; the installer bootstraps it when missing). Paths longer than
260 characters need Windows long-path support enabled; Windows refuses them
otherwise and CrossPort reports the failure.

## Cross-platform status

The application *architecture* is platform-neutral; the *product* is not yet
shipped on anything but Windows. Concretely:

- The backend derives every operating-system fact from one module
  (`apps/desktop/src-tauri/src/platform/`). The GUI layer, the transfer engine,
  persistence, and logging contain no Windows-only code paths.
- The `Platform` enum already models `windows`, `macos`, `linux`, and `other`,
  and the frontend reads the host platform from the backend over IPC rather than
  assuming one.
- Platform-specific behaviour is isolated behind explicit abstractions and
  `#[cfg]` gates: drive/volume enumeration, reparse-point detection, the
  disk-full error code, the fatal-error message box, and the test-only memory
  measurement. Each has a working non-Windows arm.

What is platform-specific today:

| Concern | Windows | Unix (Linux/macOS) |
| --- | --- | --- |
| Volume roots | Drive letters (`C:\`) via `GetLogicalDrives` | Conventional mount points (`/`, `/Volumes/...`) |
| Volume metadata | Win32 volume APIs (`GetDriveTypeW`, `GetDiskFreeSpaceExW`, `GetVolumeInformationW`) | Mount-point discovery; metadata the platform does not report stays `null` |
| Links / junctions | Reparse points (symlinks **and** junctions) reported, never followed | Symlinks reported, never followed |
| Filesystem permissions | Read-only flag from attributes | POSIX mode bits |
| App data | `%APPDATA%`, `%LOCALAPPDATA%` via Tauri's path resolver | XDG paths via the same resolver |
| Dialogs | Native folder picker hosted in Rust (`tauri-plugin-dialog`) | The same plugin; its native backend differs per platform |
| Case sensitivity | Case-insensitive path comparison | Case-sensitive comparison |

## Linux

**Status: foundation only. Not built, not tested, not supported.**

The abstractions above can carry a Linux target: mount points instead of drive
letters, POSIX permissions, symlinks, case-sensitive paths, XDG app directories,
and the native dialog plugin all have a place to live without changing the
engine. No Linux artifact is produced by this repository, no Linux CI job builds
a bundle, and no Linux distribution path is claimed.

## macOS

**Status: foundation only. Not built, not tested, not supported.**

POSIX paths, mount points under `/Volumes`, APFS/HFS+ volumes, permissions,
symlinks, case-insensitivity on the default volume, and application-support
paths are all handled by the same abstractions. No macOS bundle is produced.

**Important:** CrossPort does **not** provide native NTFS write access on macOS.
macOS mounts most NTFS volumes read-only, and CrossPort can only report that —
it operates on filesystems the operating system has already mounted, using the
permissions the operating system granted. Writing to a read-only NTFS volume
requires a filesystem driver, which is out of scope. See
`docs/product/PARAGON_CAPABILITY_GAP.md`.

## What a real Linux or macOS release would still require

1. A platform build job that produces and signs a bundle (`.dmg`/`.app`,
   `.deb`/`.AppImage`), which does not exist yet.
2. Platform QA of the surfaces that are only exercised on Windows today: the
   native folder picker, window chrome and close guard, notification
   presentation, and volume metadata for that platform's filesystem types.
3. Platform-specific documentation and install instructions.

Until those exist, this document — and every user-facing claim — says Windows
only. Adding a Linux or macOS icon set, a `cfg` arm, or a build file is not a
release.
