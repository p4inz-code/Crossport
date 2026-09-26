/* ==========================================================================
 * Test fixtures
 * Backend payloads used by more than one test suite, so a contract change is
 * made in one place instead of drifting between tests.
 * ========================================================================== */

import type { DirectoryEntry, DirectoryListing, DriveInfo } from "@/types";

const GIB = 1024 ** 3;

/** A mounted NTFS system volume; override anything a test cares about. */
export function makeVolume(overrides: Partial<DriveInfo> = {}): DriveInfo {
  return {
    id: "C:",
    root: "C:\\",
    label: "C:",
    name: null,
    kind: "fixed",
    filesystem: "NTFS",
    totalBytes: 500 * GIB,
    freeBytes: 120 * GIB,
    usedBytes: 380 * GIB,
    readonly: false,
    mounted: true,
    ...overrides,
  };
}

/** A file entry; override `kind`/`sizeBytes` for directories and links. */
export function makeEntry(
  overrides: Partial<DirectoryEntry> = {},
): DirectoryEntry {
  return {
    name: "notes.txt",
    path: "C:\\notes.txt",
    kind: "file",
    sizeBytes: 2048,
    modifiedMs: 1_700_000_000_000,
    readonly: false,
    ...overrides,
  };
}

/** A listing with a folder and a file, both on the system volume. */
export function makeListing(
  overrides: Partial<DirectoryListing> = {},
): DirectoryListing {
  return {
    path: "C:\\",
    name: "C:\\",
    parent: null,
    entries: [
      makeEntry({
        name: "Users",
        path: "C:\\Users",
        kind: "directory",
        sizeBytes: null,
      }),
      makeEntry({ name: "notes.txt", path: "C:\\notes.txt" }),
    ],
    truncated: false,
    ...overrides,
  };
}
