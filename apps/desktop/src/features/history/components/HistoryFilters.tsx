/* ==========================================================================
 * History filters
 * The five filters the backend accepts, in one row. Filtering is done in Rust
 * over the durable list, so what is shown is what is stored rather than a
 * client-side view of a partial page.
 * ========================================================================== */

import { Button } from "@/components/ui";
import {
  HISTORY_FILTERS,
  type HistoryFilter,
  historyFilterLabel,
} from "@/types";

interface HistoryFiltersProps {
  active: HistoryFilter;
  disabled: boolean;
  /** How many records are selected by each filter, when known. */
  counts: Partial<Record<HistoryFilter, number>>;
  onChange: (filter: HistoryFilter) => void;
}

export function HistoryFilters({
  active,
  disabled,
  counts,
  onChange,
}: HistoryFiltersProps) {
  return (
    // A fieldset is the element that already means "group of controls", so the
    // row needs no role of its own and keeps its name for assistive tech.
    <fieldset className="history-filters" aria-label="Filter history">
      {HISTORY_FILTERS.map((filter) => {
        const count = counts[filter];
        return (
          <Button
            key={filter}
            variant={filter === active ? "primary" : "secondary"}
            aria-pressed={filter === active}
            disabled={disabled}
            onClick={() => onChange(filter)}
          >
            {historyFilterLabel(filter)}
            {count === undefined ? "" : ` (${count})`}
          </Button>
        );
      })}
    </fieldset>
  );
}
