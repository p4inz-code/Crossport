/* ==========================================================================
 * Transfers page
 * The queue surface: every job the engine is holding, what each one has moved
 * so far, and the controls that pause, continue, cancel, or drop it.
 *
 * The page owns the store wiring only. Progress arrives as typed
 * `transfer:update` events and every control call happens in Rust; nothing on
 * this page estimates a number the engine did not report.
 * ========================================================================== */

import { Eraser, RefreshCw } from "lucide-react";
import { useNavigate } from "react-router-dom";

import { Button, Notice } from "@/components/ui";
import { PageContainer } from "@/layouts";
import { useHistoryStore, useRecoveryStore, useTransferStore } from "@/stores";
import { TransferQueue } from "../components/TransferQueue";
import { isFinished } from "../presentation";
import "./TransfersPage.css";

export function TransfersPage() {
  const navigate = useNavigate();
  const jobs = useTransferStore((state) => state.jobs);
  const status = useTransferStore((state) => state.status);
  const error = useTransferStore((state) => state.error);
  const pending = useTransferStore((state) => state.pending);
  const failures = useTransferStore((state) => state.failures);
  const refresh = useTransferStore((state) => state.refresh);
  const pause = useTransferStore((state) => state.pause);
  const resume = useTransferStore((state) => state.resume);
  const cancel = useTransferStore((state) => state.cancel);
  const remove = useTransferStore((state) => state.remove);
  const clearFinished = useTransferStore((state) => state.clearFinished);

  // What was interrupted in an earlier run is not part of the live queue, but
  // it is exactly what this page's user is looking for, so it is stated here.
  const interrupted = useRecoveryStore((state) => state.candidates.length);

  const finishedCount = jobs.filter(isFinished).length;

  return (
    <PageContainer
      title="Transfers"
      description="Jobs the transfer engine is running or has finished, in the order they were queued."
      className="transfers-page"
      actions={
        <>
          <Button
            variant="secondary"
            onClick={() => void refresh()}
            disabled={status === "loading"}
          >
            <RefreshCw size={16} strokeWidth={1.75} aria-hidden="true" />
            Refresh
          </Button>
          <Button
            variant="secondary"
            onClick={() => void clearFinished()}
            disabled={finishedCount === 0}
          >
            <Eraser size={16} strokeWidth={1.75} aria-hidden="true" />
            Clear finished
            {finishedCount > 0 ? ` (${finishedCount})` : ""}
          </Button>
        </>
      }
    >
      {interrupted > 0 ? (
        <Notice
          tone="warning"
          title={`${interrupted} interrupted transfer${
            interrupted === 1 ? "" : "s"
          } need${interrupted === 1 ? "s" : ""} a decision`}
          detail="These were running when the application last stopped. A transfer that did not prove it finished is never treated as finished, and nothing runs again until you choose."
          action={
            <Button
              size="sm"
              variant="secondary"
              onClick={() => navigate("/recovery")}
            >
              Review recovery
            </Button>
          }
        />
      ) : null}

      <TransferQueue
        jobs={jobs}
        status={status}
        error={error}
        pending={pending}
        failures={failures}
        onPause={(id) => void pause(id)}
        onResume={(id) => void resume(id)}
        onCancel={(id) => void cancel(id)}
        onRemove={(id) => void remove(id)}
        onRetry={() => void refresh()}
        onBrowse={() => navigate("/drives")}
        onOpenHistory={(id) => {
          // The record keeps the job's identifier, so the details panel can be
          // opened on the record the user just asked about.
          useHistoryStore.getState().select(id);
          navigate("/history");
        }}
      />
    </PageContainer>
  );
}
