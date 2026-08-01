/* ==========================================================================
 * PageContainer component
 * Wraps page content with consistent max-width, padding, and heading.
 * ========================================================================== */

import type { ReactNode } from "react";

import { cn } from "@/lib";
import "./PageContainer.css";

interface PageContainerProps {
  title: string;
  description?: string;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
}

export function PageContainer({
  title,
  description,
  actions,
  children,
  className,
}: PageContainerProps) {
  return (
    <div className={cn("page-container", className)}>
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
