/* ==========================================================================
 * Drives page
 * The storage surface: the volumes the Rust backend detected, with the
 * metadata the platform reported for each one, and a browser for the folders
 * inside them.
 *
 * The page owns the store wiring only. Every filesystem operation (volume
 * probing, path validation, directory listing, the native dialog) happens in
 * Rust behind typed commands; the frontend hands back paths the backend
 * produced and never builds one itself.
 * ========================================================================== */

import { RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui";
import { PageContainer } from "@/layouts";
import { pickDirectory } from "@/services/filesystem-service";
import { toIpcError } from "@/services/ipc";
import { useBrowserStore, useDrivesStore } from "@/stores";
import type { DriveInfo } from "@/types";
import { DirectoryBrowser } from "../components/DirectoryBrowser";
import { VolumeList } from "../components/VolumeList";
import { findVolumeForLocation } from "../presentation";
import "./DrivesPage.css";

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

  const [pickerBusy, setPickerBusy] = useState(false);
  const [pickerError, setPickerError] = useState<string | null>(null);

  useEffect(() => {
    void refreshVolumes();
  }, [refreshVolumes]);

  const selectedVolumeId = findVolumeForLocation(volumes, location)?.id ?? null;

  function handleSelectVolume(volume: DriveInfo): void {
    setPickerError(null);
    // The backend validates the root when it lists it; a volume whose media
    // went away answers with a structured error the browser shows.
    void open(volume.root);
  }

  async function handleOpenFolder(): Promise<void> {
    setPickerBusy(true);
    setPickerError(null);
    try {
      const picked = await pickDirectory();
      if (picked === null) {
        // The user closed the dialog without choosing a folder.
        return;
      }
      await open(picked);
    } catch (failure) {
      setPickerError(toIpcError(failure).message);
    } finally {
      setPickerBusy(false);
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
            pickerBusy={pickerBusy}
            alert={pickerError}
            onBack={() => void goBack()}
            onUp={() => void goUp()}
            onRefresh={() => void refreshDirectory()}
            onOpen={(path) => void open(path)}
            onOpenFolder={() => void handleOpenFolder()}
            onLeave={leave}
          />
        </div>
      </div>
    </PageContainer>
  );
}
