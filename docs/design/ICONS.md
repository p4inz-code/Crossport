# Icons

Status: Approved

Icons come from [lucide-react](https://lucide.dev/), imported per-icon so the
bundle stays tree-shaken.

## Conventions

- Import icons directly: `import { HardDrive } from "lucide-react"`.
- Decorative icons render with `aria-hidden="true"`.
- Default stroke width is `1.75` for UI chrome; heavier weights (2+) only for
  emphasis.
- Icon buttons pair the icon with a `title` and `aria-label`.

## Current usage

| Icon | Use |
| --- | --- |
| `HardDrive` | Brand mark, drives navigation, drive list entries |
| `Home`, `Settings` | Sidebar navigation |
| `RefreshCw` | Refresh actions |
| `FolderSearch` | Native folder picker action |
| `Sun`, `Moon`, `Monitor` | Theme toggle states |
| `Loader2` | Loading indicator (spinning) |
| `Check` | Confirmed write |
| `AlertTriangle` | Error states and error boundary fallback |
| `ShieldCheck`, `Activity`, `Layers`, `Cpu`, `FolderTree` | Home page foundation cards |
