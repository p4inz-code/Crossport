/* ==========================================================================
 * App store
 * Application-level metadata for the shell. Platform facts live in the system
 * store, which reads them from the backend instead of inferring them.
 * ========================================================================== */

import { create } from "zustand";

import { APP_NAME, APP_VERSION } from "@/lib";

interface AppState {
  name: string;
  version: string;
}

export const useAppStore = create<AppState>(() => ({
  name: APP_NAME,
  version: APP_VERSION,
}));
