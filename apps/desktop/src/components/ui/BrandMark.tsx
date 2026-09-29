/* ==========================================================================
 * BrandMark component
 * The CrossPort glyph: two arrows crossing in opposite directions. Strokes
 * with `currentColor` so the mark inherits the surface it sits on, and carries
 * no background of its own, so the same shape works on the sidebar's brand
 * tile, in a document heading, or on its own.
 *
 * The geometry is the same as `assets/brand/crossport-logo.svg`, which is the
 * source the packaged application icons are generated from. Keep the two in
 * step: this file is the in-app copy, that file is the shipped one.
 * ========================================================================== */

interface BrandMarkProps {
  /** Pixel size of the square the mark is drawn in. */
  size?: number;
  className?: string;
}

export function BrandMark({ size = 20, className }: BrandMarkProps) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 256 256"
      fill="none"
      stroke="currentColor"
      strokeWidth={22}
      strokeLinecap="round"
      strokeLinejoin="round"
      role="img"
      aria-label="CrossPort"
    >
      <path d="M72 102 H148 M144 76 L174 102 L144 128 M184 154 H108 M112 128 L82 154 L112 180" />
    </svg>
  );
}
