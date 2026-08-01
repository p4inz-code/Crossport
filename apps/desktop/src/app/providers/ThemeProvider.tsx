/* ==========================================================================
 * ThemeProvider
 * Applies the resolved theme to the document root. Theme state lives in the
 * settings store; this provider owns the DOM side of theming only.
 * ========================================================================== */

import type { ReactNode } from "react";
import { useEffect } from "react";

import { useThemeMode } from "@/hooks";

interface ThemeProviderProps {
  children: ReactNode;
}

export function ThemeProvider({ children }: ThemeProviderProps) {
  const { resolvedTheme } = useThemeMode();

  // Keep the document data-theme attribute in sync with the resolved theme.
  useEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme;
  }, [resolvedTheme]);

  return <>{children}</>;
}
