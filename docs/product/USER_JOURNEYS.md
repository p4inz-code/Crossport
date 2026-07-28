# CrossPort User Journeys

Version: 1.0

Status: Approved

---

# Purpose

This document defines how users interact with CrossPort.

The goal is to design workflows that are simple for beginners while remaining powerful for advanced users.

---

# Primary User Journey

## Scenario

A user has files on one operating system and needs them on another.

Example:

A teacher has project files on a Mac drive and needs to access them on Windows.

---

# Step 1 — Open CrossPort

Expected behavior:

- Application opens quickly.
- No account required.
- No unnecessary setup.
- No advertisements.

User immediately sees available drives.

---

# Step 2 — Detect Drives

CrossPort displays:

- Internal drives
- External drives
- Removable media

Drives should have clear labels.

Example:

```
External SSD
1 TB
NTFS
Connected
```

---

# Step 3 — Choose Transfer

Primary action:

Smart Copy

User selects:

Source

↓

Destination

↓

Files

---

# Step 4 — Review

Before starting:

CrossPort shows:

- Source location
- Destination location
- Estimated size
- Potential issues

The user understands what will happen.

---

# Step 5 — Transfer

During transfer:

User can:

- Pause
- Resume
- Cancel

Interface displays:

- Progress
- Current file
- Remaining items

---

# Step 6 — Completion

After completion:

CrossPort shows:

- Successful files
- Failed files
- Verification status

---

# Error Journey

Errors should never end the workflow.

Every error includes:

## What happened

Example:

"The drive disconnected."

## Why

Example:

"The external drive was removed during transfer."

## Next action

Example:

"Reconnect the drive to continue."

---

# Advanced User Journey

Advanced users can enable Advanced Mode.

Additional controls:

- Transfer settings
- Verification options
- Detailed logs
- Performance controls

Advanced features never appear in the default experience.

---

# Design Principle

The shortest path to success should always be the default path.
