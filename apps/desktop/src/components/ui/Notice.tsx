/* ==========================================================================
 * Notice component
 * One inline panel for a fact the user needs before anything else on the page:
 * a document that could not be used, a call that failed, a decision waiting.
 *
 * It exists so those facts look the same everywhere instead of each page
 * inventing its own banner. It states what happened in the backend's words and
 * never claims more than the caller passed it.
 * ========================================================================== */

import type { LucideIcon } from "lucide-react";
import { AlertTriangle, Info, OctagonAlert } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib";
import "./Notice.css";

export type NoticeTone = "info" | "warning" | "danger";

const TONE_ICONS: Record<NoticeTone, LucideIcon> = {
  info: Info,
  warning: AlertTriangle,
  danger: OctagonAlert,
};

interface NoticeProps {
  tone: NoticeTone;
  title: string;
  /** The backend's own explanation, when there is one. */
  detail?: string;
  /** Overrides the tone's icon. */
  icon?: LucideIcon;
  /** Buttons or links that belong with the message. */
  action?: ReactNode;
  /** How assistive technology should treat it. Errors interrupt; facts do not. */
  urgent?: boolean;
}

export function Notice({
  tone,
  title,
  detail,
  icon,
  action,
  urgent = tone === "danger",
}: NoticeProps) {
  const Icon = icon ?? TONE_ICONS[tone];

  return (
    <div
      className={cn("notice", `notice--${tone}`)}
      role={urgent ? "alert" : "status"}
    >
      <Icon
        size={18}
        strokeWidth={1.75}
        aria-hidden="true"
        className="notice__icon"
      />
      <div className="notice__body">
        <p className="notice__title">{title}</p>
        {detail !== undefined ? (
          <p className="notice__detail">{detail}</p>
        ) : null}
        {action !== undefined ? (
          <div className="notice__action">{action}</div>
        ) : null}
      </div>
    </div>
  );
}
