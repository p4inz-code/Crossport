/* ==========================================================================
 * Button component
 * ========================================================================== */

import type { ButtonHTMLAttributes, ReactNode } from "react";

import { cn } from "@/lib";
import "./Button.css";

export type ButtonVariant = "primary" | "secondary" | "destructive" | "ghost";
export type ButtonSize = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  children: ReactNode;
}

export function Button({
  variant = "primary",
  size = "md",
  className,
  children,
  type = "button",
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      className={cn(
        "button",
        `button--${variant}`,
        `button--${size}`,
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}
