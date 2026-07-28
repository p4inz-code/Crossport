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