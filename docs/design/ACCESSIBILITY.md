# Accessibility

Status: Approved

Accessibility is part of the component contract, not a post-processing step.

## Implemented

- **Focus visibility** — `:focus-visible` outlines use `--color-focus-ring` at
  the global level and on interactive components.
- **Reduced motion** — `@media (prefers-reduced-motion: reduce)` collapses
  animation and transition durations to near-zero in `base.css`.
- **Semantic structure** — pages use `main`, sections use `h2`/`h3` headings
  inside `<section>`, and navigation is wrapped in `<nav aria-label>`.
- **ARIA** — decorative icons use `aria-hidden`; live loading indicators use
  `role="status"` + `aria-live="polite"`; the theme toggle exposes an
  `aria-label` describing the current state.
- **Contrast** — semantic color tokens are defined for both themes with
  text-on-surface contrast reviewed against the primary/muted/subtle tiers.

## Keyboard

The application is usable without a mouse, and the shortcuts are deliberately
few — nothing that changes what is on disk is reachable by a keystroke.

| Keys | What it does |
| --- | --- |
| `Ctrl`/`Cmd` + `1`…`6` | Moves between Home, Drives, Transfers, History, Recovery, and Settings |
| `Alt` + `←` / `→` | Walks back and forward through the folders that were visited |
| `Alt` + `↑` | Opens the parent folder |
| `↑` / `↓` | Moves the roving focus through the entries of the open folder |
| `Space` | Checks or unchecks the focused entry |
| `Enter` | Opens the focused folder |
| `Esc` | Closes the open dialog without starting anything |

Shortcuts are installed once by the shell (`useAppShortcuts`), are ignored
while the user is typing in a field or content-editable element, and require no
modifier other than `Ctrl`/`Cmd` (with `Alt` and `Shift` explicitly excluded) so
a combination can never be ambiguous.

## Dialogs

- A dialog takes focus when it opens, gives it back to whatever held it when it
  closes, and is the only thing on screen that is interactive while it is open.
- The safe answer receives focus, and `Esc` chooses it.
- Buttons that ask about work in flight state the consequence in the label
  ("Keep working", "Close anyway") rather than "OK" and "Cancel".

## Conventions

- Buttons default to `type="button"`.
- Icon buttons always carry a `title` and an `aria-label`, and two buttons in
  one dialog never share an accessible name.
- Text scales are rem-based and respect browser font-size settings.
