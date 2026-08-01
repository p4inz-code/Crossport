/* ==========================================================================
 * Card component
 * ========================================================================== */

import type { HTMLAttributes, ReactNode } from "react";

import { cn } from "@/lib";
import "./Card.css";

interface CardProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
}

/** Base card container. */
export function Card({ className, children, ...rest }: CardProps) {
  return (
    <div className={cn("card", className)} {...rest}>
      {children}
    </div>
  );
}

interface CardHeaderProps extends HTMLAttributes<HTMLDivElement> {
  title: string;
  description?: string;
  action?: ReactNode;
}

/** Card header with title, optional description, and optional action slot. */
export function CardHeader({
  title,
  description,
  action,
  className,
}: CardHeaderProps) {
  return (
    <div className={cn("card-header", className)}>
      <div className="card-header__text">
        <h3 className="card-header__title">{title}</h3>
        {description !== undefined ? (
          <p className="card-header__description">{description}</p>
        ) : null}
      </div>
      {action !== undefined ? (
        <div className="card-header__action">{action}</div>
      ) : null}
    </div>
  );
}

interface CardBodyProps extends HTMLAttributes<HTMLDivElement> {
  children: ReactNode;
}

/** Card body content area. */
export function CardBody({ className, children, ...rest }: CardBodyProps) {
  return (
    <div className={cn("card-body", className)} {...rest}>
      {children}
    </div>
  );
}
