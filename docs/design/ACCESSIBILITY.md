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

## Conventions

- Buttons default to `type="button"`.
- Icon buttons always carry a `title` and an `aria-label`.
- Text scales are rem-based and respect browser font-size settings.
