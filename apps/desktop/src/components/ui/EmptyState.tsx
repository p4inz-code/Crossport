/* ==========================================================================
 * EmptyState component
 * ========================================================================== */

import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "@/lib";
import "./EmptyState.css";

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}

/** Centered placeholder shown when a view has no content yet. */
export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: EmptyStateProps) {
  return (
    <div className={cn("empty-state", className)}>
      <div className="empty-state__icon" aria-hidden="true">
        <Icon size={28} strokeWidth={1.5} />
      </div>
      <h3 className="empty-state__title">{title}</h3>
      {description !== undefined ? (
        <p className="empty-state__description">{description}</p>
      ) : null}
      {action !== undefined ? (
        <div className="empty-state__action">{action}</div>
      ) : null}
    </div>
  );
}
