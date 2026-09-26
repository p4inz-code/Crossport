/* ==========================================================================
 * Drives page
 * The storage surface: the volumes the Rust backend detected, with the
 * metadata the platform reported for each one, and a browser for the folders
 * inside them.
 *
 * The page owns the store wiring and the transfer composer. Every filesystem
 * operation (volume probing, path validation, directory listing, the native
 * dialog, planning, starting) happens in Rust behind typed commands; the
 * frontend hands back paths the backend produced and never builds one itself.
 *
 * Composing a transfer is two backend calls, never a local guess:
 * `plan_transfer` reports what would happen — including collisions and free
 * space — and only `start_transfer` queues a job. Choosing a different
 * conflict strategy asks the backend to plan again.
 * ========================================================================== */

import { RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui";
import { TransferDialog } from "@/features/transfers";
import { PageContainer } from "@/layouts";
import { pickDirectory } from "@/services/filesystem-service";
import { toIpcError } from "@/services/ipc";
import { planTransfer, startTransfer } from "@/services/transfer-service";
import { useBrowserStore, useDrivesStore, useTransferStore } from "@/stores";
import {
  type ConflictStrategy,
  DEFAULT_CONFLICT_STRATEGY,
  type DriveInfo,
  type TransferOperation,
  type TransferPreview,
  type TransferRequest,
} from "@/types";
import { DirectoryBrowser } from "../components/DirectoryBrowser";
import { VolumeList } from "../components/VolumeList";
import { findVolumeForLocation } from "../presentation";
import "./DrivesPage.css";

/** A transfer being composed: the request, and its dry run once it arrives. */
interface ComposerState {
  request: TransferRequest;
  preview: TransferPreview | null;
}

export function DrivesPage() {
  const volumes = useDrivesStore((state) => state.drives);
  const volumesStatus = useDrivesStore((state) => state.status);
  const volumesError = useDrivesStore((state) => state.error);
  const refreshVolumes = useDrivesStore((state) => state.refresh);

  const location = useBrowserStore((state) => state.location);
  const listing = useBrowserStore((state) => state.listing);
  const browserStatus = useBrowserStore((state) => state.status);
  const browserError = useBrowserStore((state) => state.error);
  const historyLength = useBrowserStore((state) => state.history.length);
  const open = useBrowserStore((state) => state.open);
  const refreshDirectory = useBrowserStore((state) => state.refresh);
  const goBack = useBrowserStore((state) => state.goBack);
  const goUp = useBrowserStore((state) => state.goUp);
  const leave = useBrowserStore((state) => state.close);

  const trackTransfer = useTransferStore((state) => state.apply);

  /** The "open folder" flow is waiting for the native dialog. */
  const [pickerBusy, setPickerBusy] = useState(false);
  /** A transfer is choosing its destination or being started. */
  const [transferBusy, setTransferBusy] = useState(false);
  const [alertMessage, setAlertMessage] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [composer, setComposer] = useState<ComposerState | null>(null);
  const [composerBusy, setComposerBusy] = useState(false);
  const [composerError, setComposerError] = useState<string | null>(null);

  useEffect(() => {
    void refreshVolumes();
  }, [refreshVolumes]);

  const selectedVolumeId = findVolumeForLocation(volumes, location)?.id ?? null;

  function handleSelectVolume(volume: DriveInfo): void {
    setAlertMessage(null);
    // The backend validates the root when it lists it; a volume whose media
    // went away answers with a structured error the browser shows.
    void open(volume.root);
  }

  async function handleOpenFolder(): Promise<void> {
    setPickerBusy(true);
    setAlertMessage(null);
    try {
      const picked = await pickDirectory();
      if (picked === null) {
        // The user closed the dialog without choosing a folder.
        return;
      }
      await open(picked);
    } catch (failure) {
      setAlertMessage(toIpcError(failure).message);
    } finally {
      setPickerBusy(false);
    }
  }

  /** Plans a request and opens the composer with the result. */
  async function planAndCompose(request: TransferRequest): Promise<void> {
    setComposer({ request, preview: null });
    setComposerBusy(true);
    setComposerError(null);
    setAlertMessage(null);
    try {
      const preview = await planTransfer(request);
      setComposer({ request, preview });
    } catch (failure) {
      // Nothing was queued, so the composer closes and the reason stays on the
      // page instead of behind a dialog that cannot be confirmed.
      setComposer(null);
      setAlertMessage(toIpcError(failure).message);
    } finally {
      setComposerBusy(false);
    }
  }

  /** Asks where a transfer goes, then plans it. */
  async function handleTransferRequest(
    operation: TransferOperation,
    sources: string[],
  ): Promise<void> {
    setTransferBusy(true);
    setAlertMessage(null);
    setNotice(null);
    try {
      const destination = await pickDirectory();
      if (destination === null) {
        return;
      }
      await planAndCompose({
        sources,
        destination,
        operation,
        conflict: DEFAULT_CONFLICT_STRATEGY,
      });
    } catch (failure) {
      setAlertMessage(toIpcError(failure).message);
    } finally {
      setTransferBusy(false);
    }
  }

  async function handleConflictChange(
    conflict: ConflictStrategy,
  ): Promise<void> {
    const current = composer;
    if (current === null) {
      return;
    }
    await planAndCompose({ ...current.request, conflict });
  }

  async function handleConfirm(): Promise<void> {
    const current = composer;
    if (current === null) {
      return;
    }

    setComposerBusy(true);
    setComposerError(null);
    try {
      const snapshot = await startTransfer(current.request);
      // Adopted immediately so the row appears without waiting for an event;
      // the event feed merges the same snapshot by identifier.
      trackTransfer(snapshot);
      setComposer(null);
      setNotice(
        `${snapshot.sources.length} ${
          snapshot.sources.length === 1 ? "item" : "items"
        } queued for ${snapshot.operation}. Follow it on the Transfers page.`,
      );
    } catch (failure) {
      setComposerError(toIpcError(failure).message);
    } finally {
      setComposerBusy(false);
    }
  }

  return (
    <PageContainer
      title="Drives"
      description="Volumes reported by the CrossPort backend, and the folders inside them."
      actions={
        <Button
          variant="secondary"
          onClick={() => void refreshVolumes()}
          disabled={volumesStatus === "loading"}
        >
          <RefreshCw size={16} strokeWidth={1.75} aria-hidden="true" />
          Refresh volumes
        </Button>
      }
    >
      {notice !== null ? (
        <p className="storage-notice" role="status">
          {notice}
        </p>
      ) : null}

      <div className="storage-layout">
        <section
          className="storage-layout__panel"
          aria-labelledby="volumes-heading"
        >
          <h2 className="storage-layout__heading" id="volumes-heading">
            Volumes
          </h2>
          <p className="storage-layout__description">
            Detected, classified, and measured in Rust.
          </p>
          <VolumeList
            volumes={volumes}
            status={volumesStatus}
            error={volumesError}
            selectedId={selectedVolumeId}
            onSelect={handleSelectVolume}
            onRetry={() => void refreshVolumes()}
          />
        </section>

        <div className="storage-layout__browser">
          <DirectoryBrowser
            location={location}
            listing={listing}
            status={browserStatus}
            error={browserError}
            canGoBack={historyLength > 0}
            canGoUp={
              listing !== null &&
              listing.parent !== null &&
              listing.parent !== listing.path
            }
            pickerBusy={pickerBusy || transferBusy}
            transferBusy={transferBusy || composer !== null}
            alert={alertMessage}
            onBack={() => void goBack()}
            onUp={() => void goUp()}
            onRefresh={() => void refreshDirectory()}
            onOpen={(path) => void open(path)}
            onOpenFolder={() => void handleOpenFolder()}
            onTransferRequest={(operation, sources) =>
              void handleTransferRequest(operation, sources)
            }
            onLeave={leave}
          />
        </div>
      </div>

      {composer !== null ? (
        <TransferDialog
          request={composer.request}
          preview={composer.preview}
          busy={composerBusy}
          error={composerError}
          onConflictChange={(conflict) => void handleConflictChange(conflict)}
          onConfirm={() => void handleConfirm()}
          onCancel={() => {
            setComposer(null);
            setComposerError(null);
          }}
        />
      ) : null}
    </PageContainer>
  );
}
