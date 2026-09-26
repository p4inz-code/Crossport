/* ==========================================================================
 * VolumeList component
 * The detected volumes with the metadata the backend reported for each one:
 * label, root, kind, filesystem, capacity, and reachability. Presentational
 * only — the page owns the store wiring.
 * ========================================================================== */

import { AlertTriangle, HardDrive } from "lucide-react";

import { Button, EmptyState, LoadingState } from "@/components/ui";
import { cn } from "@/lib";
import type { IpcError } from "@/services/ipc";
import type { DriveInfo, LoadStatus } from "@/types";
import {
  capacityPercent,
  capacitySummary,
  VOLUME_KIND_ICONS,
  volumeKindLabel,
  volumeNotes,
} from "../presentation";
import "./VolumeList.css";

interface VolumeListProps {
  volumes: DriveInfo[];
  status: LoadStatus;
  error: IpcError | null;
  /** Identifier of the volume the current location belongs to. */
  selectedId: string | null;
  onSelect: (volume: DriveInfo) => void;
  onRetry: () => void;
}

export function VolumeList({
  volumes,
  status,
  error,
  selectedId,
  onSelect,
  onRetry,
}: VolumeListProps) {
  return (
    <div className="volume-list">
      {status === "loading" && volumes.length === 0 ? (
        <LoadingState label="Detecting volumes…" />
      ) : null}

      {error !== null ? (
        <div role="alert">
          <EmptyState
            icon={AlertTriangle}
            title="Volume detection failed"
            description={error.message}
            action={<Button onClick={onRetry}>Try again</Button>}
          />
        </div>
      ) : null}

      {error === null && status === "ready" && volumes.length === 0 ? (
        <EmptyState
          icon={HardDrive}
          title="No volumes detected"
          description="The backend found no readable storage volumes on this machine."
        />
      ) : null}

      {volumes.length > 0 ? (
        <ul className="volume-list__items">
          {volumes.map((volume) => (
            <VolumeCard
              key={volume.id}
              volume={volume}
              selected={volume.id === selectedId}
              onSelect={onSelect}
            />
          ))}
        </ul>
      ) : null}
    </div>
  );
}

interface VolumeCardProps {
  volume: DriveInfo;
  selected: boolean;
  onSelect: (volume: DriveInfo) => void;
}

function VolumeCard({ volume, selected, onSelect }: VolumeCardProps) {
  const Icon = VOLUME_KIND_ICONS[volume.kind];
  const percent = capacityPercent(volume);
  const notes = volumeNotes(volume);

  return (
    <li>
      <button
        type="button"
        className={cn("volume-card", selected && "volume-card--selected")}
        aria-current={selected ? "true" : undefined}
        onClick={() => onSelect(volume)}
      >
        <span className="volume-card__heading">
          <Icon size={18} strokeWidth={1.75} aria-hidden="true" />
          <span className="volume-card__label">{volume.label}</span>
          {notes.map((note) => (
            <span key={note} className="volume-card__note">
              {note}
            </span>
          ))}
        </span>

        <span className="volume-card__root">{volume.root}</span>

        <span className="volume-card__meta">
          <span>{volumeKindLabel(volume.kind)}</span>
          {volume.filesystem !== null ? (
            <span className="volume-card__filesystem">{volume.filesystem}</span>
          ) : null}
        </span>

        <span className="volume-card__capacity">
          {percent !== null ? (
            <span className="volume-card__meter" aria-hidden="true">
              <span
                className="volume-card__meter-fill"
                style={{ width: `${percent}%` }}
              />
            </span>
          ) : null}
          <span className="volume-card__capacity-text">
            {capacitySummary(volume)}
          </span>
        </span>
      </button>
    </li>
  );
}
