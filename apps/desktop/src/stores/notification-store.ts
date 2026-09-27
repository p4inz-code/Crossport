/* ==========================================================================
 * Notification store
 * Short, structured status feedback about things that happened outside the
 * surface the user is looking at: a transfer finished, a verification failed, a
 * recovery is waiting.
 *
 * The rules this store exists to keep:
 *
 * - one notification per (job, event) — a repeated event does not stack;
 * - nothing here invents progress: a notification always carries a backend
 *   verdict, and `kind` says what it is;
 * - the list is bounded, so a long session cannot grow it forever.
 * ========================================================================== */

import { create } from "zustand";

/** How serious a notification is, and how it is presented. */
export const NOTIFICATION_KINDS = [
  "success",
  "warning",
  "error",
  "info",
] as const;
export type NotificationKind = (typeof NOTIFICATION_KINDS)[number];

export interface AppNotification {
  id: string;
  kind: NotificationKind;
  title: string;
  message: string;
  /** Job the notification is about, when it is about one. */
  jobId: string | null;
  /** Event identifier, used together with `jobId` to deduplicate. */
  event: string;
  createdAtMs: number;
  /**
   * Route holding what the notification is about, when a surface can show it.
   * A notification that says something happened should be able to take the
   * user to it instead of leaving them to find it.
   */
  to: string | null;
}

/** Most notifications kept at once; the oldest are dropped. */
const MAX_NOTIFICATIONS = 5;

interface NotificationState {
  notifications: AppNotification[];
  /** Adds a notification unless the same (job, event) is already shown. */
  notify: (input: {
    kind: NotificationKind;
    title: string;
    message: string;
    event: string;
    jobId?: string | null;
    to?: string | null;
  }) => void;
  dismiss: (id: string) => void;
  clear: () => void;
}

let nextId = 0;

export const useNotificationStore = create<NotificationState>((set) => ({
  notifications: [],

  notify: (input) => {
    set((state) => {
      const jobId = input.jobId ?? null;
      const alreadyShown = state.notifications.some(
        (entry) => entry.jobId === jobId && entry.event === input.event,
      );
      if (alreadyShown) {
        return state;
      }

      nextId += 1;
      const notification: AppNotification = {
        id: `notification-${nextId}`,
        kind: input.kind,
        title: input.title,
        message: input.message,
        jobId,
        event: input.event,
        createdAtMs: Date.now(),
        to: input.to ?? null,
      };

      return {
        notifications: [notification, ...state.notifications].slice(
          0,
          MAX_NOTIFICATIONS,
        ),
      };
    });
  },

  dismiss: (id) => {
    set((state) => ({
      notifications: state.notifications.filter((entry) => entry.id !== id),
    }));
  },

  clear: () => set({ notifications: [] }),
}));
