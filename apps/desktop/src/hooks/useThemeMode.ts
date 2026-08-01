/* ==========================================================================
 * useThemeMode hook
 * Returns the current theme mode and the mode resolved against the OS
 * preference, plus a setter. The resolved theme is applied to the document
 * by the ThemeProvider.
 * ========================================================================== */

import { useSyncExternalStore } from "react";

import { useSettingsStore } from "@/stores";
import type { ThemeMode } from "@/types";

const THEME_QUERY = "(prefers-color-scheme: dark)";

function subscribe(callback: () => void): () => void {
  const media = window.matchMedia(THEME_QUERY);
  media.addEventListener("change", callback);
  return () => media.removeEventListener("change", callback);
}

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia(THEME_QUERY).matches ? "dark" : "light";
}

export function useThemeMode(): {
  mode: ThemeMode;
  resolvedTheme: "light" | "dark";
  setTheme: (theme: ThemeMode) => void;
} {
  const mode = useSettingsStore((state) => state.theme);
  const setTheme = useSettingsStore((state) => state.setTheme);

  const systemTheme = useSyncExternalStore(
    subscribe,
    getSystemTheme,
    getSystemTheme,
  );

  const resolvedTheme = mode === "system" ? systemTheme : mode;

  return { mode, resolvedTheme, setTheme };
}
