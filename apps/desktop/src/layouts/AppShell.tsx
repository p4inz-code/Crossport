/* ==========================================================================
 * AppShell layout
 * Application frame: sidebar, top bar, routed content, status bar.
 * ========================================================================== */

import { Outlet } from "react-router-dom";

import { NotificationStack } from "@/features/notifications";
// Imported from the component module rather than the feature barrel: the shell
// needs the banner, not the page, and going through the barrel would pull the
// lazy Recovery route into the initial bundle.
import { RecoveryBanner } from "@/features/recovery/components/RecoveryBanner";
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
          <RecoveryBanner />
          <Outlet />
        </main>
        <StatusBar />
      </div>
      <NotificationStack />
    </div>
  );
}
