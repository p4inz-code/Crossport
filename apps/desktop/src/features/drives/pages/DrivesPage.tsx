/* ==========================================================================
 * Drives page
 * Lists the storage roots the Rust backend enumerated, and exercises the
 * native folder picker plus path validation foundation end to end.
 *
 * Phase 2 extends this page with volume kinds, capacity, and filesystem
 * reporting; the IPC and path layers it needs already exist.
 * ========================================================================== */

import {
  AlertTriangle,
  FolderSearch,
  HardDrive,
  RefreshCw,
} from "lucide-react";
import { useEffect, useState } from "react";

import {
  Button,
  Card,
  CardBody,
  CardHeader,
  EmptyState,
  LoadingState,
  Section,
} from "@/components/ui";
import { PageContainer } from "@/layouts";
import { formatBytes, formatDateTime } from "@/lib";
import { inspectPath, pickDirectory } from "@/services/filesystem-service";
import { toIpcError } from "@/services/ipc";
import { useDrivesStore } from "@/stores";
import type { EntryMetadata } from "@/types";
import "./DrivesPage.css";

export function DrivesPage() {
  const drives = useDrivesStore((state) => state.drives);
  const status = useDrivesStore((state) => state.status);
  const error = useDrivesStore((state) => state.error);
  const refresh = useDrivesStore((state) => state.refresh);

  const [selection, setSelection] = useState<EntryMetadata | null>(null);
  const [selectionError, setSelectionError] = useState<string | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function handlePickFolder(): Promise<void> {
    setDialogOpen(true);
    setSelectionError(null);
    try {
      const picked = await pickDirectory();
      if (picked === null) {
        // The user closed the dialog without choosing a folder.
        return;
      }
      setSelection(await inspectPath(picked));
    } catch (failure) {
      setSelection(null);
      setSelectionError(toIpcError(failure).message);
    } finally {
      setDialogOpen(false);
    }
  }

  return (
    <PageContainer
      title="Drives"
      description="Storage roots reported by the CrossPort backend."
      actions={
        <Button
          variant="secondary"
          onClick={() => void refresh()}
          disabled={status === "loading"}
        >
          <RefreshCw size={16} strokeWidth={1.75} aria-hidden="true" />
          Refresh
        </Button>
      }
    >
      <Section
        title="Detected drives"
        description="Enumerated in Rust and validated before the list reaches the UI."
      >
        {status === "loading" && drives.length === 0 ? (
          <LoadingState label="Enumerating drives…" />
        ) : null}

        {error !== null ? (
          <div role="alert">
            <EmptyState
              icon={AlertTriangle}
              title="Drive enumeration failed"
              description={error.message}
              action={<Button onClick={() => void refresh()}>Try again</Button>}
            />
          </div>
        ) : null}

        {error === null && status === "ready" && drives.length === 0 ? (
          <EmptyState
            icon={HardDrive}
            title="No drives detected"
            description="The backend found no readable storage roots on this machine."
          />
        ) : null}

        {drives.length > 0 ? (
          <ul className="drives-list">
            {drives.map((drive) => (
              <li key={drive.root} className="drives-list__item">
                <HardDrive size={18} strokeWidth={1.75} aria-hidden="true" />
                <span className="drives-list__label">{drive.label}</span>
                <span className="drives-list__root">{drive.root}</span>
              </li>
            ))}
          </ul>
        ) : null}
      </Section>

      <Section
        title="Path check"
        description="Picks a folder with the native dialog and validates it in Rust — the webview never touches a path itself."
      >
        <Card>
          <CardHeader
            title="Folder validation"
            description="The selected path is normalized, must be absolute, and must not escape its root."
            action={
              <Button
                variant="secondary"
                onClick={() => void handlePickFolder()}
                disabled={dialogOpen}
              >
                <FolderSearch size={16} strokeWidth={1.75} aria-hidden="true" />
                {dialogOpen
                  ? "Waiting for the dialog…"
                  : "Browse for a folder…"}
              </Button>
            }
          />
          <CardBody>
            {selectionError !== null ? (
              <p className="path-error" role="alert">
                {selectionError}
              </p>
            ) : null}

            {selection === null && selectionError === null ? (
              <p className="path-hint">
                No folder selected yet. The picker is a native Windows dialog
                hosted by the backend.
              </p>
            ) : null}

            {selection !== null ? (
              <dl className="path-details">
                <div className="path-details__row">
                  <dt>Path</dt>
                  <dd>{selection.path}</dd>
                </div>
                <div className="path-details__row">
                  <dt>Name</dt>
                  <dd>{selection.name}</dd>
                </div>
                <div className="path-details__row">
                  <dt>Kind</dt>
                  <dd>
                    {selection.isDir
                      ? "Directory"
                      : selection.isFile
                        ? "File"
                        : "Other"}
                    {selection.isSymlink ? " (symlink)" : ""}
                  </dd>
                </div>
                <div className="path-details__row">
                  <dt>Size</dt>
                  <dd>{formatBytes(selection.sizeBytes)}</dd>
                </div>
                <div className="path-details__row">
                  <dt>Modified</dt>
                  <dd>{formatDateTime(selection.modifiedMs)}</dd>
                </div>
              </dl>
            ) : null}
          </CardBody>
        </Card>
      </Section>
    </PageContainer>
  );
}
