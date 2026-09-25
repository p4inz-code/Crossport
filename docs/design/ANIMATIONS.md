# Animations

Status: Approved

Motion is centralized in `apps/desktop/src/styles/tokens/transitions.css`.

## Durations

| Token | Duration |
| --- | --- |
| `--duration-fast` | 120ms |
| `--duration-normal` | 200ms |
| `--duration-slow` | 320ms |

## Easing

- `--ease-out`: cubic-bezier(0, 0, 0.2, 1)
- `--ease-in`: cubic-bezier(0.4, 0, 1, 1)
- `--ease-in-out`: cubic-bezier(0.4, 0, 0.2, 1)

## Composed transitions

- `--transition-fast`, `--transition-normal`, `--transition-slow`

## Rules

- Interactions animate only `background-color`, `border-color`, `color`,
  `box-shadow`, or `transform` — never layout properties.
- Hover/active/focus transitions use `--transition-fast`.
- The loading spinner is the only continuous animation and is disabled for
  users with `prefers-reduced-motion: reduce`.
