/* ==========================================================================
 * Recovery card
 * One interrupted transfer, what is known about it, and the decisions that are
 * actually available.
 *
 * Actions are disabled when the backend says they cannot work — a job whose
 * source is gone offers no restart — so the user never presses a button that
 * only produces a refusal. Every action is explicit: nothing here runs on its
 * own, and nothing removes a partial file without the user asking for it.
 * ========================================================================== */

import { AlertTriangle, RotateCcw, ShieldCheck, Trash2 } from "lucide-react";

import { Button, Card, CardBody, CardHeader } from "@/components/ui";
import { formatDateTime } from "@/lib";
import type { RecoveryCandidate } from "@/types";
import {
  actionLabel,
  artifactSummary,
  candidateOutcome,
  candidateProgress,
  candidateProof,
  candidateReason,
  candidateTitle,
  restartImpactLine,
} from "../presentation";

interface RecoveryCardProps {
  candidate: RecoveryCandidate;
  /** True while an action for this candidate is in flight. */
  busy: boolean;
  /** Structured failure of the last action for this candidate. */
  failure: string | null;
  onAct: (id: string, action: "restart" | "discard" | "confirm") => void;
}

export function RecoveryCard({
  candidate,
  busy,
  failure,
  onAct,
}: RecoveryCardProps) {
  const artifacts = artifactSummary(candidate);
  const impact = restartImpactLine(candidate);
  const proof = candidateProof(candidate);
  const reason = candidateReason(candidate);

  return (
    <Card className={`recovery-card recovery-card--${candidate.outcome}`}>
      <CardHeader
        title={candidateTitle(candidate)}
        description={candidate.id}
        action={
          <span className="recovery-card__outcome">
            {candidateOutcome(candidate)}
          </span>
        }
      />

      <CardBody>
        <dl className="recovery-card__grid">
          <div className="recovery-card__field">
            <dt>Reported progress</dt>
            <dd>{candidateProgress(candidate)}</dd>
          </div>

          <div className="recovery-card__field">
            <dt>Stopped</dt>
            <dd>{formatDateTime(candidate.updatedAtMs)}</dd>
          </div>

          <div className="recovery-card__field">
            <dt>Queued</dt>
            <dd>{formatDateTime(candidate.queuedAtMs)}</dd>
          </div>

          {artifacts !== null ? (
            <div className="recovery-card__field recovery-card__field--wide">
              <dt>Partial files left behind</dt>
              <dd>{artifacts}</dd>
            </div>
          ) : null}
        </dl>

        {reason !== null ? (
          <p className="recovery-card__reason">
            <AlertTriangle size={16} strokeWidth={1.75} aria-hidden="true" />
            {reason}
          </p>
        ) : null}

        {proof !== null ? (
          <p className="recovery-card__proof">
            <ShieldCheck size={16} strokeWidth={1.75} aria-hidden="true" />
            {proof}
          </p>
        ) : null}

        {impact !== null && candidate.canRestart ? (
          <p className="recovery-card__impact">{impact}</p>
        ) : null}

        {failure !== null ? (
          <p className="recovery-card__failure" role="alert">
            {failure}
          </p>
        ) : null}

        <div className="recovery-card__actions">
          <Button
            variant="primary"
            disabled={busy || !candidate.canRestart}
            onClick={() => onAct(candidate.id, "restart")}
            title={
              candidate.canRestart
                ? undefined
                : "This transfer cannot be planned again right now"
            }
          >
            <RotateCcw size={16} strokeWidth={1.75} aria-hidden="true" />
            {actionLabel("restart")}
          </Button>

          <Button
            variant="secondary"
            disabled={busy || !candidate.canDiscard}
            onClick={() => onAct(candidate.id, "discard")}
          >
            <Trash2 size={16} strokeWidth={1.75} aria-hidden="true" />
            {actionLabel("discard")}
          </Button>

          <Button
            variant="secondary"
            disabled={busy || !candidate.confirmedByArchive}
            onClick={() => onAct(candidate.id, "confirm")}
            title={
              candidate.confirmedByArchive
                ? undefined
                : "Only the archive's own record can prove a transfer finished"
            }
          >
            <ShieldCheck size={16} strokeWidth={1.75} aria-hidden="true" />
            {actionLabel("confirm")}
          </Button>
        </div>
      </CardBody>
    </Card>
  );
}
