/* ==========================================================================
 * Settings store
 * Holds user preferences (theme, locale) with localStorage persistence.
 * ========================================================================== */

import { create } from "zustand";

import { STORAGE_KEYS } from "@/lib";
import { readStorage, writeStorage } from "@/services";
import {
  type AppSettings,
  DEFAULT_SETTINGS,
  settingsSchema,
  type ThemeMode,
} from "@/types";

interface SettingsState extends AppSettings {
  setTheme: (theme: ThemeMode) => void;
  setLocale: (locale: string) => void;
  /** Loads persisted settings from storage, falling back to defaults. */
  hydrate: () => void;
}

function persist(settings: AppSettings): void {
  writeStorage(STORAGE_KEYS.settings, settings);
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  ...DEFAULT_SETTINGS,

  setTheme: (theme) => {
    set({ theme });
    persist({ theme, locale: get().locale });
  },

  setLocale: (locale) => {
    set({ locale });
    persist({ theme: get().theme, locale });
  },

  hydrate: () => {
    const stored = readStorage<AppSettings>(STORAGE_KEYS.settings);
    const parsed = settingsSchema.safeParse(stored);
    const settings: AppSettings = parsed.success
      ? parsed.data
      : DEFAULT_SETTINGS;
    set(settings);
  },
}));
