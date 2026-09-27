/* ==========================================================================
 * TransferQueue component
 * The queue as a list of jobs: what each job is doing, how far along it is,
 * what it could not transfer, and the controls that change its course.
 *
 * Presentational only. It renders the snapshots the store holds and asks the
 * page to act — every control call and every piece of wording comes from
 * somewhere else.
 *
 * Cancelling is deliberately a two-step action: a cancel discards whatever
 * partial output the job had written, so the button asks before it does that.
 * ========================================================================== */

import { Play, Trash2, TriangleAlert, X } from "lucide-react";
import { useState } from "react";

import { Button, EmptyState, LoadingState } from "@/components/ui";
import { cn, formatBytes } from "@/lib";
import type { IpcError } from "@/services/ipc";
import type { LoadStatus, TransferSnapshot } from "@/types";
import {
  canCancel,
  canPause,
  canResume,
  EMPTY_QUEUE_ICON,
  formatDuration,
  formatEta,
  formatSpeed,
  isFinished,
  isMoving,
  issueError,
  issueLabel,
  isVerifying,
  jobError,
  keyedIssues,
  metadataNote,
  TRANSFER_ISSUE_ICONS,
  TRANSFER_OPERATION_ICONS,
  TRANSFER_OPERATION_LABELS,
  TRANSFER_STATUS_ICONS,
  TRANSFER_STATUS_TONES,
  transferBytesLabel,
  transferCountsLabel,
  transferStatusLabel,
  transferTitle,
  VERIFICATION_ICONS,
  VERIFICATION_TONES,
  verificationLine,
} from "../presentation";
import "./TransferQueue.css";

/** How many issues of one job are listed before the rest are counted. */
const ISSUE_PREVIEW_LIMIT = 4;

interface TransferQueueProps {
  jobs: TransferSnapshot[];
  status: LoadStatus;
  error: IpcError | null;
  /** Identifiers with an in-flight control call. */
  pending: string[];
  /** Control-call failures, keyed by job identifier. */
  failures: Record<string, IpcError>;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onCancel: (id: string) => void;
  onRemove: (id: string) => void;
  onRetry: () => void;
}

export function TransferQueue({
  jobs,
  status,
  error,
  pending,
  failures,
  onPause,
  onResume,
  onCancel,
  onRemove,
  onRetry,
}: TransferQueueProps) {
  return (
    <div className="transfer-queue">
      {status === "loading" && jobs.length === 0 ? (
        <LoadingState label="Reading the transfer queue…" />
      ) : null}

      {error !== null ? (
        <div role="alert">
          <EmptyState
            icon={TriangleAlert}
            title="Transfer queue unavailable"
            description={error.message}
            action={<Button onClick={onRetry}>Try again</Button>}
          />
        </div>
      ) : null}

      {error === null && status === "ready" && jobs.length === 0 ? (
        <EmptyState
          icon={EMPTY_QUEUE_ICON}
          title="No transfers yet"
          description="Open the Drives page, select the items you want to move, and choose Copy to… or Move to…."
        />
      ) : null}

      {jobs.length > 0 ? (
        <ul className="transfer-queue__items">
          {jobs.map((job) => (
            <TransferCard
              key={job.id}
              job={job}
              busy={pending.includes(job.id)}
              failure={failures[job.id] ?? null}
              onPause={onPause}
              onResume={onResume}
              onCancel={onCancel}
              onRemove={onRemove}
            />
          ))}
        </ul>
      ) : null}
    </div>
  );
}

interface TransferCardProps {
  job: TransferSnapshot;
  busy: boolean;
  failure: IpcError | null;
  onPause: (id: string) => void;
  onResume: (id: string) => void;
  onCancel: (id: string) => void;
  onRemove: (id: string) => void;
}

