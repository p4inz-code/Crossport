> **Status: roadmap: partly implemented.** This document describes the planned product scope; the repository currently implements Phases 1-5 (settings, platform volumes, filesystem browsing with a breadcrumb trail and back/forward/up, the transfer engine, post-transfer verification, crash recovery, transfer history, and the product experience around them: composer, queue, history and recovery surfaces, notifications, keyboard support, and the close guard). Version 2 and later candidates are not started.
# Feature specification

Version: 1.0

Status: Living Document

---

# Product scope

CrossPort focuses on safe, reliable, cross-platform file movement.

Features are accepted only if they strengthen this mission.

---

# Version 1 features

## Smart copy

Guided file transfer between supported operating systems.

---

## Drag and drop

Support:

- Files
- Folders
- Drives

---

## Transfer queue

Support multiple queued transfers.

---

## Pause and resume

Transfers should recover whenever possible.

---

## Retry failed files

Retry individual failures without restarting the entire transfer.

---

## Smart verification

Optional verification after transfer completion.

Verification methods may evolve in future versions.

---

## History

Maintain transfer history.

Retention period is user configurable.

---

## Friendly errors

Every error should explain:

- What happened
- Why
- What to do next

---

## Connected drives

Automatically detect available drives.

Organize by category.

Examples:

- Internal
- External
- Network (future)

---

## Settings

Support:

- Theme
- Update preferences
- Verification
- History retention
- Advanced Mode

---

# Version 2 candidates

- Folder synchronization
- Watch folders
- Batch operations
- Keyboard shortcuts
- Advanced transfer rules

---

# Version 3 candidates

- Automation
- Scheduling
- Background service
- CLI companion

---

# Version 4 candidates

- Optional crash reporting
- Enterprise deployment improvements
- Expanded filesystem support

---

# Version 5 candidates

- Plugin architecture
- Community extensions
- Advanced workflow automation

---

# Acceptance requirements

Every feature must satisfy all of the following:

Improves user workflow

Maintains simplicity

Preserves reliability

Fits the product mission

Can be tested independently

Does not introduce unnecessary complexity

---

# Rejection Criteria

Features should be rejected if they:

- Duplicate operating system functionality
- Expand outside the product mission
- Reduce usability
- Increase maintenance without meaningful user value
- Require internet for core workflows
