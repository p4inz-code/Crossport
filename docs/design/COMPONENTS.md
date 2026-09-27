> **Status: partly implemented.** The repository implements Phases 1–5. What exists today is the UI primitives in `components/ui` (`Button`, `Card`/`CardHeader`/`CardBody`, `EmptyState`, `LoadingState`, `Notice`, `Section`) plus the feature components under `src/features/` and the shell components in `src/layouts/`. Everything else in this document is planned design.
# CrossPort UI Components

Version: 1.0

Status: Approved

---

# Purpose

Defines reusable interface components.

Components should provide consistency across the application.

---

# Component Philosophy

Components should be:

- Simple
- Reusable
- Accessible
- Theme compatible

---

# Core Components

## Button

Used for user actions.

Types:

- Primary
- Secondary
- Destructive
- Ghost

---

## Drive Card

Displays connected drives.

Information:

- Drive name
- Storage size
- Filesystem
- Status

---

## Transfer Card

Displays active transfers.

Information:

- File name
- Progress
- Speed
- Remaining time
- Status

Implemented in `src/features/transfers/components/TransferQueue.tsx`: one card
per job, in engine order, with the status (including `Verifying`), the counts and
bytes the engine reported, throughput and remaining time when it can measure
them, the items it skipped or failed, the verification verdict and what was not
preserved, the controls that apply to that job, and a link to its durable record
once it is finished. Progress is never invented: a job with no measurable
progress shows a working state instead of a percentage.

## Notification

Short status feedback about something that happened on another surface.

Implemented in `src/features/notifications/components/NotificationStack.tsx`:
one notification per (job, event), bounded to the newest five, dismissible
individually, and — when a surface holds what it is about — carrying the action
that opens it. Errors are announced assertively; everything else politely.

## Notice

An inline message inside a card or a page body, used when a state is not a
failure but still needs saying (a degraded history document, a truncated
listing, a caveat about metadata). `Notice` in `components/ui` carries the tone
and the icon so the same situation looks the same everywhere.

## Close Confirmation

Implemented in `src/layouts/CloseConfirmDialog.tsx`. The backend holds the
window close while work is in flight and says what would be set aside; this is
where the user answers it. It states the facts (jobs running, decisions
waiting), explains that files are written under a temporary name and renamed
only when complete, offers the safe answer first, and never chooses for the user.

---

## Progress Indicator

Used for:

- Transfers
- Verification
- Background tasks

Must clearly communicate state.

---

## Dialog

Used for:

- Confirmations
- Warnings
- Important decisions

Dialogs must not interrupt unnecessarily.

---

## Notification

Used for:

- Completion
- Errors
- Updates

Notifications should be actionable.

---

## Settings Panel

Contains:

- General settings
- Transfer settings
- Advanced options

---

# Component Rules

Components must:

- Have one purpose
- Support themes
- Support accessibility
- Avoid business logic

---

# Design Consistency

The same action should use the same component everywhere.

---

# Future Components

Possible additions:

- Transfer timeline
- Automation builder
- Plugin interface
- Advanced diagnostics

---

# Principle

Good components make complex applications feel simple.