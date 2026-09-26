/* ==========================================================================
 * Drives presentation helpers
 * Framework-free mapping from backend values to what the storage surface
 * shows: labels, icons, capacity arithmetic, and which volume a location
 * belongs to. Kept out of the components so it is directly testable and so no
 * component invents its own wording for a backend value.
 * ========================================================================== */

import type { LucideIcon } from "lucide-react";
import {
  Disc,
  File,
  FileQuestion,
  Folder,
  HardDrive,
  Link2,
  MemoryStick,
  Network,
  Usb,
} from "lucide-react";

import { formatBytes } from "@/lib";
import type { DirectoryEntry, DriveInfo, EntryKind, VolumeKind } from "@/types";

/** Human wording for each volume kind. */
export const VOLUME_KIND_LABELS: Record<VolumeKind, string> = {
  fixed: "Internal disk",
  removable: "Removable storage",
  network: "Network location",
  optical: "Optical drive",
  ram: "RAM disk",
  unknown: "Unknown type",
};

/** Human wording for each entry kind. */
export const ENTRY_KIND_LABELS: Record<EntryKind, string> = {
  directory: "Folder",
  file: "File",
  symlink: "Symlink",
  other: "Other",
};

/**
 * Icon for each volume kind. Keyed by kind, so a new backend kind cannot be
 * added without giving it an icon.
 */
export const VOLUME_KIND_ICONS: Record<VolumeKind, LucideIcon> = {
  fixed: HardDrive,
  removable: Usb,
  network: Network,
  optical: Disc,
  ram: MemoryStick,
  unknown: HardDrive,
};

/** Icon for each entry kind, keyed the same way. */
export const ENTRY_KIND_ICONS: Record<EntryKind, LucideIcon> = {
  directory: Folder,
  file: File,
  symlink: Link2,
  other: FileQuestion,
};

export function volumeKindLabel(kind: VolumeKind): string {
  return VOLUME_KIND_LABELS[kind];
}

export function entryKindLabel(kind: EntryKind): string {
  return ENTRY_KIND_LABELS[kind];
}

/**
 * How full a volume is, as a percentage. `null` when the platform did not
 * report the capacity, which is not the same as an empty volume.
 */
export function capacityPercent(volume: DriveInfo): number | null {
  const { totalBytes, usedBytes } = volume;
  if (totalBytes === null || usedBytes === null || totalBytes <= 0) {
    return null;
  }
  return Math.min(100, Math.max(0, Math.round((usedBytes / totalBytes) * 100)));
}

/** `28.4 GB free of 476 GB`, or an honest statement that it is unknown. */
export function capacitySummary(volume: DriveInfo): string {
  const { freeBytes, totalBytes } = volume;
  if (freeBytes === null || totalBytes === null || totalBytes <= 0) {
    return "Capacity unavailable";
  }
  return `${formatBytes(freeBytes)} free of ${formatBytes(totalBytes)}`;
}

/**
 * Status notes worth showing next to a volume: whether it is reachable right
 * now and whether it rejects writes. Returns an empty list when the platform
 * reported nothing unusual.
 */
export function volumeNotes(volume: DriveInfo): string[] {
  const notes: string[] = [];
  if (!volume.mounted) {
    notes.push("Not available");
  }
  if (volume.readonly === true) {
    notes.push("Read-only");
  }
  return notes;
}

/** Whether an entry can be opened as a directory by the backend. */
export function isOpenableDirectory(entry: DirectoryEntry): boolean {
  return entry.kind === "directory" || entry.kind === "symlink";
}

/**
 * Whether the transfer engine will copy an entry.
 *
 * Links and other special entries are reported by the engine but never
 * followed or copied, so the browser says so before the user asks for a
 * transfer rather than after.
 */
export function isCopyable(entry: DirectoryEntry): boolean {
  return entry.kind === "directory" || entry.kind === "file";
}

/**
 * `3 items · 12.4 MB` for a selection, without inventing sizes the backend did
 * not report, and naming anything that cannot be copied.
 */
export function selectionSummary(entries: DirectoryEntry[]): string {
  const count = entries.length;
  const parts = [`${count} ${count === 1 ? "item" : "items"}`];

  const measured = entries.filter((entry) => entry.sizeBytes !== null);
  if (measured.length === 0) {
    parts.push("size not reported");
  } else {
    const bytes = measured.reduce(
      (total, entry) => total + (entry.sizeBytes ?? 0),
      0,
    );
    const partial = measured.length < count ? "at least " : "";
    parts.push(`${partial}${formatBytes(bytes)}`);
  }

  const skipped = entries.filter((entry) => !isCopyable(entry)).length;
  if (skipped > 0) {
    parts.push(
      `${skipped} ${skipped === 1 ? "entry is" : "entries are"} not copied`,
    );
  }

  return parts.join(" · ");
}

/**
 * The volume a location lives in, or `null` when no reported volume contains
 * it (a UNC path, or a drive that was unplugged since the last refresh).
 *
 * Comparison is case-insensitive: Windows paths are, and the lowercase form
 * is only used to highlight a volume, never to construct a path.
 */
export function findVolumeForLocation(
  volumes: DriveInfo[],
  location: string | null,
): DriveInfo | null {
  if (location === null) {
    return null;
  }

  const target = location.toLowerCase();
  let best: DriveInfo | null = null;
  let bestLength = -1;

  for (const volume of volumes) {
    // `/` has no trailing separator to strip and contains every path.
    const root = volume.root.replace(/[\\/]+$/, "").toLowerCase();
    const contains =
      root.length === 0 ||
      target === root ||
      target.startsWith(`${root}/`) ||
      target.startsWith(`${root}\\`);

    // Prefer the most specific match: the longest root that contains the
    // location, and the first reported one when two are equally specific.
    if (contains && root.length > bestLength) {
      best = volume;
      bestLength = root.length;
    }
  }

  return best;
}
