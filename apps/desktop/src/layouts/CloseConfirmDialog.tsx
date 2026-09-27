/* ==========================================================================
 * CloseConfirmDialog component
 * The question the application asks before it lets work in flight go.
 *
 * Rust holds the window close and says what would be set aside; nothing here
 * decides that. Closing is offered as a real choice — the transfer state is
 * journaled, so an interrupted job is listed under Recovery next time — but it
 * is never taken silently, and it is never the default action.
 * ========================================================================== */

import { ArrowRightLeft, ShieldAlert, X } from "lucide-react";
import { useEffect, useRef } from "react";

import { Button } from "@/components/ui";
import { useCloseGuard } from "@/hooks";
import { exitApp } from "@/services/app-service";
import "./CloseConfirmDialog.css";

export function CloseConfirmDialog() {
  const { warning, clear } = useCloseGuard();
  const keepWorkingRef = useRef<HTMLButtonElement>(null);

  // The safe answer takes focus, and Escape takes it too.
  useEffect(() => {
    if (warning === null) {
      return;
    }
    keepWorkingRef.current?.focus();

    function onKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") {
        event.preventDefault();
        clear();
      }
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [warning, clear]);

  if (warning === null) {
    return null;
  }

  return (
    <div className="close-confirm__overlay" role="presentation">
      <div
        className="close-confirm"
        role="dialog"
        aria-modal="true"
        aria-labelledby="close-confirm-title"
        aria-describedby="close-confirm-detail"
      >
        <header className="close-confirm__header">
          <span className="close-confirm__icon" aria-hidden="true">
            <ShieldAlert size={20} strokeWidth={1.75} />
          </span>
          <div>
            <h2 className="close-confirm__title" id="close-confirm-title">
              Close CrossPort?
            </h2>
            <p className="close-confirm__detail" id="close-confirm-detail">
              {warning.detail}
            </p>
          </div>
          <button
            type="button"
            className="close-confirm__dismiss"
            aria-label="Stay in CrossPort"
            onClick={clear}
          >
            <X size={16} strokeWidth={2} aria-hidden="true" />
          </button>
        </header>

        <dl className="close-confirm__facts">
          <div className="close-confirm__fact">
            <dt>Running now</dt>
            <dd>{warning.liveJobs}</dd>
          </div>
          <div className="close-confirm__fact">
            <dt>Waiting for a decision</dt>
            <dd>{warning.interruptedJobs}</dd>
          </div>
        </dl>

        <p className="close-confirm__note">
          Files are written to a temporary name and renamed only when complete,
          so nothing is left half-written. A job that was running is listed
          under Recovery the next time the application starts.
        </p>

        <footer className="close-confirm__actions">
          <Button
            ref={keepWorkingRef}
            variant="secondary"
            onClick={clear}
            title="Stay in CrossPort (Escape)"
          >
            <ArrowRightLeft size={16} strokeWidth={1.75} aria-hidden="true" />
            Keep working
          </Button>
          <Button
            variant="destructive"
            onClick={() => {
              void exitApp().catch((error: unknown) => {
                console.warn(
                  "[app] the application could not be closed",
                  error,
                );
              });
            }}
          >
            Close anyway
          </Button>
        </footer>
      </div>
    </div>
  );
}
