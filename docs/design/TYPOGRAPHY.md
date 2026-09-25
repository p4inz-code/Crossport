# Typography

Status: Approved

Typography tokens are defined in `apps/desktop/src/styles/tokens/typography.css`.

## Families

| Token | Stack |
| --- | --- |
| `--font-sans` | Inter, system-ui, -apple-system, Segoe UI, Roboto, Helvetica Neue, Arial, sans-serif |
| `--font-mono` | SFMono-Regular, ui-monospace, Cascadia Code, JetBrains Mono, Menlo, Consolas, monospace |

## Scale

| Token | Size |
| --- | --- |
| `--text-xs` | 12px |
| `--text-sm` | 13px |
| `--text-base` | 15px (body default) |
| `--text-md` | 16px |
| `--text-lg` | 18px |
| `--text-xl` | 20px |
| `--text-2xl` | 24px |
| `--text-3xl` | 30px |
| `--text-4xl` | 36px |

## Weights, leading, tracking

- Weights: `--font-regular` (400), `--font-medium` (500), `--font-semibold`
  (600), `--font-bold` (700).
- Leading: `--leading-none`, `--leading-tight`, `--leading-normal`,
  `--leading-relaxed`.
- Tracking: `--tracking-tight`, `--tracking-normal`, `--tracking-wide`.

## Usage conventions

- Page titles: `--text-2xl` + `--font-bold`.
- Card/section titles: `--text-md` + `--font-semibold`.
- Body text: `--text-base` with `--leading-normal`.
- Meta text: `--text-sm` or `--text-xs` with muted color tokens.
