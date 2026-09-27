/* ==========================================================================
 * Routed page test helper
 * Pages call `useNavigate`, so a test has to put a router above them — exactly
 * as the application does. Rendering through `MemoryRouter` keeps the tests
 * exercising the real page instead of stubbing the router out.
 * ========================================================================== */

import { type RenderResult, render } from "@testing-library/react";
import type { ReactElement } from "react";
import { MemoryRouter } from "react-router-dom";

/**
 * The router a routed page needs, as an element.
 *
 * Exported so a test that re-renders the same tree can wrap it again:
 * `rerender(withRouter(<TransfersPage />))`.
 */
export function withRouter(
  ui: ReactElement,
  initialEntries: string[] = ["/"],
): ReactElement {
  return <MemoryRouter initialEntries={initialEntries}>{ui}</MemoryRouter>;
}

/** Renders a routed page inside an in-memory router. */
export function renderPage(
  ui: ReactElement,
  initialEntries: string[] = ["/"],
): RenderResult {
  return render(withRouter(ui, initialEntries));
}
