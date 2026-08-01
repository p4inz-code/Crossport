/* ==========================================================================
 * Section component
 * ========================================================================== */

import type { HTMLAttributes, ReactNode } from "react";

import { cn } from "@/lib";
import "./Section.css";

interface SectionProps extends HTMLAttributes<HTMLElement> {
  title?: string;
  description?: string;
  children: ReactNode;
}

/** Vertical section with an optional title and description. */
export function Section({
  title,
  description,
  children,
  className,
  ...rest
}: SectionProps) {
  return (
    <section className={cn("section", className)} {...rest}>
      {title !== undefined ? <h2 className="section__title">{title}</h2> : null}
      {description !== undefined ? (
        <p className="section__description">{description}</p>
      ) : null}
      <div className="section__content">{children}</div>
    </section>
  );
}
