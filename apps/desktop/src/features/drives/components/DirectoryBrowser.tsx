/* ==========================================================================
 * DirectoryBrowser component
 * The filesystem surface: navigation controls, the current location, the
 * entries of the listed directory, and the loading, empty, and error states.
 *
 * Presentational only. It renders the listing the backend returned and asks
 * the page to open paths or start a transfer — it never builds or validates a
 * path itself, and the paths it reports are the ones the backend produced.
 *
 * Two selections coexist on purpose: checking rows picks the items a transfer
 * will move, while clicking a row shows its details. Selecting a folder's name
 * still opens it, so a folder can be both browsed and moved.
 * ========================================================================== */

import {
  AlertTriangle,
  ArrowLeft,
  ArrowUp,
  ChevronRight,
  Copy,
  FolderInput,
  FolderSearch,
  FolderX,
  RefreshCw,
} from "lucide-react";
import { type KeyboardEvent as ReactKeyboardEvent, useState } from "react";

import { Button, EmptyState, LoadingState } from "@/components/ui";
import { cn, formatBytes, formatDateTime } from "@/lib";
import type { IpcError } from "@/services/ipc";
import type {
  DirectoryEntry,
  DirectoryListing,
  LoadStatus,
  TransferOperation,
} from "@/types";
import {
  ENTRY_KIND_ICONS,
  entryKindLabel,
  isOpenableDirectory,
  selectionSummary,
} from "../presentation";
import "./DirectoryBrowser.css";

interface DirectoryBrowserProps {
  /** Absolute directory currently shown, or `null` when nothing is open. */
  location: string | null;
  listing: DirectoryListing | null;
  status: LoadStatus;
  error: IpcError | null;
  canGoBack: boolean;
  canGoUp: boolean;
  /** The native folder picker is open. */
  pickerBusy: boolean;
  /** A transfer is being composed: picking a destination, or planning one. */
  transferBusy: boolean;
  /** Message about an action on this folder, kept apart from listing failures. */
  alert: string | null;
  onBack: () => void;
  onUp: () => void;
  onRefresh: () => void;
  onOpen: (path: string) => void;
  onOpenFolder: () => void;
  onTransferRequest: (operation: TransferOperation, sources: string[]) => void;
  onLeave: () => void;
}

