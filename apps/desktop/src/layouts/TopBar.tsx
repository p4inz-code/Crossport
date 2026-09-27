/* ==========================================================================
 * TopBar component
 * Application header with a theme-mode toggle.
 * ========================================================================== */

import { Monitor, Moon, Sun } from "lucide-react";
import { useLocation } from "react-router-dom";

import { useThemeMode } from "@/hooks";
import { APP_NAME, routeLabel } from "@/lib";
import { THEME_MODES, type ThemeMode } from "@/types";
import "./TopBar.css";

const THEME_META: Record<ThemeMode, { label: string; icon: typeof Sun }> = {
  light: { label: "Light", icon: Sun },
  dark: { label: "Dark", icon: Moon },
  system: { label: "System", icon: Monitor },
};

export function TopBar() {
  const { mode, setTheme } = useThemeMode();
  // The section the user is actually in, so the bar never shows a stale label.
  const section = routeLabel(useLocation().pathname);

  function cycleTheme() {
    const next =
      THEME_MODES[(THEME_MODES.indexOf(mode) + 1) % THEME_MODES.length];
    setTheme(next);
  }

  const ThemeIcon = THEME_META[mode].icon;

  return (
    <header className="topbar">
      <div className="topbar__title">
        <span className="topbar__app-name">{APP_NAME}</span>
        <span className="topbar__separator" aria-hidden="true">
          /
        </span>
        <span className="topbar__context">{section}</span>
      </div>

      <button
        type="button"
        className="topbar__theme-toggle"
        onClick={cycleTheme}
        title={`Theme: ${THEME_META[mode].label}`}
        aria-label={`Switch theme (current: ${THEME_META[mode].label})`}
      >
        <ThemeIcon size={18} strokeWidth={1.75} aria-hidden="true" />
      </button>
    </header>
  );
}
