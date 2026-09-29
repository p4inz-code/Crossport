/* ==========================================================================
 * Sidebar component
 * Primary navigation rail. The routes come from the shared navigation table
 * and the icons from the map below, so a route can never appear in one place
 * without the other. Only implemented pages are listed.
 * ========================================================================== */

import type { LucideIcon } from "lucide-react";
import {
  ArrowRightLeft,
  HardDrive,
  History,
  Home,
  Settings,
  ShieldAlert,
} from "lucide-react";
import { NavLink } from "react-router-dom";

import { BrandMark } from "@/components/ui";
import { APP_ROUTES, cn } from "@/lib";
import { useAppStore, useRecoveryStore } from "@/stores";
import "./Sidebar.css";

/**
 * Icons for the routes the navigation table declares.
 *
 * The table owns the paths and labels; this map only says what each one looks
 * like. A route added to the table without an icon fails to compile, which is
 * the point.
 */
const ICONS: Record<string, LucideIcon> = {
  "/": Home,
  "/drives": HardDrive,
  "/transfers": ArrowRightLeft,
  "/history": History,
  "/recovery": ShieldAlert,
  "/settings": Settings,
};

export function Sidebar() {
  const appName = useAppStore((state) => state.name);
  const interrupted = useRecoveryStore((state) => state.candidates.length);

  return (
    <aside className="sidebar">
      <div className="sidebar__brand">
        <span className="sidebar__logo" aria-hidden="true">
          <BrandMark size={18} />
        </span>
        <span className="sidebar__brand-name">{appName}</span>
      </div>

      <nav className="sidebar__nav" aria-label="Primary">
        <ul className="sidebar__list">
          {APP_ROUTES.map((item) => {
            const Icon = ICONS[item.path] ?? Home;
            const end = item.path === "/";
            return (
              <li key={item.path}>
                <NavLink
                  to={item.path}
                  end={end}
                  title={`${item.label} (Ctrl+${item.key})`}
                  className={({ isActive }) =>
                    cn("sidebar__link", isActive && "sidebar__link--active")
                  }
                >
                  <Icon size={18} strokeWidth={1.75} aria-hidden="true" />
                  <span>{item.label}</span>
                  {item.path === "/recovery" && interrupted > 0 ? (
                    // The visible count is a digit; the status role gives it a
                    // name assistive tech can read and a change to announce.
                    <span
                      className="sidebar__badge"
                      role="status"
                      aria-label={`${interrupted} interrupted transfer${interrupted === 1 ? "" : "s"}`}
                    >
                      {interrupted}
                    </span>
                  ) : null}
                </NavLink>
              </li>
            );
          })}
        </ul>
      </nav>

      <div className="sidebar__footer">
        <p className="sidebar__hint">
          Ctrl+1…6 moves between pages. Alt+← and Alt+→ walk the folders you
          have visited.
        </p>
      </div>
    </aside>
  );
}
