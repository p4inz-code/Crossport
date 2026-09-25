> **Status: partial — Phase 1 foundation implemented.** Path normalization and validation (`src-tauri/src/filesystem/path.rs`), metadata inspection (`metadata.rs`), drive-root enumeration (`platform/drives.rs`), and the native folder picker (`commands/dialog.rs`) exist today. Traversal, capacity, filesystem types, and the transfer engine are later phases. Sections below that describe unimplemented behavior are planning material.
# CrossPort Filesystem Engine

Version: 2.0

Status: Approved

---

# Purpose

The Filesystem Engine provides a unified abstraction layer for interacting with different operating systems and filesystem formats.

Its purpose is to hide platform differences while maintaining reliable file operations.

---

# Implemented today (Phase 1)

| Capability | Module | Notes |
| --- | --- |
| Path validation | `filesystem/path.rs` | Must be absolute, no null bytes, `..` may not escape the root, `.` removed; resolved lexically with no filesystem access |
| Directory validation | `filesystem/path.rs` | `must_be_directory` maps missing paths to `path_not_found` and files to `path_not_directory` |
| Metadata access | `filesystem/metadata.rs` | Name, kind, symlink flag, size, modification time, read-only flag |
| Drive discovery | `platform/drives.rs` | Storage roots that are readable directories, deduplicated and sorted |
| Native dialog | `commands/dialog.rs` | Folder selection validated before it reaches the frontend |

Nothing in this layer mutates the filesystem.

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

Drive detection should provide:

- Name
- Path
- Size
- Available space
- Filesystem type
- Connection status

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