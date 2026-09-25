/* ==========================================================================
 * Storage service
 * Validated web fallback for settings persistence. Used only when the app
 * runs outside Tauri (browser dev mode); the desktop backend owns the
 * canonical copy of settings. Every value is validated by the settings
 * schema before it is returned, so corrupt data can never leak into state.
 * ========================================================================== */

import { STORAGE_KEYS } from "@/lib";
import { type AppSettings, DEFAULT_SETTINGS, settingsSchema } from "@/types";

/**
 * Reads settings from localStorage. Returns defaults when the key is missing,
 * the value is corrupt, or validation fails. Failures are logged, never
 * thrown — storage must not crash the UI.
 */
export function readSettingsFromLocal(): AppSettings {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEYS.settings);
    if (raw === null) {
      return DEFAULT_SETTINGS;
    }
    const parsed = settingsSchema.safeParse(JSON.parse(raw));
    if (!parsed.success) {
      console.warn(
        `[storage] stored settings are invalid; using defaults: ${parsed.error.message}`,
      );
      return DEFAULT_SETTINGS;
    }
    return parsed.data;
  } catch (error) {
    console.warn("[storage] failed to read settings; using defaults", error);
    return DEFAULT_SETTINGS;
  }
}

/**
 * Persists settings to localStorage. The value is validated before writing;
 * failures are logged, never thrown.
 */
export function writeSettingsToLocal(settings: AppSettings): void {
  try {
    const validated = settingsSchema.parse(settings);
    window.localStorage.setItem(
      STORAGE_KEYS.settings,
      JSON.stringify(validated),
    );
  } catch (error) {
    console.error("[storage] failed to persist settings", error);
  }
}
