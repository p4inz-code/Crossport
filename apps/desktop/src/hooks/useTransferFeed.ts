/* ==========================================================================
 * useTransferFeed hook
 * Keeps the transfer queue live for as long as the application is mounted:
 * loads what the engine already knows about, then follows `transfer:update`.
 *
 * The subscription is resolved asynchronously (Tauri's `listen` returns a
 * promise), so unmounting before it arrives unsubscribes immediately instead
 * of leaking a listener that would outlive the tree that asked for it.
 * ========================================================================== */

import { useEffect } from "react";

import { useTransferStore } from "@/stores";

export function useTransferFeed(): void {
  useEffect(() => {
    let stopped = false;
    let unsubscribe: (() => void) | null = null;

    void useTransferStore.getState().refresh();
    void useTransferStore
      .getState()
      .connect()
      .then((stop) => {
        if (stopped) {
          stop();
          return;
        }
        unsubscribe = stop;
      })
      .catch((error: unknown) => {
        console.warn("[transfer] subscribing to progress failed", error);
      });

    return () => {
      stopped = true;
      unsubscribe?.();
      unsubscribe = null;
    };
  }, []);
}
