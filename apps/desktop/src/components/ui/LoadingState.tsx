/* ==========================================================================
 * LoadingState component
 * ========================================================================== */

import { Loader2 } from "lucide-react";

import { cn } from "@/lib";
import "./LoadingState.css";

interface LoadingStateProps {
  label?: string;
  className?: string;
}

/** Centered loading indicator with an optional label. */
export function LoadingState({
  label = "Loading…",
  className,
}: LoadingStateProps) {
  return (
    <div
      className={cn("loading-state", className)}
      role="status"
      aria-live="polite"
    >
      <Loader2
        className="loading-state__spinner"
        size={24}
        aria-hidden="true"
      />
      <span className="loading-state__label">{label}</span>
    </div>
  );
}