export function DirectoryBrowser({
  location,
  listing,
  status,
  error,
  canGoBack,
  canGoUp,
  pickerBusy,
  transferBusy,
  alert,
  onBack,
  onUp,
  onRefresh,
  onOpen,
  onOpenFolder,
  onTransferRequest,
  onLeave,
}: DirectoryBrowserProps) {
  // A single row is highlighted for its details; a separate set of checkboxes
  // picks what a transfer moves.
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [checkedPaths, setCheckedPaths] = useState<readonly string[]>([]);

  // Both are stored as paths, so an entry that disappears with the next
  // listing simply stops being selected, and no stale path can be transferred.
  const selectedEntry =
    listing?.entries.find((entry) => entry.path === selectedPath) ?? null;
  const checkedEntries =
    listing?.entries.filter((entry) => checkedPaths.includes(entry.path)) ?? [];
  const checkedSources = checkedEntries.map((entry) => entry.path);
  const allChecked =
    listing !== null &&
    listing.entries.length > 0 &&
    checkedEntries.length === listing.entries.length;

  function toggleChecked(path: string): void {
    setCheckedPaths((current) =>
      current.includes(path)
        ? current.filter((entry) => entry !== path)
        : [...current, path],
    );
  }

  function activate(entry: DirectoryEntry, force: boolean): void {
    if (isOpenableDirectory(entry) && (force || entry.kind === "directory")) {
      setSelectedPath(null);
      onOpen(entry.path);
      return;
    }
    setSelectedPath(entry.path);
  }

  function handleKeyDown(event: ReactKeyboardEvent<HTMLElement>): void {
    if (!event.altKey) {
      return;
    }
    if (event.key === "ArrowLeft" && canGoBack) {
      event.preventDefault();
      onBack();
    }
    if (event.key === "ArrowUp" && canGoUp) {
      event.preventDefault();
      onUp();
    }
  }

  return (
    <section
      className="browser"
      aria-label="Directory browser"
      onKeyDown={handleKeyDown}
    >
      <div className="browser__toolbar">
        <div className="browser__controls">
          <Button
            size="sm"
            variant="secondary"
            onClick={onBack}
            disabled={!canGoBack}
          >
            <ArrowLeft size={15} strokeWidth={1.75} aria-hidden="true" />
            Back
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={onUp}
            disabled={!canGoUp}
          >
            <ArrowUp size={15} strokeWidth={1.75} aria-hidden="true" />
            Up
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={onRefresh}
            disabled={location === null || status === "loading"}
          >
            <RefreshCw size={15} strokeWidth={1.75} aria-hidden="true" />
            Refresh
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={onOpenFolder}
            disabled={pickerBusy || transferBusy}
          >
            <FolderSearch size={15} strokeWidth={1.75} aria-hidden="true" />
            {pickerBusy ? "Waiting for the dialog…" : "Open folder…"}
          </Button>
          <Button
            size="sm"
            onClick={() => onTransferRequest("copy", checkedSources)}
            disabled={checkedSources.length === 0 || transferBusy}
          >
            <Copy size={15} strokeWidth={1.75} aria-hidden="true" />
            Copy to…
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => onTransferRequest("move", checkedSources)}
            disabled={checkedSources.length === 0 || transferBusy}
          >
            <FolderInput size={15} strokeWidth={1.75} aria-hidden="true" />
            Move to…
          </Button>
        </div>

        <div className="browser__location">
          <span className="browser__location-label">Location</span>
          <span className="browser__location-path">
            {location ?? "No folder selected"}
          </span>
        </div>
      </div>

      {alert !== null ? (
        <p className="browser__alert" role="alert">
          {alert}
        </p>
      ) : null}

      <div className="browser__body">
        {location === null ? (
          <EmptyState
            icon={FolderSearch}
            title="No folder open"
            description="Choose a volume on the left to browse it, or open any folder with the native dialog."
          />
        ) : null}

        {location !== null && status === "loading" && listing === null ? (
          <LoadingState label="Reading folder…" />
        ) : null}

        {location !== null && error !== null ? (
          <div role="alert">
            <EmptyState
              icon={AlertTriangle}
              title="Cannot open this folder"
              description={error.message}
              action={
                <div className="browser__recovery">
                  <Button onClick={onRefresh}>Try again</Button>
                  <Button variant="secondary" onClick={onLeave}>
                    Back to volumes
                  </Button>
                </div>
              }
            />
          </div>
        ) : null}

        {location !== null && error === null && listing !== null ? (
          listing.entries.length === 0 ? (
            <EmptyState
              icon={FolderX}
              title="This folder is empty"
              description={listing.path}
            />
          ) : (
            <>
              {listing.truncated ? (
                <p className="browser__notice" role="status">
                  Showing the first {listing.entries.length} entries. This
                  folder holds more than CrossPort lists at once.
                </p>
              ) : null}
              <table className="browser__table">
                <thead>
                  <tr>
                    <th scope="col" className="browser__cell--select">
                      <input
                        type="checkbox"
                        aria-label="Select all entries"
                        checked={allChecked}
                        ref={(node) => {
                          if (node !== null) {
                            node.indeterminate =
                              checkedEntries.length > 0 && !allChecked;
                          }
                        }}
                        onChange={() =>
                          setCheckedPaths(
                            allChecked
                              ? []
                              : listing.entries.map((entry) => entry.path),
                          )
                        }
                      />
                    </th>
                    <th scope="col">Name</th>
                    <th scope="col" className="browser__cell--size">
                      Size
                    </th>
                    <th scope="col" className="browser__cell--modified">
                      Modified
                    </th>
                    <th scope="col" className="browser__cell--kind">
                      Kind
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {listing.entries.map((entry) => (
                    <EntryRow
                      key={entry.path}
                      entry={entry}
                      selected={entry.path === selectedPath}
                      checked={checkedPaths.includes(entry.path)}
                      onToggleChecked={toggleChecked}
                      onActivate={activate}
                    />
                  ))}
                </tbody>
              </table>
            </>
          )
        ) : null}
      </div>

      <div className="browser__status">
        {checkedEntries.length > 0 ? (
          <span className="browser__selection">
            <span>{selectionSummary(checkedEntries)}</span>
            <Button
              size="sm"
              variant="ghost"
              onClick={() => setCheckedPaths([])}
            >
              Clear selection
            </Button>
          </span>
        ) : selectedEntry !== null ? (
          <span>
            Selected <strong>{selectedEntry.name}</strong> ·{" "}
            {entryKindLabel(selectedEntry.kind)} ·{" "}
            {selectedEntry.sizeBytes === null
              ? "size not reported"
              : formatBytes(selectedEntry.sizeBytes)}{" "}
            · modified {formatDateTime(selectedEntry.modifiedMs)}
          </span>
        ) : listing !== null ? (
          <Summary listing={listing} />
        ) : (
          <span>No folder loaded.</span>
        )}
      </div>
    </section>
  );
}

interface EntryRowProps {
  entry: DirectoryEntry;
  selected: boolean;
  checked: boolean;
  onToggleChecked: (path: string) => void;
  onActivate: (entry: DirectoryEntry, force: boolean) => void;
}

function EntryRow({
  entry,
  selected,
  checked,
  onToggleChecked,
  onActivate,
}: EntryRowProps) {
  const Icon = ENTRY_KIND_ICONS[entry.kind];
  const isDirectory = entry.kind === "directory";

  return (
    <tr className={cn("browser__row", selected && "browser__row--selected")}>
      <td className="browser__cell--select">
        <input
          type="checkbox"
          aria-label={`Select ${entry.name}`}
          checked={checked}
          onChange={() => onToggleChecked(entry.path)}
        />
      </td>
      <th scope="row" className="browser__cell--name">
        <button
          type="button"
          className="browser__entry"
          aria-current={selected ? "true" : undefined}
          onClick={() => onActivate(entry, false)}
          onDoubleClick={() => onActivate(entry, true)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              onActivate(entry, true);
            }
          }}
        >
          <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
          <span className="browser__entry-name">{entry.name}</span>
          {isDirectory ? (
            <ChevronRight size={14} strokeWidth={1.75} aria-hidden="true" />
          ) : null}
        </button>
      </th>
      <td className="browser__cell--size">
        {entry.sizeBytes === null ? "—" : formatBytes(entry.sizeBytes)}
      </td>
      <td className="browser__cell--modified">
        {formatDateTime(entry.modifiedMs)}
      </td>
      <td className="browser__cell--kind">
        {entryKindLabel(entry.kind)}
        {entry.readonly === true ? " · read-only" : ""}
      </td>
    </tr>
  );
}

function Summary({ listing }: { listing: DirectoryListing }) {
  const folders = listing.entries.filter(
    (entry) => entry.kind === "directory",
  ).length;
  const files = listing.entries.filter((entry) => entry.kind === "file").length;
  const others = listing.entries.length - folders - files;

  return (
    <span>
      {folders} {folders === 1 ? "folder" : "folders"} · {files}{" "}
      {files === 1 ? "file" : "files"}
      {others > 0 ? ` · ${others} other` : ""}
    </span>
  );
}
