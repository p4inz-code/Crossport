/* ==========================================================================
 * Recovery page
 * What was interrupted when the application last stopped, and the explicit
 * decisions available.
 *
 * Two rules make this page what it is:
 *
 * - nothing here runs automatically: a restart, a discard, and a confirmation
 *   all happen because the user pressed a button;
 * - a transfer is never called complete because a destination file exists. The
 *   only completion the page shows is the one the archive recorded.
 * ========================================================================== */

import {
  AlertTriangle,
  RefreshCw,
  ShieldAlert,
  ShieldCheck,
} from "lucide-react";
import { useEffect } from "react";

import { Button, EmptyState, LoadingState, Section } from "@/components/ui";
import { PageContainer } from "@/layouts";
import { useRecoveryStore } from "@/stores";
import { archiveLoadStateLabel, isDocumentNoteworthy } from "@/types";
import { RecoveryCard } from "../components/RecoveryCard";
import "./RecoveryPage.css";

export function RecoveryPage() {
  const candidates = useRecoveryStore((state) => state.candidates);
  const documentStatus = useRecoveryStore((state) => state.documentStatus);
  const writable = useRecoveryStore((state) => state.writable);
  const status = useRecoveryStore((state) => state.status);
  const error = useRecoveryStore((state) => state.error);
  const pending = useRecoveryStore((state) => state.pending);
  const failures = useRecoveryStore((state) => state.failures);
  const load = useRecoveryStore((state) => state.load);
  const act = useRecoveryStore((state) => state.act);

  // The startup feed loads this once; opening the page directly (or reloading
  // it) loads it again, so the list is never stale and never empty by accident.
  useEffect(() => {
    void load();
  }, [load]);

  const loading = status === "loading";
  const restartable = candidates.filter(
    (candidate) => candidate.canRestart,
  ).length;

  return (
    <PageContainer
      title="Recovery"
      description="Transfers that were still running when the application stopped. Nothing is changed until you choose an action."
      className="recovery-page"
      actions={
        <Button
          variant="secondary"
          onClick={() => void load()}
          disabled={loading}
        >
          <RefreshCw size={16} strokeWidth={1.75} aria-hidden="true" />
          Refresh
        </Button>
      }
    >
      {documentStatus !== null && isDocumentNoteworthy(documentStatus) ? (
        <div className="recovery-page__degraded" role="alert">
          <AlertTriangle size={18} strokeWidth={1.75} aria-hidden="true" />
          <div>
            <p className="recovery-page__degraded-title">
              {archiveLoadStateLabel(documentStatus.state)}
            </p>
            <p className="recovery-page__degraded-detail">
              {documentStatus.detail ??
                "The interrupted-transfer state could not be used as written."}
              {writable
                ? " It was set aside so recovery can keep working; a transfer that was running may no longer be listed."
                : " Its contents are untouched; this build will not overwrite a newer document."}
            </p>
          </div>
        </div>
      ) : null}

      {loading && candidates.length === 0 ? (
        <LoadingState label="Checking for interrupted transfers…" />
      ) : null}

      {error !== null ? (
        <div className="recovery-page__error" role="alert">
          <p className="recovery-page__error-title">
            Interrupted transfers could not be listed
          </p>
          <p className="recovery-page__error-message">
            {error.message}
            {error.code === "unavailable"
              ? " Recovery state is stored by the desktop application."
              : ""}
          </p>
          <Button variant="secondary" onClick={() => void load()}>
            Try again
          </Button>
        </div>
      ) : null}

      {error === null && !loading && candidates.length === 0 ? (
        <EmptyState
          icon={ShieldCheck}
          title="Nothing needs recovery"
          description="Every transfer the application started either finished or was accounted for."
        />
      ) : null}

      {candidates.length > 0 ? (
        <Section
          title="Interrupted transfers"
          description={
            restartable === candidates.length
              ? "Each one can be run again from the beginning, or discarded, or left alone."
              : `${restartable} of ${candidates.length} can be run again; the others can still be inspected or discarded.`
          }
        >
          <ul className="recovery-page__list">
            {candidates.map((candidate) => (
              <li key={candidate.id}>
                <RecoveryCard
                  candidate={candidate}
                  busy={pending.includes(candidate.id)}
                  failure={failures[candidate.id]?.message ?? null}
                  onAct={(id, action) => void act(id, action)}
                />
              </li>
            ))}
          </ul>
        </Section>
      ) : null}

      <Section title="How recovery works">
        <div className="recovery-page__notes">
          <p>
            <ShieldAlert size={16} strokeWidth={1.75} aria-hidden="true" />A
            transfer that did not prove it finished is treated as unfinished.
            The archive never infers success from a destination file alone.
          </p>
          <p>
            <ShieldAlert size={16} strokeWidth={1.75} aria-hidden="true" />A
            restart begins every affected file from zero after removing the
            partial files it left behind: nothing persisted proves which prefix
            of a partial file is valid, so continuing mid-file is not offered.
          </p>
          <p>
            <ShieldAlert size={16} strokeWidth={1.75} aria-hidden="true" />
            Discarding only removes files named as this job&apos;s own temporary
            output; everything else in the destination is left untouched.
          </p>
        </div>
      </Section>
    </PageContainer>
  );
}
