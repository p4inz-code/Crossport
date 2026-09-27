/* ==========================================================================
 * Notification stack
 * The application's only toast surface. Deliberately small and dismissible:
 * progress stays on the transfer surface, and this says what happened once the
 * work is over (or that a decision is waiting).
 *
 * Each notification is announced politely and can be dismissed individually;
 * nothing here steals focus or blocks the page.
 * ========================================================================== */

import type { LucideIcon } from "lucide-react";
import {
  AlertTriangle,
  CheckCircle2,
  Info,
  ShieldAlert,
  X,
} from "lucide-react";
import { cn } from "@/lib";
import { useNotificationStore } from "@/stores";
import type { NotificationKind } from "@/stores/notification-store";
import "./NotificationStack.css";

const ICONS: Record<NotificationKind, LucideIcon> = {
  success: CheckCircle2,
  warning: AlertTriangle,
  error: ShieldAlert,
  info: Info,
};

export function NotificationStack() {
  const notifications = useNotificationStore((state) => state.notifications);
  const dismiss = useNotificationStore((state) => state.dismiss);

  if (notifications.length === 0) {
    return null;
  }

  return (
    // A named section is a landmark region without declaring a role, and the
    // polite live region is what announces a new notification once.
    <section
      className="notification-stack"
      aria-label="Status notifications"
      aria-live="polite"
    >
      {notifications.map((notification) => {
        const Icon = ICONS[notification.kind];
        return (
          <div
            key={notification.id}
            className={cn("notification", `notification--${notification.kind}`)}
            role={notification.kind === "error" ? "alert" : "status"}
          >
            <Icon
              size={18}
              strokeWidth={1.75}
              aria-hidden="true"
              className="notification__icon"
            />
            <div className="notification__body">
              <p className="notification__title">{notification.title}</p>
              <p className="notification__message">{notification.message}</p>
            </div>
            <button
              type="button"
              className="notification__dismiss"
              onClick={() => dismiss(notification.id)}
              aria-label={`Dismiss ${notification.title}`}
            >
              <X size={14} strokeWidth={2} aria-hidden="true" />
            </button>
          </div>
        );
      })}
    </section>
  );
}
