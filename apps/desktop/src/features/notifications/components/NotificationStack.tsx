/* ==========================================================================
 * Notification stack
 * The application's only toast surface. Deliberately small and dismissible:
 * progress stays on the transfer surface, and this says what happened once the
 * work is over (or that a decision is waiting).
 *
 * Each notification is announced politely, can be dismissed individually, and
 * — when a surface holds what it is about — offers to open that surface, so a
 * message never leaves the user hunting for the thing it mentions. Nothing
 * here steals focus or blocks the page.
 * ========================================================================== */

import type { LucideIcon } from "lucide-react";
import {
  AlertTriangle,
  CheckCircle2,
  Info,
  ShieldAlert,
  X,
} from "lucide-react";
import { useNavigate } from "react-router-dom";

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

/** What the action button says for each place a notification can lead. */
const ACTION_LABELS: Record<string, string> = {
  "/transfers": "View transfers",
  "/history": "View history",
  "/recovery": "Review recovery",
};

export function NotificationStack() {
  const navigate = useNavigate();
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
        const actionLabel =
          notification.to === null
            ? null
            : (ACTION_LABELS[notification.to] ?? "Open");
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
              {actionLabel !== null && notification.to !== null ? (
                <button
                  type="button"
                  className="notification__action"
                  onClick={() => {
                    const target = notification.to;
                    dismiss(notification.id);
                    if (target !== null) {
                      navigate(target);
                    }
                  }}
                >
                  {actionLabel}
                </button>
              ) : null}
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
