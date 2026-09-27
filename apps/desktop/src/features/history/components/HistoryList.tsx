/* ==========================================================================
 * History list
 * One row per record: when it happened, what it did, where it went, how much
 * moved, how long it took, and the verification verdict.
 *
 * Rows are buttons, so the list is keyboard-navigable and selecting a row is
 * the only way the details panel opens. A row never shows a green mark for a
 * record whose verification failed.
 * ========================================================================== */

import type { LucideIcon } from "lucide-react";
import {
  AlertTriangle,
  CheckCircle2,
  CircleSlash,
  HelpCircle,
} from "lucide-react";

import { cn, formatBytes, formatDateTime, formatDuration } from "@/lib";
import {
  type HistoryRecord,
  historyStatusLabel,
  verificationPolicyLabel,
} from "@/types";
import {
  type HistoryTone,
  historyRowSummary,
  historyTone,
} from "../presentation";

const TONE_ICONS: Record<HistoryTone, LucideIcon> = {
  success: CheckCircle2,
  warning: AlertTriangle,
  danger: AlertTriangle,
  neutral: CircleSlash,
};

interface HistoryListProps {
  records: HistoryRecord[];
  selectedId: string | null;
  onSelect: (id: string) => void;
}

export function HistoryList({
  records,
  selectedId,
  onSelect,
}: HistoryListProps) {
  return (
    <ul className="history-list">
      {records.map((record) => {
        const tone = historyTone(record);
        const Icon = TONE_ICONS[tone];
        const verificationFailed =
          record.verification !== null &&
          (record.verification.status === "mismatch" ||
            record.verification.status === "failed");

        return (
          <li key={record.id}>
            <button
              type="button"
              className={cn(
                "history-row",
                `history-row--${tone}`,
                record.id === selectedId && "history-row--selected",
              )}
              aria-current={record.id === selectedId}
              onClick={() => onSelect(record.id)}
            >
              <span className="history-row__status">
                <Icon size={16} strokeWidth={1.75} aria-hidden="true" />
                {historyStatusLabel(record.status)}
              </span>

              <span className="history-row__what">
                <span className="history-row__source">
                  {record.sources[0]}
                  {record.sources.length > 1
                    ? ` +${record.sources.length - 1}`
                    : ""}
                </span>
                <span className="history-row__destination">
                  → {record.destination}
                </span>
              </span>

              <span className="history-row__numbers">
                <span>{historyRowSummary(record)}</span>
                <span className="history-row__meta">
                  {formatDuration(record.durationMs)} ·{" "}
                  {formatDateTime(record.finishedAtMs)}
                </span>
              </span>

              <span
                className={cn(
                  "history-row__verification",
                  verificationFailed && "history-row__verification--failed",
                )}
              >
                {record.verification === null ? (
                  <>
                    <HelpCircle
                      size={14}
                      strokeWidth={1.75}
                      aria-hidden="true"
                    />
                    No verification recorded
                  </>
                ) : (
                  <>
                    {record.verification.verdict}
                    <span className="history-row__policy">
                      {verificationPolicyLabel(record.verification.policy)}
                    </span>
                  </>
                )}
              </span>

              <span className="history-row__bytes">
                {formatBytes(record.totalBytes)}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}
