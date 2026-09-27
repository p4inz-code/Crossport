/* ==========================================================================
 * DirectoryBrowser component
 * The filesystem surface: navigation controls, the breadcrumb trail, the
 * entries of the listed directory, and the loading, empty, and error states.
 *
 * Presentational only. It renders the listing the backend returned and asks
 * the page to open paths or start a transfer — it never builds or validates a
 * path itself, and the paths it reports are the ones the backend produced,
 * breadcrumbs included.
 *
 * Two selections coexist on purpose: checking rows picks the items a transfer
 * will move (the source selection), while clicking a row shows its details.
 * Selecting a folder's name still opens it, so a folder can be both browsed and
 * moved.
 *
 * Keyboard navigation is a roving focus: one row is in the tab order at a time,
 * the arrow keys move between rows, Space toggles the row's selection, and Enter
 * opens a folder. That keeps a directory with thousands of entries usable
 * without thousands of tab stops.
 * ========================================================================== */

import {
  AlertTriangle,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  ChevronRight,
  Copy,
  FolderInput,
  FolderSearch,
  FolderX,
  RefreshCw,
} from "lucide-react";
import {
  type KeyboardEvent as ReactKeyboardEvent,
  useRef,
  useState,
} from "react";

import { Button, EmptyState, LoadingState } from "@/components/ui";
import { cn, formatBytes, formatDateTime } from "@/lib";
import type { IpcError } from "@/services/ipc";
import type {
  DirectoryEntry,
  DirectoryListing,
  LoadStatus,
  PathAncestor,
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
  /** The current location and its parents, oldest first. */
  trail: PathAncestor[];
  listing: DirectoryListing | null;
  status: LoadStatus;
  error: IpcError | null;
  canGoBack: boolean;
  canGoForward: boolean;
  canGoUp: boolean;
  /** The native folder picker is open. */
  pickerBusy: boolean;
  /** A transfer is being composed: picking a destination, or planning one. */
  transferBusy: boolean;
  /** Message about an action on this folder, kept apart from listing failures. */
  alert: string | null;
  onBack: () => void;
  onForward: () => void;
  onUp: () => void;
  onRefresh: () => void;
  onOpen: (path: string) => void;
  onOpenFolder: () => void;
  onTransferRequest: (operation: TransferOperation, sources: string[]) => void;
  onLeave: () => void;
}

