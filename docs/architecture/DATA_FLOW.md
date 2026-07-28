# CrossPort Data Flow

Version: 1.0

Status: Approved

---

# Purpose

This document defines how information moves through CrossPort.

---

# General Flow

```
User Action

↓

UI Layer

↓

Application Services

↓

Domain Modules

↓

Platform Layer

↓

Operating System
```

---

# Drive Detection Flow

```
Operating System

↓

Platform Module

↓

Filesystem Engine

↓

Core

↓

UI
```

The UI never directly queries the operating system.

---

# Transfer Flow

```
User Selects Files

↓

Transfer Request Created

↓

Validation

↓

Transfer Queue

↓

Transfer Engine

↓

Verification

↓

History Storage

↓

User Notification
```

---

# Settings Flow

```
User Changes Setting

↓

Settings Module

↓

Persistent Storage

↓

Affected Module Reads Configuration
```

---

# Event System

Modules communicate through events.

Examples:

```
TransferStarted

TransferProgress

TransferPaused

TransferCompleted

TransferFailed

DriveConnected

DriveRemoved
```

---

# Data Ownership

Each module owns its own data.

Examples:

Transfer owns:

- Transfer state

History owns:

- Past records

Settings owns:

- User preferences

---

# Forbidden Data Flow

Avoid:

- UI directly modifying storage
- Platform directly controlling UI
- Modules modifying other modules' internal state

---

# Principle

Information should move through predictable paths.

No hidden communication.