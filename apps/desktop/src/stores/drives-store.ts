/* ==========================================================================
 * Drives store
 * Holds the drive list reported by the backend. Enumeration happens on
 * demand (`refresh`) so startup stays cheap.
 * ========================================================================== */

import { create } from "zustand";

import { listDrives } from "@/services/drives-service";
import { type IpcError, toIpcError } from "@/services/ipc";
import type { DriveInfo, LoadStatus } from "@/types";

interface DrivesState {
  drives: DriveInfo[];
  status: LoadStatus;
  error: IpcError | null;
  refresh: () => Promise<void>;
}

export const useDrivesStore = create<DrivesState>((set) => ({
  drives: [],
  status: "idle",
  error: null,

  refresh: async () => {
    set({ status: "loading", error: null });
    try {
      const drives = await listDrives();
      set({ drives, status: "ready", error: null });
    } catch (error) {
      const ipcError = toIpcError(error);
      console.warn("[drives] enumeration failed", ipcError);
      set({ drives: [], status: "error", error: ipcError });
    }
  },
}));
