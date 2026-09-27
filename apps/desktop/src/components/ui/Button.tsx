/* ==========================================================================
 * Button component
 * ========================================================================== */

import type { ButtonHTMLAttributes, ReactNode, Ref } from "react";

import { cn } from "@/lib";
import "./Button.css";

export type ButtonVariant = "primary" | "secondary" | "destructive" | "ghost";
export type ButtonSize = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  children: ReactNode;
  /**
   * The underlying button, so a caller can move focus to it (a dialog's
   * default action, for example). Declared rather than forwarded so the
   * component keeps working without a wrapper element.
   */
  ref?: Ref<HTMLButtonElement>;
}

export function Button({
  variant = "primary",
  size = "md",
  className,
  children,
  type = "button",
  ref,
  ...rest
}: ButtonProps) {
  return (
    <button
      ref={ref}
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
