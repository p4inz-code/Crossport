import { describe, expect, it } from "vitest";

import { makeEntry, makeVolume } from "@/test/fixtures";
import {
  capacityPercent,
  capacitySummary,
  ENTRY_KIND_ICONS,
  ENTRY_KIND_LABELS,
  entryKindLabel,
  findVolumeForLocation,
  isOpenableDirectory,
  VOLUME_KIND_ICONS,
  VOLUME_KIND_LABELS,
  volumeKindLabel,
  volumeNotes,
} from "./presentation";

const GIB = 1024 ** 3;

describe("volume presentation", () => {
  it("labels every volume kind", () => {
    expect(volumeKindLabel("fixed")).toBe("Internal disk");
    expect(volumeKindLabel("removable")).toBe("Removable storage");
    expect(volumeKindLabel("network")).toBe("Network location");
    expect(volumeKindLabel("optical")).toBe("Optical drive");
    expect(volumeKindLabel("ram")).toBe("RAM disk");
    expect(volumeKindLabel("unknown")).toBe("Unknown type");
  });

  it("gives every volume kind an icon", () => {
    expect(Object.keys(VOLUME_KIND_ICONS).sort()).toEqual(
      Object.keys(VOLUME_KIND_LABELS).sort(),
    );
    for (const icon of Object.values(VOLUME_KIND_ICONS)) {
      expect(icon).toBeDefined();
    }
  });

  it("computes the used percentage", () => {
    expect(
      capacityPercent(makeVolume({ totalBytes: 100, usedBytes: 25 })),
    ).toBe(25);
    expect(capacityPercent(makeVolume({ totalBytes: 3, usedBytes: 2 }))).toBe(
      67,
    );
  });

  it("refuses to invent a percentage without capacity", () => {
    expect(
      capacityPercent(
        makeVolume({ totalBytes: null, freeBytes: null, usedBytes: null }),
      ),
    ).toBeNull();
    expect(
      capacityPercent(makeVolume({ totalBytes: 0, usedBytes: 0 })),
    ).toBeNull();
    expect(
      capacityPercent(makeVolume({ totalBytes: 100, usedBytes: null })),
    ).toBeNull();
  });

  it("never reports more than full", () => {
    expect(
      capacityPercent(makeVolume({ totalBytes: 100, usedBytes: 250 })),
    ).toBe(100);
  });

  it("summarizes free space against capacity", () => {
    expect(
      capacitySummary(
        makeVolume({ totalBytes: 500 * GIB, freeBytes: 120 * GIB }),
      ),
    ).toBe("120 GB free of 500 GB");
  });

  it("says so when the platform did not report capacity", () => {
    expect(
      capacitySummary(makeVolume({ totalBytes: null, freeBytes: null })),
    ).toBe("Capacity unavailable");
    expect(capacitySummary(makeVolume({ totalBytes: 0, freeBytes: 0 }))).toBe(
      "Capacity unavailable",
    );
  });

  it("notes unreachable and read-only volumes", () => {
    expect(volumeNotes(makeVolume())).toEqual([]);
    expect(volumeNotes(makeVolume({ mounted: false }))).toEqual([
      "Not available",
    ]);
    expect(volumeNotes(makeVolume({ readonly: true }))).toEqual(["Read-only"]);
    expect(volumeNotes(makeVolume({ mounted: false, readonly: true }))).toEqual(
      ["Not available", "Read-only"],
    );
  });
});

describe("entry presentation", () => {
  it("labels every entry kind", () => {
    expect(entryKindLabel("directory")).toBe("Folder");
    expect(entryKindLabel("file")).toBe("File");
    expect(entryKindLabel("symlink")).toBe("Symlink");
    expect(entryKindLabel("other")).toBe("Other");
  });

  it("gives every entry kind an icon", () => {
    expect(Object.keys(ENTRY_KIND_ICONS).sort()).toEqual(
      Object.keys(ENTRY_KIND_LABELS).sort(),
    );
    for (const icon of Object.values(ENTRY_KIND_ICONS)) {
      expect(icon).toBeDefined();
    }
  });

  it("only treats folders and links as openable", () => {
    expect(isOpenableDirectory(makeEntry({ kind: "directory" }))).toBe(true);
    expect(isOpenableDirectory(makeEntry({ kind: "symlink" }))).toBe(true);
    expect(isOpenableDirectory(makeEntry({ kind: "file" }))).toBe(false);
    expect(isOpenableDirectory(makeEntry({ kind: "other" }))).toBe(false);
  });
});

describe("findVolumeForLocation", () => {
  const system = makeVolume();
  const media = makeVolume({ id: "D:", root: "D:\\", label: "MEDIA" });
  const root = makeVolume({ id: "/", root: "/", label: "/", kind: "unknown" });
  const usb = makeVolume({
    id: "/mnt/usb",
    root: "/mnt/usb/",
    label: "/mnt/usb",
    kind: "removable",
  });

  it("finds the volume that contains a location", () => {
    expect(findVolumeForLocation([system, media], "C:\\Users")?.id).toBe("C:");
    expect(findVolumeForLocation([system, media], "D:\\")?.id).toBe("D:");
  });

  it("matches roots case-insensitively", () => {
    expect(findVolumeForLocation([system], "c:\\users")?.id).toBe("C:");
  });

  it("returns null when nothing is open", () => {
    expect(findVolumeForLocation([system], null)).toBeNull();
  });

  it("returns null when no reported volume contains the location", () => {
    expect(findVolumeForLocation([system], "Z:\\Backups")).toBeNull();
  });

  it("prefers the most specific root", () => {
    expect(findVolumeForLocation([root, usb], "/mnt/usb/photos")?.id).toBe(
      "/mnt/usb",
    );
    expect(findVolumeForLocation([usb, root], "/mnt/usb/photos")?.id).toBe(
      "/mnt/usb",
    );
    expect(findVolumeForLocation([root, usb], "/etc")?.id).toBe("/");
  });

  it("does not match a sibling with the same prefix", () => {
    expect(findVolumeForLocation([system], "C:Backup")).toBeNull();
  });
});
