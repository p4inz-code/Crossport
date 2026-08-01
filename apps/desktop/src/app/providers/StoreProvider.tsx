/* ==========================================================================
 * StoreProvider
 * Wires Zustand stores into the application tree. Each store is created at
 * module scope; this provider is the single composition point for store-side
 * bootstrapping (hydration, subscriptions) as features land.
 * ========================================================================== */

import type { ReactNode } from "react";
import { useEffect } from "react";

import { useSettingsStore } from "@/stores";

interface StoreProviderProps {
  children: ReactNode;
}

export function StoreProvider({ children }: StoreProviderProps) {
  // Hydrate persisted settings once after mount.
  useEffect(() => {
    useSettingsStore.getState().hydrate();
  }, []);

  return <>{children}</>;
}
