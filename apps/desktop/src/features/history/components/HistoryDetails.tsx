/* ==========================================================================
 * Transfer details
 * Everything history kept about one transfer: what it did, where it went, how
 * much moved, how long it took, what the verification proved, and — when it
 * failed — the structured reason it failed with.
 *
 * The panel exists because "something went wrong" is not an answer. A failure
 * shows the backend's error code and message, and the per-item issues that go
 * with them, in the backend's own wording.
 * ========================================================================== */

import { AlertTriangle, Trash2, X } from "lucide-react";

import { Button, Card, CardBody, CardHeader } from "@/components/ui";
import { formatDateTime } from "@/lib";
import type { HistoryRecord } from "@/types";
import { historyDetails } from "../presentation";

interface HistoryDetailsProps {
  record: HistoryRecord;
  /** True while a delete for this record is in flight. */
  deleting: boolean;
  onClose: () => void;
  onDelete: (id: string) => void;
}

export function HistoryDetails({
  record,
  deleting,
  onClose,
  onDelete,
}: HistoryDetailsProps) {
  const details = historyDetails(record);

  return (
    <Card className={`history-details history-details--${details.tone}`}>
      <CardHeader
        title={`${details.operation} · ${details.statusLabel}`}
        description={record.id}
        action={
          <>
            <Button
              variant="secondary"
              onClick={() => onDelete(record.id)}
              disabled={deleting}
            >
              <Trash2 size={16} strokeWidth={1.75} aria-hidden="true" />
              {deleting ? "Removing…" : "Delete record"}
            </Button>
            <Button variant="secondary" onClick={onClose}>
              <X size={16} strokeWidth={1.75} aria-hidden="true" />
              Close
            </Button>
          </>
        }
      />

      <CardBody>
        <dl className="history-details__grid">
          <div className="history-details__field history-details__field--wide">
            <dt>Source{details.sources.length === 1 ? "" : "s"}</dt>
            <dd>
              {details.sources.map((source) => (
                <span key={source} className="history-details__path">
                  {source}
                </span>
              ))}
            </dd>
          </div>

          <div className="history-details__field history-details__field--wide">
            <dt>Destination</dt>
            <dd>
              <span className="history-details__path">
                {details.destination}
              </span>
            </dd>
          </div>

          <div className="history-details__field">
            <dt>Items</dt>
            <dd>{details.items}</dd>
          </div>

          <div className="history-details__field">
            <dt>Bytes</dt>
            <dd>{details.bytes}</dd>
          </div>

          <div className="history-details__field">
            <dt>Duration</dt>
            <dd>{details.duration}</dd>
          </div>

          <div className="history-details__field">
            <dt>Verification</dt>
            <dd
              className={
                details.verificationFailed
                  ? "history-details__verification history-details__verification--failed"
                  : "history-details__verification"
              }
            >
              {details.verification}
            </dd>
          </div>

          <div className="history-details__field">
            <dt>Queued</dt>
            <dd>{formatDateTime(details.queuedAt)}</dd>
          </div>

          <div className="history-details__field">
            <dt>Started</dt>
            <dd>{formatDateTime(details.startedAt)}</dd>
          </div>

          <div className="history-details__field">
            <dt>Finished</dt>
            <dd>{formatDateTime(details.finishedAt)}</dd>
          </div>

          {details.conflicts !== null ? (
            <div className="history-details__field history-details__field--wide">
              <dt>Conflicts</dt>
              <dd>{details.conflicts}</dd>
            </div>
          ) : null}

          {details.recovery !== null ? (
            <div className="history-details__field history-details__field--wide">
              <dt>Recovery</dt>
              <dd>{details.recovery}</dd>
            </div>
          ) : null}
        </dl>

        {details.failure !== null ? (
          <div className="history-details__failure" role="alert">
            <p className="history-details__failure-title">
              <AlertTriangle size={16} strokeWidth={2} aria-hidden="true" />
              Why it failed
            </p>
            <p className="history-details__failure-code">
              {details.failure.code}
            </p>
            <p className="history-details__failure-message">
              {details.failure.message}
            </p>
          </div>
        ) : null}

        {details.issues.length > 0 ? (
          <div className="history-details__issues">
            <p className="history-details__issues-title">
              Items reported
              {details.issuesTruncated ? " (more than are listed)" : ""}
            </p>
            <ul className="history-details__issue-list">
              {details.issues.map((issue) => (
                <li key={issue} className="history-details__issue">
                  {issue}
                </li>
              ))}
            </ul>
          </div>
        ) : null}
      </CardBody>
    </Card>
  );
}
