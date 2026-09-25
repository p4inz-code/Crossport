/* ==========================================================================
 * Settings store
 * Holds user preferences (theme, locale). Hydration and persistence go
 * through the settings service: the backend owns the canonical copy inside
 * Tauri, and validated localStorage is the web fallback.
 *
 * Local validation runs before the backend is called, so an invalid value
 * never reaches the IPC boundary and the UI can report the failure with the
 * same structured error shape the backend would have used.
 * ========================================================================== */

import { create } from "zustand";

import { IpcError, toIpcError } from "@/services/ipc";
import { getSettings, updateSettings } from "@/services/settings-service";
import {
  type AppSettings,
  DEFAULT_SETTINGS,
  type LoadStatus,
  type SaveStatus,
  settingsSchema,
  type ThemeMode,
} from "@/types";

interface SettingsState extends AppSettings {
  /** Hydration state for the settings document. */
  status: LoadStatus;
  /** Last hydration or write failure; `null` once a write succeeds. */
  error: IpcError | null;
  /** Write state, surfaced by the settings page. */
  saveStatus: SaveStatus;
  setTheme: (theme: ThemeMode) => void;
  setLocale: (locale: string) => void;
  /** Loads persisted settings, falling back to defaults. */
  hydrate: () => Promise<void>;
}

export const useSettingsStore = create<SettingsState>((set, get) => {
  /** Persists a validated settings document. */
  async function persist(next: AppSettings): Promise<void> {
    set({ saveStatus: "saving", error: null });
    try {
      await updateSettings(next);
      set({ saveStatus: "saved", error: null });
    } catch (error) {
      const ipcError = toIpcError(error);
      console.error("[settings] failed to persist settings", ipcError);
      set({ saveStatus: "error", error: ipcError });
    }
  }

  /** Validates a change before it is applied to state. */
  function apply(next: AppSettings): boolean {
    const validated = settingsSchema.safeParse(next);
    if (!validated.success) {
      set({
        saveStatus: "error",
        error: new IpcError("invalid_input", validateMessage(next)),
      });
      return false;
    }
    return true;
  }

  return {
    ...DEFAULT_SETTINGS,
    status: "idle",
    error: null,
    saveStatus: "idle",

    setTheme: (theme) => {
      const next: AppSettings = { theme, locale: get().locale };
      if (!apply(next)) {
        return;
      }
      set({ theme });
      void persist(next);
    },

    setLocale: (locale) => {
      const next: AppSettings = { locale: locale.trim(), theme: get().theme };
      if (!apply(next)) {
        return;
      }
      set({ locale: next.locale });
      void persist(next);
    },

    hydrate: async () => {
      set({ status: "loading", error: null });
      try {
        const settings = await getSettings();
        set({ ...settings, status: "ready", error: null });
      } catch (error) {
        const ipcError = toIpcError(error);
        console.warn("[settings] hydration failed; using defaults", ipcError);
        set({ ...DEFAULT_SETTINGS, status: "error", error: ipcError });
      }
    },
  };
});

/** Human-readable reason for a rejected settings document. */
function validateMessage(candidate: AppSettings): string {
  const result = settingsSchema.safeParse(candidate);
  if (result.success) {
    return "Settings are invalid.";
  }
  return result.error.issues
    .map((issue) => `${issue.path.join(".")} ${issue.message}`)
    .join("; ");
}
