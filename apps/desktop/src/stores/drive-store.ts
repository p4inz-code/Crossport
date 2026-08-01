/* ==========================================================================
 * Drive store — initial state only
 * Placeholder state for detected drives (implemented in a later phase).
 * ========================================================================== */

import { create } from "zustand";

import type { Drive } from "@/types";

interface DriveState {
  drives: Drive[];
  isScanning: boolean;
  /** Timestamp (ISO string) of the last completed scan, if any. */
  lastScanAt: string | null;
}

export const useDriveStore = create<DriveState>(() => ({
  drives: [],
  isScanning: false,
  lastScanAt: null,
}));
