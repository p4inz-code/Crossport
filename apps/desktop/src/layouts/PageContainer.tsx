/* ==========================================================================
 * PageContainer component
 * Wraps page content with consistent width, padding, and heading.
 *
 * Pages are not a fixed-width column in the middle of the window. The
 * `width` variant says how much of the window a surface is entitled to:
 * a settings form wants a readable measure, a directory browser wants the
 * whole frame. The default is the readable measure, so a page that says
 * nothing still looks deliberate.
 * ========================================================================== */

import type { ReactNode } from "react";

import { cn } from "@/lib";
import "./PageContainer.css";

/** How much of the window a page may use. */
export type PageWidth = "default" | "wide" | "fluid";

interface PageContainerProps {
  title: string;
  description?: string;
  actions?: ReactNode;
  children: ReactNode;
  /** `default` (readable measure), `wide`, or `fluid` (fills the frame). */
  width?: PageWidth;
  className?: string;
}

export function PageContainer({
  title,
  description,
  actions,
  children,
  width = "default",
  className,
}: PageContainerProps) {
  return (
    <div
      className={cn("page-container", `page-container--${width}`, className)}
    >
      <header className="page-container__header">
        <div className="page-container__heading">
          <h1 className="page-container__title">{title}</h1>
          {description !== undefined ? (
            <p className="page-container__description">{description}</p>
          ) : null}
        </div>
        {actions !== undefined ? (
          <div className="page-container__actions">{actions}</div>
        ) : null}
      </header>
      <div className="page-container__content">{children}</div>
    </div>
  );
}
