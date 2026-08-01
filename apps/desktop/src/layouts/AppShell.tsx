/* ==========================================================================
 * AppShell layout
 * Application frame: sidebar, top bar, routed content, status bar.
 * ========================================================================== */

import { Outlet } from "react-router-dom";

import { Sidebar } from "./Sidebar";
import { StatusBar } from "./StatusBar";
import { TopBar } from "./TopBar";
import "./AppShell.css";

export function AppShell() {
  return (
    <div className="app-shell">
      <Sidebar />
      <div className="app-shell__frame">
        <TopBar />
        <main className="app-shell__main">
          <Outlet />
        </main>
        <StatusBar />
      </div>
    </div>
  );
}
