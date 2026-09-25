# Colors

Status: Approved

All colors flow from semantic tokens defined in
`apps/desktop/src/styles/tokens/colors.css`. Components never use raw hex
values.

## Token groups

| Group | Examples |
| --- | --- |
| Surfaces | `--color-bg`, `--color-surface`, `--color-surface-raised`, `--color-surface-subtle` |
| Borders | `--color-border`, `--color-border-strong` |
| Text | `--color-text`, `--color-text-muted`, `--color-text-subtle` |
| Brand | `--color-primary`, `--color-primary-hover`, `--color-primary-active`, `--color-primary-fg`, `--color-primary-subtle` |
| Status | `--color-success`, `--color-warning`, `--color-danger` (each with a `-subtle` variant) |
| Focus/overlay | `--color-focus-ring`, `--color-overlay` |
| Misc | `--color-scrollbar-thumb`, `--color-scrollbar-thumb-hover` |

## Theming

- `:root` defines the light theme.
- `[data-theme="dark"]` overrides the same variable names.
- `ThemeProvider` sets `data-theme` on `<html>` from the resolved setting
  (`useThemeMode`), which follows the OS preference when mode is `system`.

## Rules

- Never reference hex values in components — always use tokens.
- Use `-subtle` variants for tinted backgrounds (active nav, empty-state icons).
- Status colors distinguish success / warning / danger states; use the plain
  token for text and the `-subtle` token for backgrounds.
