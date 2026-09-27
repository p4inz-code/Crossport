/* ==========================================================================
 * TransferDialog component
 * The confirmation step before a transfer starts: where the items land, how
 * much data they carry, what collides with them, and which conflict strategy
 * resolves it.
 *
 * The numbers come from the backend's dry run, so nothing here is estimated by
 * the frontend, and changing the strategy asks the backend to plan again
 * rather than guessing what would happen.
 * ========================================================================== */

import {
  AlertTriangle,
  ArrowRight,
  FolderInput,
  HardDrive,
  ShieldCheck,
} from "lucide-react";
import { useEffect, useRef } from "react";

import { Button, LoadingState } from "@/components/ui";
import { formatBytes } from "@/lib";
import type {
  ConflictStrategy,
  TransferPreview,
  TransferRequest,
} from "@/types";
import {
  CONFLICT_STRATEGIES,
  DEFAULT_VERIFICATION_POLICY,
  verificationPolicyLabel,
} from "@/types";
import {
  basename,
  CONFLICT_STRATEGY_DESCRIPTIONS,
  CONFLICT_STRATEGY_LABELS,
  TRANSFER_OPERATION_LABELS,
} from "../presentation";
import "./TransferDialog.css";

interface TransferDialogProps {
  request: TransferRequest;
  /** The backend's dry run, or `null` while it is being planned. */
  preview: TransferPreview | null;
  /** A plan or start call is in flight. */
  busy: boolean;
  /** Failure from planning or starting, already normalized for display. */
  error: string | null;
  onConflictChange: (conflict: ConflictStrategy) => void;
  onConfirm: () => void;
  onCancel: () => void;
}

/** Most sources named before the rest are counted. */
const NAMED_SOURCES = 4;

