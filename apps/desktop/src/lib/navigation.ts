/* ==========================================================================
 * Navigation table
 * The routes the application offers, in the order they are shown, as plain data
 * with no framework imports.
 *
 * Three surfaces read this: the sidebar renders it, the top bar names the
 * current section from it, and the keyboard shortcuts jump by it. Keeping one
 * table is what stops them drifting apart — a route that exists in one place
 * and not the others cannot happen here.
 * ========================================================================== */

export interface AppRoute {
  /** Route path as React Router declares it. */
  path: string;
  /** What the sidebar and top bar call it. */
  label: string;
  /** Digit completing the `Ctrl`/`Cmd` journey to this route. */
  key: string;
}

export const APP_ROUTES: AppRoute[] = [
  { path: "/", label: "Home", key: "1" },
  { path: "/drives", label: "Drives", key: "2" },
  { path: "/transfers", label: "Transfers", key: "3" },
  { path: "/history", label: "History", key: "4" },
  { path: "/recovery", label: "Recovery", key: "5" },
  { path: "/settings", label: "Settings", key: "6" },
];

/** The section a pathname belongs to, for the top bar. */
export function routeLabel(pathname: string): string {
  if (pathname === "/") {
    return "Home";
  }
  const match = APP_ROUTES.find(
    (route) =>
      route.path !== "/" &&
      (pathname === route.path || pathname.startsWith(`${route.path}/`)),
  );
  return match?.label ?? "Not found";
}
