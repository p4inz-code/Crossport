/* ==========================================================================
 * StoreProvider
 * Wires Zustand stores into the application tree. Each store is created at
 * module scope; this provider is the single composition point for store-side
 * bootstrapping.
 * ========================================================================== */

import type { ReactNode } from "react";
import { useEffect } from "react";

import { useSettingsStore, useSystemStore } from "@/stores";

interface StoreProviderProps {
  children: ReactNode;
}

export function StoreProvider({ children }: StoreProviderProps) {
  // Hydrate persisted settings and platform facts once after mount. Drive
  // enumeration is deliberately left to the drives page so startup stays
  // cheap.
  useEffect(() => {
    void useSettingsStore.getState().hydrate();
    void useSystemStore.getState().hydrate();
  }, []);

  return <>{children}</>;
}
