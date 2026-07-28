# Module Architecture

Version: 1.0

---

# Philosophy

Every package exists for one reason.

If a package gains multiple unrelated responsibilities, it should be split.

---

# Core

Coordinates application behavior.

Owns no platform-specific implementation.

---

# Filesystem

Provides a unified interface for filesystem operations.

Responsibilities:

- Drive discovery
- Directory traversal
- File metadata
- Permissions abstraction
- Filesystem capability detection

---

# Transfer

Responsible for:

- Copy
- Move
- Queue
- Resume
- Pause
- Retry
- Conflict handling

Transfer owns no UI.

---

# Verification

Responsible for validating completed transfers.

Supports:

- Metadata verification
- Checksum verification
- Future verification strategies

---

# History

Stores completed operations.

Responsible for:

- Recent transfers
- Transfer logs
- Retention policy

---

# Notifications

Produces user-visible notifications.

Never performs business logic.

---

# Updates

Checks for new releases.

Handles:

- Version comparison
- Release information
- User-controlled updates

No automatic installation without user approval.

---

# Platform

Contains operating-system-specific implementations.

Submodules:

- Windows
- macOS
- Linux

No business logic should exist here.

---

# Settings

Responsible only for configuration.

Settings never perform application logic.

---

# UI Kit

Reusable visual components.

Contains:

- Buttons
- Cards
- Dialogs
- Inputs
- Layout
- Navigation

UI Kit contains zero business logic.

---

# Shared

Reusable code used by multiple packages.

Examples:

- Constants
- Helpers
- Common interfaces
- Shared models

Shared must remain lightweight.

---

# Dependency Rules

Allowed:

UI

↓

Core

↓

Transfer

↓

Filesystem

↓

Platform

Forbidden:

Platform

↓

UI

Transfer

↓

UI

Filesystem

↓

Notifications

Settings

↓

Transfer

Circular dependencies are never permitted.
