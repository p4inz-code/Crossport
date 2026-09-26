import { describe, expect, it } from "vitest";
import { makeEntry, makeListing, makeVolume } from "@/test/fixtures";
import { driveInfoSchema, driveListSchema } from "./drives";
import {
  directoryEntrySchema,
  directoryListingSchema,
  entryMetadataSchema,
  pickedDirectorySchema,
} from "./filesystem";
import { systemInfoSchema } from "./system";

describe("drive schemas", () => {
  it("accepts a fully reported backend volume", () => {
    expect(driveInfoSchema.safeParse(makeVolume()).success).toBe(true);
  });

  it("accepts unreported metadata instead of requiring it", () => {
    const sparse = makeVolume({
      id: "/mnt/usb",
      root: "/mnt/usb/",
      label: "/mnt/usb",
      name: null,
      kind: "unknown",
      filesystem: null,
      totalBytes: null,
      freeBytes: null,
      usedBytes: null,
      readonly: null,
      mounted: false,
    });

    expect(driveInfoSchema.safeParse(sparse).success).toBe(true);
  });

  it("rejects empty identifiers and an unknown volume kind", () => {
    expect(driveInfoSchema.safeParse(makeVolume({ id: "" })).success).toBe(
      false,
    );
    expect(driveInfoSchema.safeParse(makeVolume({ root: "" })).success).toBe(
      false,
    );
    expect(driveInfoSchema.safeParse(makeVolume({ label: "" })).success).toBe(
      false,
    );
    expect(
      driveInfoSchema.safeParse({ ...makeVolume(), kind: "floppy" }).success,
    ).toBe(false);
  });

  it("rejects negative capacity and a missing mounted flag", () => {
    expect(
      driveInfoSchema.safeParse(makeVolume({ totalBytes: -1 })).success,
    ).toBe(false);

    const { mounted: _mounted, ...withoutFlag } = makeVolume();
    expect(driveInfoSchema.safeParse(withoutFlag).success).toBe(false);
  });

  it("accepts an empty drive list but not a malformed one", () => {
    expect(driveListSchema.safeParse([]).success).toBe(true);
    expect(driveListSchema.safeParse(null).success).toBe(false);
    expect(driveListSchema.safeParse(["C:\\"]).success).toBe(false);
  });
});

describe("directory schemas", () => {
  it("accepts a backend listing", () => {
    expect(directoryListingSchema.safeParse(makeListing()).success).toBe(true);
  });

  it("accepts a root listing with no parent and empty folders", () => {
    const listing = makeListing({
      path: "/",
      name: "/",
      parent: null,
      entries: [],
    });

    expect(directoryListingSchema.safeParse(listing).success).toBe(true);
  });

  it("accepts entries the platform could not describe", () => {
    const entry = makeEntry({
      kind: "symlink",
      sizeBytes: null,
      modifiedMs: null,
      readonly: null,
    });

    expect(directoryEntrySchema.safeParse(entry).success).toBe(true);
  });

  it("rejects unknown entry kinds and impossible sizes", () => {
    expect(
      directoryEntrySchema.safeParse({ ...makeEntry(), kind: "socket" })
        .success,
    ).toBe(false);
    expect(
      directoryEntrySchema.safeParse({ ...makeEntry(), sizeBytes: -1 }).success,
    ).toBe(false);
    expect(
      directoryEntrySchema.safeParse({ ...makeEntry(), sizeBytes: 1.5 })
        .success,
    ).toBe(false);
  });

  it("rejects a listing without its entries or path", () => {
    const { entries: _entries, ...withoutEntries } = makeListing();
    expect(directoryListingSchema.safeParse(withoutEntries).success).toBe(
      false,
    );
    expect(
      directoryListingSchema.safeParse({ ...makeListing(), path: "" }).success,
    ).toBe(false);
  });
});

describe("entry metadata schema", () => {
  const metadata = {
    path: "/home/user",
    name: "user",
    isDir: true,
    isFile: false,
    isSymlink: false,
    sizeBytes: 4096,
    modifiedMs: 1_700_000_000_000,
    readonly: false,
  };

  it("accepts a backend metadata payload", () => {
    expect(entryMetadataSchema.safeParse(metadata).success).toBe(true);
  });

  it("accepts a null modification time", () => {
    expect(
      entryMetadataSchema.safeParse({ ...metadata, modifiedMs: null }).success,
    ).toBe(true);
  });

  it("rejects impossible sizes and missing flags", () => {
    expect(
      entryMetadataSchema.safeParse({ ...metadata, sizeBytes: -1 }).success,
    ).toBe(false);
    expect(
      entryMetadataSchema.safeParse({ ...metadata, sizeBytes: 1.5 }).success,
    ).toBe(false);

    const { isDir: _isDir, ...withoutFlag } = metadata;
    expect(entryMetadataSchema.safeParse(withoutFlag).success).toBe(false);
  });

  it("treats a picked directory as a path or a cancellation", () => {
    expect(pickedDirectorySchema.safeParse("C:\\Users").success).toBe(true);
    expect(pickedDirectorySchema.safeParse(null).success).toBe(true);
    expect(pickedDirectorySchema.safeParse("").success).toBe(false);
  });
});

describe("system info schema", () => {
  it("accepts the backend contract", () => {
    expect(
      systemInfoSchema.safeParse({
        platform: "windows",
        os: "windows",
        arch: "x86_64",
        family: "windows",
      }).success,
    ).toBe(true);
  });

  it("rejects unknown platforms and missing fields", () => {
    expect(
      systemInfoSchema.safeParse({
        platform: "solaris",
        os: "solaris",
        arch: "sparc",
        family: "unix",
      }).success,
    ).toBe(false);

    expect(systemInfoSchema.safeParse({ platform: "linux" }).success).toBe(
      false,
    );
  });
});
