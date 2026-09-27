/* ==========================================================================
 * Root application component
 * Routes are lazy-loaded to keep the initial bundle small as pages grow.
 * Only implemented pages are routed.
 * ========================================================================== */

import type { ReactNode } from "react";
import { lazy, Suspense } from "react";
import { createHashRouter, RouterProvider } from "react-router-dom";

import { LoadingState } from "@/components/ui";
import { AppShell } from "@/layouts";
import { HomePage } from "./pages/HomePage";
import { AppProviders } from "./providers/AppProviders";

const DrivesPage = lazy(() =>
  import("@/features/drives").then((module) => ({
    default: module.DrivesPage,
  })),
);
const SettingsPage = lazy(() =>
  import("@/features/settings").then((module) => ({
    default: module.SettingsPage,
  })),
);
const TransfersPage = lazy(() =>
  import("@/features/transfers").then((module) => ({
    default: module.TransfersPage,
  })),
);
const HistoryPage = lazy(() =>
  import("@/features/history").then((module) => ({
    default: module.HistoryPage,
  })),
);
const RecoveryPage = lazy(() =>
  import("@/features/recovery").then((module) => ({
    default: module.RecoveryPage,
  })),
);

/** Wraps a lazy route with a Suspense fallback. */
function lazyElement(element: ReactNode): ReactNode {
  return <Suspense fallback={<LoadingState />}>{element}</Suspense>;
}

const router = createHashRouter([
  {
    path: "/",
    element: <AppShell />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "drives", element: lazyElement(<DrivesPage />) },
      { path: "transfers", element: lazyElement(<TransfersPage />) },
      { path: "history", element: lazyElement(<HistoryPage />) },
      { path: "recovery", element: lazyElement(<RecoveryPage />) },
      { path: "settings", element: lazyElement(<SettingsPage />) },
    ],
  },
]);

export function App() {
  return (
    <AppProviders>
      <RouterProvider router={router} />
    </AppProviders>
  );
}
