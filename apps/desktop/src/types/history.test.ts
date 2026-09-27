import { describe, expect, it } from "vitest";

import { makeHistoryRecord } from "@/test/fixtures";
import {
  HISTORY_FILTERS,
  HISTORY_STATUSES,
  historyFilterLabel,
  historyRecordSchema,
  historyStatusLabel,
  historyStatusSucceeded,
} from "./history";

describe("history contracts", () => {
  it("accepts a record the backend produced", () => {
    expect(historyRecordSchema.safeParse(makeHistoryRecord()).success).toBe(
      true,
    );
  });

  it("accepts a record from an older document without a verification summary", () => {
    const parsed = historyRecordSchema.safeParse(
      makeHistoryRecord({ verification: null, recovery: null }),
    );

    expect(parsed.success).toBe(true);
    expect(parsed.success && parsed.data.verification).toBeNull();
  });

  it("rejects a record whose verification status is not one the backend publishes", () => {
    const broken = {
      ...makeHistoryRecord(),
      verification: { ...makeHistoryRecord().verification, status: "probably" },
    };

    expect(historyRecordSchema.safeParse(broken).success).toBe(false);
  });

  it("rejects a record with no source at all", () => {
    expect(
      historyRecordSchema.safeParse(makeHistoryRecord({ sources: [] })).success,
    ).toBe(false);
  });

  it("rejects a negative byte count rather than showing it", () => {
    expect(
      historyRecordSchema.safeParse(makeHistoryRecord({ transferredBytes: -1 }))
        .success,
    ).toBe(false);
  });

  it("only counts completed and recovered records as successes", () => {
    expect(historyStatusSucceeded("completed")).toBe(true);
    expect(historyStatusSucceeded("recovered")).toBe(true);
    expect(historyStatusSucceeded("interrupted")).toBe(false);
    expect(historyStatusSucceeded("failed")).toBe(false);
    expect(historyStatusSucceeded("cancelled")).toBe(false);
  });

  it("labels every status and filter without a fallthrough", () => {
    for (const status of HISTORY_STATUSES) {
      expect(historyStatusLabel(status).length).toBeGreaterThan(0);
    }
    for (const filter of HISTORY_FILTERS) {
      expect(historyFilterLabel(filter).length).toBeGreaterThan(0);
    }
  });
});
