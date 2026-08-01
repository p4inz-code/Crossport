/* ==========================================================================
 * App store — initial state only
 * Holds top-level application metadata and status.
 * ========================================================================== */

import { create } from "zustand";

import { APP_NAME, APP_VERSION } from "@/lib";
import type { AppPlatform, AppStatus } from "@/types";

const isTauri =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

interface AppState {
  name: string;
  version: string;
  platform: AppPlatform;
  status: AppStatus;
}

export const useAppStore = create<AppState>(() => ({
  name: APP_NAME,
  version: APP_VERSION,
  platform: isTauri ? "desktop" : "web",
  status: "ready",
}));
