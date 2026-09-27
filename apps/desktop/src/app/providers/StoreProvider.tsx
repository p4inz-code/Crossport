/* ==========================================================================
 * StoreProvider
 * Wires Zustand stores into the application tree. Each store is created at
 * module scope; this provider is the single composition point for store-side
 * bootstrapping.
 * ========================================================================== */

import type { ReactNode } from "react";
import { useEffect } from "react";

import {
  useRecoveryFeed,
  useTransferFeed,
  useTransferNotifications,
} from "@/hooks";
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

  // A transfer keeps running while the user is elsewhere in the app, so the
  // queue is fed from the shell rather than from one page.
  useTransferFeed();
  // Finished transfers speak up once each, wherever the user is, and what was
  // interrupted is checked once at startup — never acted on.
  useTransferNotifications();
  useRecoveryFeed();

  return <>{children}</>;
}
