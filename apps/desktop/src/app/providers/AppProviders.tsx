/* ==========================================================================
 * AppProviders
 * Composition root for all global providers. Order matters:
 * ErrorBoundary > StoreProvider > ThemeProvider > routed application.
 * ========================================================================== */

import type { ReactNode } from "react";

import { ErrorBoundary } from "./ErrorBoundary";
import { StoreProvider } from "./StoreProvider";
import { ThemeProvider } from "./ThemeProvider";

interface AppProvidersProps {
  children: ReactNode;
}

export function AppProviders({ children }: AppProvidersProps) {
  return (
    <ErrorBoundary>
      <StoreProvider>
        <ThemeProvider>{children}</ThemeProvider>
      </StoreProvider>
    </ErrorBoundary>
  );
}
