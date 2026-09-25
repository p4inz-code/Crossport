/* ==========================================================================
 * Display formatting helpers
 * Small, dependency-free formatters used by pages that render backend values.
 * ========================================================================== */

const UNITS = ["B", "KB", "MB", "GB", "TB", "PB"] as const;

/** Formats a byte count for display, e.g. `2048` → `2 KB`. */
export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) {
    return "Unknown";
  }

  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }

  const rounded = unit === 0 ? value : Math.round(value * 10) / 10;
  return `${rounded} ${UNITS[unit]}`;
}

/** Formats a backend millisecond timestamp, or `Unknown` when absent. */
export function formatDateTime(milliseconds: number | null): string {
  if (milliseconds === null || !Number.isFinite(milliseconds)) {
    return "Unknown";
  }
  return new Date(milliseconds).toLocaleString();
}