export function TransferDialog({
  request,
  preview,
  busy,
  error,
  onConflictChange,
  onConfirm,
  onCancel,
}: TransferDialogProps) {
  const itemCount = request.sources.length;
  const operation = TRANSFER_OPERATION_LABELS[request.operation];
  const title = `${operation} ${itemCount} ${itemCount === 1 ? "item" : "items"}`;
  const policy = request.verification ?? DEFAULT_VERIFICATION_POLICY;
  const dialogRef = useRef<HTMLDivElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);

  // A dialog takes focus when it opens and gives it back when it closes, so the
  // keyboard stays where the user left it. Escape cancels, which is the only
  // safe interpretation of a dismissed composer: nothing was queued yet.
  useEffect(() => {
    const previous = document.activeElement;
    cancelRef.current?.focus();
    return () => {
      if (previous instanceof HTMLElement) {
        previous.focus();
      }
    };
  }, []);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent): void {
      if (event.key === "Escape") {
        event.preventDefault();
        onCancel();
      }
    }
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onCancel]);

  const namedSources = request.sources.slice(0, NAMED_SOURCES);
  const extraSources = request.sources.length - namedSources.length;

  return (
    <div className="transfer-dialog__overlay">
      <div
        ref={dialogRef}
        className="transfer-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={`${title} to ${basename(request.destination)}`}
        aria-describedby="transfer-dialog-summary"
      >
        <header className="transfer-dialog__header">
          <h2 className="transfer-dialog__title">{title}</h2>
          <p
            className="transfer-dialog__destination"
            id="transfer-dialog-summary"
            title={request.destination}
          >
            {/* The title already names the operation; the description names
                where it lands, which is what the dialog is asking about. */}
            {`Into ${request.destination}`}
          </p>
        </header>

        {preview === null ? (
          <LoadingState label="Checking the destination…" />
        ) : (
          <div className="transfer-dialog__body">
            <dl className="transfer-dialog__facts">
              {/* Only shown when the backend reported no per-root mapping: the
                  mapping below already names every source, and repeating it
                  here would say the same thing twice. */}
              {preview.roots.length === 0 ? (
                <div className="transfer-dialog__fact">
                  <dt>From</dt>
                  <dd>
                    <span className="transfer-dialog__sources">
                      {namedSources.map((source) => (
                        <span
                          key={source}
                          className="transfer-dialog__source"
                          title={source}
                        >
                          {source}
                        </span>
                      ))}
                      {extraSources > 0 ? (
                        <span className="transfer-dialog__source">
                          +{extraSources} more
                        </span>
                      ) : null}
                    </span>
                  </dd>
                </div>
              ) : null}
              <div className="transfer-dialog__fact">
                <dt>Verification</dt>
                <dd>
                  <span className="transfer-dialog__verification">
                    <ShieldCheck
                      size={14}
                      strokeWidth={1.75}
                      aria-hidden="true"
                    />
                    {verificationPolicyLabel(policy)}
                  </span>
                </dd>
              </div>
              <div className="transfer-dialog__fact">
                <dt>Data</dt>
                <dd>{formatBytes(preview.totalBytes)}</dd>
              </div>
              <div className="transfer-dialog__fact">
                <dt>Files</dt>
                <dd>{preview.totalFiles}</dd>
              </div>
              <div className="transfer-dialog__fact">
                <dt>Folders</dt>
                <dd>{preview.totalDirectories}</dd>
              </div>
              <div className="transfer-dialog__fact">
                <dt>Free space there</dt>
                <dd>
                  {preview.availableBytes === null
                    ? "Not reported"
                    : formatBytes(preview.availableBytes)}
                </dd>
              </div>
            </dl>

            {preview.roots.length > 0 ? (
              <ul className="transfer-dialog__roots">
                {preview.roots.map((root) => (
                  <li key={root.source} className="transfer-dialog__root">
                    <span className="transfer-dialog__path" title={root.source}>
                      {root.source}
                    </span>
                    <ArrowRight
                      size={14}
                      strokeWidth={1.75}
                      aria-hidden="true"
                    />
                    <span
                      className="transfer-dialog__path"
                      title={root.destination}
                    >
                      {root.destination}
                    </span>
                    <span className="transfer-dialog__root-meta">
                      {root.skipped
                        ? "Will be left alone by the conflict strategy"
                        : `${formatBytes(root.bytes)} · ${root.files} ${
                            root.files === 1 ? "file" : "files"
                          }`}
                    </span>
                  </li>
                ))}
              </ul>
            ) : null}

            <fieldset className="transfer-dialog__conflict">
              <legend>If something is already there</legend>
              <div className="transfer-dialog__options">
                {CONFLICT_STRATEGIES.map((strategy) => (
                  <label
                    key={strategy}
                    className="transfer-dialog__option"
                    htmlFor={`conflict-${strategy}`}
                  >
                    <input
                      id={`conflict-${strategy}`}
                      type="radio"
                      name="conflict"
                      value={strategy}
                      checked={request.conflict === strategy}
                      disabled={busy}
                      onChange={() => onConflictChange(strategy)}
                    />
                    <span>
                      <span className="transfer-dialog__option-label">
                        {CONFLICT_STRATEGY_LABELS[strategy]}
                      </span>
                      <span className="transfer-dialog__option-detail">
                        {CONFLICT_STRATEGY_DESCRIPTIONS[strategy]}
                      </span>
                    </span>
                  </label>
                ))}
              </div>
            </fieldset>

            {preview.conflicts > 0 ? (
              <p className="transfer-dialog__note">
                <AlertTriangle
                  size={15}
                  strokeWidth={1.75}
                  aria-hidden="true"
                />
                {preview.conflicts} of these items already exist at the
                destination
                {preview.skippedItems > 0
                  ? `, and ${preview.skippedItems} would be left alone (${formatBytes(
                      preview.skippedBytes,
                    )}).`
                  : "."}
              </p>
            ) : null}

            <p className="transfer-dialog__note transfer-dialog__note--calm">
              <ShieldCheck size={15} strokeWidth={1.75} aria-hidden="true" />
              {policy === "none"
                ? "Nothing will be checked after each file is written. Change this in Settings before starting if that is not what you want."
                : "Each copied file is checked before the job reports it as done. Change the policy in Settings; this job keeps the one shown here."}
            </p>

            {request.operation === "move" ? (
              <p className="transfer-dialog__note transfer-dialog__note--calm">
                <HardDrive size={15} strokeWidth={1.75} aria-hidden="true" />
                {preview.sameVolume
                  ? "The source and destination are on the same volume, so this move is a rename and copies nothing."
                  : "The source and destination are on different volumes, so each item is copied and then removed from the source."}
              </p>
            ) : null}
          </div>
        )}

        {error !== null ? (
          <p className="transfer-dialog__error" role="alert">
            <FolderInput size={15} strokeWidth={1.75} aria-hidden="true" />
            {error}
          </p>
        ) : null}

        <footer className="transfer-dialog__actions">
          <Button
            ref={cancelRef}
            variant="secondary"
            onClick={onCancel}
            disabled={busy}
            title="Cancel (Escape)"
          >
            Cancel
          </Button>
          <Button onClick={onConfirm} disabled={busy || preview === null}>
            {busy ? "Starting…" : `Start ${operation.toLowerCase()}`}
          </Button>
        </footer>
      </div>
    </div>
  );
}
