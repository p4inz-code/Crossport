/* ==========================================================================
 * Class-name composition utility
 * ========================================================================== */

type ClassValue = string | number | false | null | undefined;

/**
 * Composes a list of class values into a single className string,
 * filtering out falsy values. Mirrors the clsx API without the dependency.
 */
export function cn(...values: ClassValue[]): string {
  return values.filter(Boolean).join(" ");
}