function TransferCard({
  job,
  busy,
  failure,
  onPause,
  onResume,
  onCancel,
  onRemove,
}: TransferCardProps) {
  const [confirmingCancel, setConfirmingCancel] = useState(false);

  const StatusIcon = TRANSFER_STATUS_ICONS[job.status];
  const OperationIcon = TRANSFER_OPERATION_ICONS[job.operation];
  const title = transferTitle(job);
  const { progress, verification } = job;
  const percent = progress.percent;
  const problems = progress.failedItems + progress.skippedItems;
  const problem = failure ?? jobError(job);
  const VerificationIcon = VERIFICATION_ICONS[verification.status];
  const metadata = metadataNote(verification);
  // A queued job has no verdict yet, so there is nothing to claim about it.
  const showsVerification = verification.status !== "pending";

  return (
    <li className="transfer-card">
      <div className="transfer-card__header">
        <span className="transfer-card__title" title={job.sources.join(", ")}>
          {title}
        </span>
        <span className="transfer-card__operation">
          <OperationIcon size={14} strokeWidth={1.75} aria-hidden="true" />
          {TRANSFER_OPERATION_LABELS[job.operation]}
        </span>
        <span
          className={cn(
            "transfer-card__status",
            `transfer-card__status--${TRANSFER_STATUS_TONES[job.status]}`,
          )}
        >
          <StatusIcon
            size={14}
            strokeWidth={1.75}
            aria-hidden="true"
            className={
              isMoving(job) ? "transfer-card__status-icon--spin" : undefined
            }
          />
          {transferStatusLabel(job)}
        </span>
      </div>

      <p className="transfer-card__destination">
        {TRANSFER_OPERATION_LABELS[job.operation]} into {job.destination}
      </p>

      <div className="transfer-card__progress">
        <div
          className={cn(
            "transfer-card__bar",
            percent === null && "transfer-card__bar--indeterminate",
          )}
          role="progressbar"
          aria-label={`${title} progress`}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent ?? undefined}
        >
          <span
            className="transfer-card__bar-fill"
            style={{ width: `${percent ?? 100}%` }}
          />
        </div>
        <span className="transfer-card__percent">
          {percent === null ? "Working…" : `${percent}%`}
        </span>
      </div>

      <div className="transfer-card__metrics">
        <span>{transferBytesLabel(progress)}</span>
        <span>{transferCountsLabel(progress)}</span>
        {percent !== null && progress.bytesPerSecond > 0 ? (
          <span>
            {formatSpeed(progress.bytesPerSecond)}
            {progress.etaSeconds !== null
              ? ` · ${formatEta(progress.etaSeconds)}`
              : ""}
          </span>
        ) : null}
        {progress.elapsedMs > 0 ? (
          <span>{formatDuration(progress.elapsedMs)} elapsed</span>
        ) : null}
      </div>

      {progress.currentFile !== null && isMoving(job) ? (
        <p className="transfer-card__current">
          <span className="transfer-card__current-name">
            {progress.currentFile}
          </span>
          {isVerifying(job) ? (
            <span>Checking what was written</span>
          ) : progress.currentFileTotalBytes > 0 ? (
            <span>
              {formatBytes(progress.currentFileBytes)} of{" "}
              {formatBytes(progress.currentFileTotalBytes)}
            </span>
          ) : null}
        </p>
      ) : null}

      {showsVerification ? (
        <div className="transfer-card__verification">
          <p
            className={cn(
              "transfer-card__verdict",
              `transfer-card__verdict--${VERIFICATION_TONES[verification.status]}`,
            )}
          >
            <VerificationIcon
              size={14}
              strokeWidth={1.75}
              aria-hidden="true"
              className={
                verification.status === "verifying"
                  ? "transfer-card__status-icon--spin"
                  : undefined
              }
            />
            {verificationLine(verification)}
          </p>
          {metadata !== null ? (
            <p className="transfer-card__metadata">{metadata}</p>
          ) : null}
        </div>
      ) : null}

      {problems > 0 && job.issues.length > 0 ? (
        <details className="transfer-card__issues">
          <summary>{issueSummary(job)}</summary>
          <ul className="transfer-card__issue-list">
            {keyedIssues(job.issues.slice(0, ISSUE_PREVIEW_LIMIT)).map(
              ({ key, issue }) => {
                const IssueIcon = TRANSFER_ISSUE_ICONS[issue.reason];
                const error = issueError(issue);
                return (
                  <li key={key} className="transfer-card__issue">
                    <IssueIcon
                      size={14}
                      strokeWidth={1.75}
                      aria-hidden="true"
                    />
                    <span className="transfer-card__issue-path">
                      {issue.path}
                    </span>
                    <span className="transfer-card__issue-detail">
                      {issueLabel(issue)}
                    </span>
                    {error !== null && error.code === "disk_full" ? (
                      <span className="transfer-card__issue-note">
                        The destination volume is full.
                      </span>
                    ) : null}
                  </li>
                );
              },
            )}
          </ul>
          {job.issues.length > ISSUE_PREVIEW_LIMIT ? (
            <p className="transfer-card__issues-more">
              …and {job.issues.length - ISSUE_PREVIEW_LIMIT} more.
            </p>
          ) : null}
          {job.issuesTruncated ? (
            <p className="transfer-card__issues-more">
              More issues were recorded than the engine keeps.
            </p>
          ) : null}
        </details>
      ) : null}

      {problem !== null ? (
        <p className="transfer-card__error" role="alert">
          {problem.message}
        </p>
      ) : null}

      <div className="transfer-card__actions">
        {canPause(job) ? (
          <Button
            size="sm"
            variant="secondary"
            onClick={() => onPause(job.id)}
            disabled={busy}
          >
            Pause
          </Button>
        ) : null}

        {canResume(job) ? (
          <Button size="sm" onClick={() => onResume(job.id)} disabled={busy}>
            <Play size={15} strokeWidth={1.75} aria-hidden="true" />
            Resume
          </Button>
        ) : null}

        {canCancel(job) && !confirmingCancel ? (
          <Button
            size="sm"
            variant="destructive"
            onClick={() => setConfirmingCancel(true)}
            disabled={busy}
          >
            <X size={15} strokeWidth={1.75} aria-hidden="true" />
            Cancel
          </Button>
        ) : null}

        {confirmingCancel ? (
          <div className="transfer-card__confirm">
            <span className="transfer-card__confirm-text">
              Discard this transfer?
            </span>
            <Button
              size="sm"
              variant="destructive"
              onClick={() => {
                setConfirmingCancel(false);
                onCancel(job.id);
              }}
              disabled={busy}
            >
              Yes, cancel it
            </Button>
            <Button
              size="sm"
              variant="secondary"
              onClick={() => setConfirmingCancel(false)}
            >
              Keep it
            </Button>
          </div>
        ) : null}

        {isFinished(job) ? (
          <Button
            size="sm"
            variant="secondary"
            onClick={() => onRemove(job.id)}
            disabled={busy}
          >
            <Trash2 size={15} strokeWidth={1.75} aria-hidden="true" />
            Remove from list
          </Button>
        ) : null}
      </div>
    </li>
  );
}

/** `2 skipped · 1 failed`, used as the summary of an issue list. */
function issueSummary(job: TransferSnapshot): string {
  const notes: string[] = [];
  if (job.progress.failedItems > 0) {
    notes.push(`${job.progress.failedItems} failed`);
  }
  if (job.progress.skippedItems > 0) {
    notes.push(`${job.progress.skippedItems} skipped`);
  }
  return notes.join(" · ");
}
