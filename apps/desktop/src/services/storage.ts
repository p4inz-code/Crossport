/* ==========================================================================
 * Storage service
 * Typed, fail-safe wrapper around localStorage. All persistence in the app
 * goes through this service so storage failures never crash the UI.
 * ========================================================================== */

/**
 * Reads a JSON value from localStorage. Returns null when the key is missing
 * or the stored value cannot be parsed.
 */
export function readStorage<T>(key: string): T | null {
  try {
    const raw = window.localStorage.getItem(key);
    if (raw === null) {
      return null;
    }
    return JSON.parse(raw) as T;
  } catch (error) {
    console.error(`[storage] failed to read "${key}"`, error);
    return null;
  }
}

/** Writes a JSON value to localStorage. Failures are logged, never thrown. */
export function writeStorage<T>(key: string, value: T): void {
  try {
    window.localStorage.setItem(key, JSON.stringify(value));
  } catch (error) {
    console.error(`[storage] failed to write "${key}"`, error);
  }
}
