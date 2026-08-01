/* ==========================================================================
 * Sidebar component
 * Primary navigation rail. Nav items and their icons are defined together
 * here so adding a route can never leave an icon missing.
 * ========================================================================== */

import type { LucideIcon } from "lucide-react";
import {
  ArrowLeftRight,
  HardDrive,
  History,
  Home,
  Settings,
} from "lucide-react";
import { NavLink } from "react-router-dom";

import { cn } from "@/lib";
import { useAppStore } from "@/stores";
import "./Sidebar.css";

interface NavItem {
  to: string;
  label: string;
  icon: LucideIcon;
  /** Whether the link matches only when the path equals `to` exactly. */
  end?: boolean;
}

const NAV_ITEMS: NavItem[] = [
  { to: "/", label: "Home", icon: Home, end: true },
  { to: "/drives", label: "Drives", icon: HardDrive },
  { to: "/transfers", label: "Transfers", icon: ArrowLeftRight },
  { to: "/history", label: "History", icon: History },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  const appName = useAppStore((state) => state.name);

  return (
    <aside className="sidebar">
      <div className="sidebar__brand">
        <span className="sidebar__logo" aria-hidden="true">
          <HardDrive size={20} strokeWidth={1.75} />
        </span>
        <span className="sidebar__brand-name">{appName}</span>
      </div>

      <nav className="sidebar__nav" aria-label="Primary">
        <ul className="sidebar__list">
          {NAV_ITEMS.map((item) => {
            const Icon = item.icon;
            return (
              <li key={item.to}>
                <NavLink
                  to={item.to}
                  end={item.end}
                  className={({ isActive }) =>
                    cn("sidebar__link", isActive && "sidebar__link--active")
                  }
                >
                  <Icon size={18} strokeWidth={1.75} aria-hidden="true" />
                  <span>{item.label}</span>
                </NavLink>
              </li>
            );
          })}
        </ul>
      </nav>

      <div className="sidebar__footer">
        <p className="sidebar__hint">Application foundation ready</p>
      </div>
    </aside>
  );
}
