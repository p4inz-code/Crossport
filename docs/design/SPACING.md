# Spacing

Status: Approved

The spacing scale is defined in `apps/desktop/src/styles/tokens/spacing.css`
as a 4px-based scale (`--space-*`).

## Scale

| Token | Rem | Pixels |
| --- | --- | --- |
| `--space-0` | 0 | 0 |
| `--space-0-5` | 0.125 | 2 |
| `--space-1` | 0.25 | 4 |
| `--space-1-5` | 0.375 | 6 |
| `--space-2` | 0.5 | 8 |
| `--space-2-5` | 0.625 | 10 |
| `--space-3` | 0.75 | 12 |
| `--space-4` | 1 | 16 |
| `--space-5` | 1.25 | 20 |
| `--space-6` | 1.5 | 24 |
| `--space-7` | 1.75 | 28 |
| `--space-8` | 2 | 32 |
| `--space-9` | 2.25 | 36 |
| `--space-10` | 2.5 | 40 |
| `--space-12` | 3 | 48 |
| `--space-14` | 3.5 | 56 |
| `--space-16` | 4 | 64 |
| `--space-20` | 5 | 80 |
| `--space-24` | 6 | 96 |

## Rules

- Use tokens for margin, padding, and gap — never raw values.
- Default component rhythm: `--space-4` for card headers, `--space-5` for card
  bodies, `--space-6` between page sections.
- Half steps (`-0-5`, `-1-5`, `-2-5`) are for fine-grained alignment.
