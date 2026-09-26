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

import { Button } from "@/components/ui";
import { PageContainer } from "@/layouts";
import { useTransferStore } from "@/stores";
import { TransferQueue } from "../components/TransferQueue";
import { isFinished } from "../presentation";
import "./TransfersPage.css";

export function TransfersPage() {
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
      />
    </PageContainer>
  );
}
