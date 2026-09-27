/* ==========================================================================
 * useRecoveryFeed hook
 * Loads what was interrupted when the application starts, and raises exactly
 * one notification about it. It never acts: a recovery action only ever happens
 * because the user asked for one.
 *
 * The load runs once per mount of the application shell, which is what makes it
 * the startup experience rather than a poll.
 * ========================================================================== */

import { useEffect } from "react";

import { notificationForRecovery } from "@/features/notifications/presentation";
import { useNotificationStore, useRecoveryStore } from "@/stores";

export function useRecoveryFeed(): void {
  useEffect(() => {
    let stopped = false;

    void useRecoveryStore
      .getState()
      .load()
      .then(() => {
        if (stopped) {
          return;
        }
        const candidates = useRecoveryStore.getState().candidates;
        const notification = notificationForRecovery(candidates);
        if (notification !== null) {
          useNotificationStore.getState().notify({
            ...notification,
            jobId: null,
          });
        }
      })
      .catch((error: unknown) => {
        console.warn("[recovery] startup recovery check failed", error);
      });

    return () => {
      stopped = true;
    };
  }, []);
}
