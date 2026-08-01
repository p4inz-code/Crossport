/* ==========================================================================
 * ErrorBoundary
 * Class-based boundary that catches render errors anywhere below it and
 * renders a recoverable fallback instead of unmounting the whole app.
 * ========================================================================== */

import { AlertTriangle } from "lucide-react";
import type { ErrorInfo, ReactNode } from "react";
import { Component } from "react";

import { Button, EmptyState } from "@/components/ui";
import "./ErrorBoundary.css";

interface ErrorBoundaryProps {
  children: ReactNode;
}

interface ErrorBoundaryState {
  hasError: boolean;
}

export class ErrorBoundary extends Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  state: ErrorBoundaryState = { hasError: false };

  static getDerivedStateFromError(): ErrorBoundaryState {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    console.error("[error-boundary] uncaught render error", error, info);
  }

  private handleReload = (): void => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="error-boundary">
          <EmptyState
            icon={AlertTriangle}
            title="Something went wrong"
            description="An unexpected error occurred while rendering the application. Reloading should restore it."
            action={
              <Button variant="primary" onClick={this.handleReload}>
                Reload application
              </Button>
            }
          />
        </div>
      );
    }

    return this.props.children;
  }
}
