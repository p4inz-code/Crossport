/* ==========================================================================
 * System store
 * Holds the platform facts reported by the backend. In browser dev mode the
 * command is unavailable, which is reported as such instead of being faked.
 * ========================================================================== */

import { create } from "zustand";
import { type IpcError, toIpcError } from "@/services/ipc";
import { getSystemInfo } from "@/services/system-service";
import type { LoadStatus, SystemInfo } from "@/types";

interface SystemState {
  info: SystemInfo | null;
  status: LoadStatus;
  error: IpcError | null;
  hydrate: () => Promise<void>;
}

export const useSystemStore = create<SystemState>((set) => ({
  info: null,
  status: "idle",
  error: null,

  hydrate: async () => {
    set({ status: "loading", error: null });
    try {
      const info = await getSystemInfo();
      set({ info, status: "ready", error: null });
    } catch (error) {
      const ipcError = toIpcError(error);
      console.warn("[system] could not read platform info", ipcError);
      set({ info: null, status: "error", error: ipcError });
    }
  },
}));
