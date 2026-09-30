# Brand Guidelines

## Name

The product is called **CrossPort** (one word, capitals C and P). Avoid
"Crosport", "Cross-Port", or "Cross Port".

## Studio

CrossPort is a product of **P4inz Interactive Labs**. Use that exact form where a
product/company relationship is stated — the installer publisher, package
metadata, copyright lines, and release attribution. Do not invent a legal suffix
(Ltd, LLC, Inc, and so on), a registration number, a legal address, or a company
website. The studio name is not a substitute for the product name: copy about the
software says "CrossPort".

## Positioning

A fast, reliable desktop file-transfer utility: free, offline first, no ads, no
telemetry, no account. Windows is the only packaged and tested platform today;
the macOS and Linux work is application foundation only and must never be
described as support (`docs/product/PLATFORM_SUPPORT.md`). The longer-term
direction is in `docs/product/VISION.md`.

## Visual identity

- Colors come from the design tokens in `apps/desktop/src/styles/tokens/colors.css`
  (primary blue `#2563eb` in light theme).
- Typography: the `--font-sans` stack (Inter first).
- Iconography: lucide icons, 1.75 stroke default.
- The interface is calm, professional, and uncluttered — never flashy or
  gaming-styled.

## Logo

The mark is two arrows crossing in opposite directions on a solid rounded
field: one shape that says "files moving both ways between locations". It is
deliberately simple so it survives 16px, where the installer and taskbar draw
it, and it carries its own blue field so it stays legible on light and dark
backgrounds without a second version.

| File | Role |
| --- | --- |
| `assets/brand/crossport-logo.svg` | The canonical source. Every other asset is derived from it. |
| `apps/desktop/src-tauri/icons/` | The packaged application, installer, and taskbar icons. |
| `apps/desktop/public/favicon.svg` | The webview favicon and the browser-dev tab icon. |
| `apps/desktop/src/components/ui/BrandMark.tsx` | The in-app glyph (sidebar brand tile, headings). |

Regenerate the packaged icons from the source after any change to it:

```bash
pnpm --filter desktop exec tauri icon ../../assets/brand/crossport-logo.svg
rm -rf apps/desktop/src-tauri/icons/android apps/desktop/src-tauri/icons/ios
```

The command emits Android and iOS icon sets too; CrossPort ships no mobile
target, so those are removed. This keeps one logo in the repository rather than
a set that drifts apart.

Rules: do not recolor the mark, do not add text inside it, do not stretch it
(non-uniform scale), and do not place it on a background that fights its field.
The glyph inherits `currentColor` in-app so it works on any surface; the field
is drawn only where the mark is shown on its own.

## Voice

- Plain, honest language.
- Errors explain what happened, why, and what to do next
  (`docs/architecture/ERROR_HANDLING.md`).
- Never overpromise; feature copy reflects what is actually implemented.
