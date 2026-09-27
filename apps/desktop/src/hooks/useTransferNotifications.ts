/* ==========================================================================
 * useTransferNotifications hook
 * Watches the queue and raises one notification each time a job *reaches* a
 * terminal state while the application is running.
 *
 * The distinction matters: seeding the seen-state on mount means a page reload
 * does not replay notifications for transfers that finished earlier, while a
 * job that finishes while the user is on another page still speaks up once.
 * ========================================================================== */

import { useEffect } from "react";

import { notificationForTransfer } from "@/features/notifications/presentation";
import { useNotificationStore, useTransferStore } from "@/stores";

export function useTransferNotifications(): void {
  useEffect(() => {
    const seen = new Map<string, string>();
    for (const job of useTransferStore.getState().jobs) {
      seen.set(job.id, job.status);
    }

    return useTransferStore.subscribe((state) => {
      for (const job of state.jobs) {
        const previous = seen.get(job.id);
        seen.set(job.id, job.status);

        // A job seen for the first time here was not observed changing, so it
        // is not a transition worth announcing.
        if (previous === undefined || previous === job.status) {
          continue;
        }

        const notification = notificationForTransfer(job);
        if (notification !== null) {
          useNotificationStore.getState().notify({
            ...notification,
            jobId: job.id,
          });
        }
      }

      // Jobs the queue no longer holds are forgotten, so their identifiers do
      // not accumulate and a reused identifier cannot inherit this state.
      for (const id of [...seen.keys()]) {
        if (!state.jobs.some((job) => job.id === id)) {
          seen.delete(id);
        }
      }
    });
  }, []);
}
