> **Status: read paths and the transfer engine implemented.** Path normalization and validation (`src-tauri/src/filesystem/path.rs`), metadata inspection (`metadata.rs`), single-directory listing (`directory.rs`), ancestor resolution for breadcrumbs (`ancestors.rs`), volume detection with classification and capacity (`platform/volume.rs`, `platform/drives.rs`), and the native folder picker (`commands/dialog.rs`) exist today. The transfer engine builds on this layer for recursive copy/move (`src-tauri/src/transfer/`) and verifies what it wrote (`src-tauri/src/verification/`). Sections below that describe unimplemented behavior are planning material.
# CrossPort Filesystem Engine

Version: 2.0

Status: Approved

---

# Purpose

The Filesystem Engine provides a unified abstraction layer for interacting with different operating systems and filesystem formats.

Its purpose is to hide platform differences while maintaining reliable file operations.

---

# Implemented today

| Capability | Module | Notes |
| --- | --- |
| Path validation | `filesystem/path.rs` | Must be absolute, no null bytes, `..` may not escape the root, `.` removed; resolved lexically with no filesystem access |
| Directory validation | `filesystem/path.rs` | `must_be_directory` maps missing paths to `path_not_found` and files to `path_not_directory` |
| Metadata access | `filesystem/metadata.rs` | Name, kind, symlink flag, size, modification time, read-only flag |
| Volume model | `platform/volume.rs` | Volume kind, name, filesystem, total/free/used capacity, read-only flag, mounted status; facts the platform does not report stay unknown |
| Volume detection | `platform/drives.rs` | Candidates from the platform, probed on Windows through `GetDriveTypeW` / `GetDiskFreeSpaceExW` / `GetVolumeInformationW`, deduplicated and sorted |
| Directory listing | `filesystem/directory.rs` | One directory per request, typed entries (name, path, kind, size, modified, read-only), directories before files, truncated at 10,000 entries, symlinks reported but never followed |
| Path ancestors | `filesystem/ancestors.rs` | The path and every directory above it, oldest first, each step validated and labeled — what the breadcrumb trail is built from, so the frontend never assembles a path itself |
| Native dialog | `commands/dialog.rs` | Folder selection validated before it reaches the frontend |

Nothing in the browsing path mutates the filesystem and no browsing operation recurses: the browser lists one directory at a time. Tree walking and every write live in the transfer engine, which consumes this layer's path and safety helpers instead of duplicating them.

The browser keeps a bounded back/forward history in the frontend (`src/stores/browser-store.ts`), but every path it navigates to is one the backend produced — a listing's own path, a parent the backend reported, or an ancestor the backend resolved. A trail that cannot be read leaves the crumbs empty rather than failing the folder that was opened.

---

# Responsibilities

The Filesystem Engine handles:

- Drive discovery
- Filesystem detection
- File metadata
- Directory traversal
- Permissions information
- Capability detection

---

# Non-Responsibilities

The Filesystem Engine does not handle:

- Transfer execution
- User interface
- Notifications
- Update management

---

# Platform Support

The engine must support:

- Windows
- macOS
- Linux

Platform-specific behavior must remain isolated.

---

# Filesystem Abstraction

CrossPort should not assume every filesystem behaves identically.

The engine must expose capabilities.

Example:

```
Filesystem

├── Read Support
├── Write Support
├── Permission Support
├── Metadata Support
└── Advanced Features
```

---

# Drive Discovery

Volume detection provides, where the platform reports it:

- Name (Windows volume label)
- Path (mount root)
- Size (total capacity)
- Available space (free bytes; used bytes derived only when both are consistent)
- Filesystem type
- Connection status (`mounted`)
- Volume kind (fixed, removable, network, optical, RAM, unknown)
- Read-only flag

Anything the platform cannot report is returned as unknown rather than guessed
at, and a volume type that cannot be classified (for example a mount point on
Linux) is reported as `unknown`.

---

# Safety Rules

The Filesystem Engine must:

- Never modify files without instruction
- Never silently change permissions
- Never format drives
- Never delete user data

---

# Performance

Requirements:

- Efficient scanning
- Lazy loading where possible
- Avoid unnecessary filesystem calls

---

# Future Support

Architecture should allow future support for:

- Additional filesystem formats
- Network locations
- Cloud-mounted drives
- External devices

without redesigning the application.

---

# Principle

The filesystem layer should make complex systems feel simple without hiding important safety information.