export function DirectoryBrowser({
  location,
  trail,
  listing,
  status,
  error,
  canGoBack,
  canGoForward,
  canGoUp,
  pickerBusy,
  transferBusy,
  alert,
  onBack,
  onForward,
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
  // The row holding keyboard focus, so one row is tabbable at a time.
  const [focusedPath, setFocusedPath] = useState<string | null>(null);
  const rowRefs = useRef(new Map<string, HTMLButtonElement>());

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

  /** Moves the keyboard focus and the highlighted row by `delta` rows. */
  function moveFocus(fromPath: string | null, delta: number): void {
    const entries = listing?.entries ?? [];
    if (entries.length === 0) {
      return;
    }
    const index =
      fromPath === null
        ? -1
        : entries.findIndex((entry) => entry.path === fromPath);
    const target =
      entries[Math.min(Math.max(index + delta, 0), entries.length - 1)];
    if (target === undefined) {
      return;
    }
    setSelectedPath(target.path);
    setFocusedPath(target.path);
    rowRefs.current.get(target.path)?.focus();
  }

  function handleRowKey(
    event: ReactKeyboardEvent<HTMLElement>,
    entry: DirectoryEntry,
  ): void {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      moveFocus(entry.path, event.key === "ArrowDown" ? 1 : -1);
      return;
    }
    if (event.key === " ") {
      event.preventDefault();
      toggleChecked(entry.path);
    }
  }

  function handleKeyDown(event: ReactKeyboardEvent<HTMLElement>): void {
    if (!event.altKey) {
      return;
    }
    if (event.key === "ArrowLeft" && canGoBack) {
      event.preventDefault();
      onBack();
    }
    if (event.key === "ArrowRight" && canGoForward) {
      event.preventDefault();
      onForward();
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
            title="Back (Alt+Left)"
          >
            <ArrowLeft size={15} strokeWidth={1.75} aria-hidden="true" />
            Back
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={onForward}
            disabled={!canGoForward}
            title="Forward (Alt+Right)"
          >
            <ArrowRight size={15} strokeWidth={1.75} aria-hidden="true" />
            Forward
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={onUp}
            disabled={!canGoUp}
            title="Up one folder (Alt+Up)"
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
        </div>

        {location !== null ? (
          <div className="browser__trail">
            <nav className="browser__crumbs" aria-label="Breadcrumb">
              {trail.map((step, index) => {
                const current = index === trail.length - 1;
                return (
                  <span key={step.path} className="browser__crumb">
                    {index > 0 ? (
                      <ChevronRight
                        size={14}
                        strokeWidth={1.75}
                        aria-hidden="true"
                        className="browser__crumb-separator"
                      />
                    ) : null}
                    <button
                      type="button"
                      className={cn(
                        "browser__crumb-button",
                        current && "browser__crumb-button--current",
                      )}
                      aria-current={current ? "page" : undefined}
                      title={step.path}
                      disabled={current}
                      onClick={() => onOpen(step.path)}
                    >
                      {step.label}
                    </button>
                  </span>
                );
              })}
              {trail.length === 0 ? (
                <span className="browser__crumb-fallback">{location}</span>
              ) : null}
            </nav>
            <span className="browser__source-label">
              Selection is the source
            </span>
          </div>
        ) : null}
      </div>

      <div className="browser__actions">
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
        <p className="browser__actions-hint">
          Pick a destination in the dialog; the backend plans the transfer and
          reports what it would do before anything moves.
        </p>
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
              description={`Nothing to show in ${listing.path}.`}
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
                  {listing.entries.map((entry, index) => (
                    <EntryRow
                      key={entry.path}
                      entry={entry}
                      selected={entry.path === selectedPath}
                      checked={checkedPaths.includes(entry.path)}
                      // One tab stop: the focused row, or the first one.
                      tabbable={
                        focusedPath === null
                          ? index === 0
                          : entry.path === focusedPath
                      }
                      registerRef={(node) => {
                        if (node === null) {
                          rowRefs.current.delete(entry.path);
                        } else {
                          rowRefs.current.set(entry.path, node);
                        }
                      }}
                      onToggleChecked={toggleChecked}
                      onActivate={activate}
                      onKey={handleRowKey}
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
            <span className="browser__keyboard-hint">
              Space toggles a row, Enter opens a folder
            </span>
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
          <span>
            {location === null ? "No folder selected" : "No folder loaded."}
          </span>
        )}
      </div>
    </section>
  );
}

interface EntryRowProps {
  entry: DirectoryEntry;
  selected: boolean;
  checked: boolean;
  tabbable: boolean;
  registerRef: (node: HTMLButtonElement | null) => void;
  onToggleChecked: (path: string) => void;
  onActivate: (entry: DirectoryEntry, force: boolean) => void;
  onKey: (
    event: ReactKeyboardEvent<HTMLElement>,
    entry: DirectoryEntry,
  ) => void;
}

function EntryRow({
  entry,
  selected,
  checked,
  tabbable,
  registerRef,
  onToggleChecked,
  onActivate,
  onKey,
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
          // Out of the tab order on purpose: Space on the focused row toggles
          // it, so a folder with thousands of entries is still navigable.
          tabIndex={-1}
          onChange={() => onToggleChecked(entry.path)}
        />
      </td>
      <th scope="row" className="browser__cell--name">
        <button
          type="button"
          className="browser__entry"
          ref={registerRef}
          tabIndex={tabbable ? 0 : -1}
          aria-current={selected ? "true" : undefined}
          onClick={() => onActivate(entry, false)}
          onDoubleClick={() => onActivate(entry, true)}
          onKeyDown={(event) => {
            onKey(event, entry);
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
    <span className="browser__summary">
      {/* The counts stay in their own node: a test, a screen reader, or a
          future caller can read exactly how many of each kind are shown. */}
      <span>
        {folders} {folders === 1 ? "folder" : "folders"} · {files}{" "}
        {files === 1 ? "file" : "files"}
        {others > 0 ? ` · ${others} other` : ""}
      </span>
      <span className="browser__keyboard-hint">
        ↑/↓ move, Space selects, Enter opens
      </span>
    </span>
  );
}
