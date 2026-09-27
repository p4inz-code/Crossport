/* ==========================================================================
 * History page
 * The durable record of transfers: recent ones first, filterable by how they
 * ended, with a details panel for the one being inspected.
 *
 * The page reports what it cannot show instead of hiding it: a history
 * document that could not be used, or one written by a newer build, is stated
 * above the list rather than rendered as an empty history.
 * ========================================================================== */

import { Eraser, History, RefreshCw } from "lucide-react";
import { useEffect } from "react";
import { useNavigate } from "react-router-dom";

import {
  Button,
  EmptyState,
  LoadingState,
  Notice,
  Section,
} from "@/components/ui";
import { PageContainer } from "@/layouts";
import { useHistoryStore } from "@/stores";
import { archiveLoadStateLabel, isDocumentNoteworthy } from "@/types";
import { HistoryDetails } from "../components/HistoryDetails";
import { HistoryFilters } from "../components/HistoryFilters";
import { HistoryList } from "../components/HistoryList";
import "./HistoryPage.css";

export function HistoryPage() {
  const navigate = useNavigate();
  const records = useHistoryStore((state) => state.records);
  const filter = useHistoryStore((state) => state.filter);
  const total = useHistoryStore((state) => state.total);
  const limit = useHistoryStore((state) => state.limit);
  const documentStatus = useHistoryStore((state) => state.documentStatus);
  const writable = useHistoryStore((state) => state.writable);
  const status = useHistoryStore((state) => state.status);
  const error = useHistoryStore((state) => state.error);
  const selectedId = useHistoryStore((state) => state.selectedId);
  const pending = useHistoryStore((state) => state.pending);
  const load = useHistoryStore((state) => state.load);
  const select = useHistoryStore((state) => state.select);
  const remove = useHistoryStore((state) => state.remove);
  const clear = useHistoryStore((state) => state.clear);

  useEffect(() => {
    void load();
  }, [load]);

  const selected = records.find((record) => record.id === selectedId) ?? null;
  const loading = status === "loading";

  return (
    <PageContainer
      title="History"
      description="Finished transfers, kept by the backend. Filters run over the durable list."
      className="history-page"
      actions={
        <>
          <Button
            variant="secondary"
            onClick={() => void load()}
            disabled={loading}
          >
            <RefreshCw size={16} strokeWidth={1.75} aria-hidden="true" />
            Refresh
          </Button>
          <Button
            variant="secondary"
            onClick={() => void clear()}
            disabled={loading || total === 0 || !writable}
          >
            <Eraser size={16} strokeWidth={1.75} aria-hidden="true" />
            Clear history
          </Button>
        </>
      }
    >
      {documentStatus !== null && isDocumentNoteworthy(documentStatus) ? (
        <Notice
          tone="warning"
          title={archiveLoadStateLabel(documentStatus.state)}
          detail={`${
            documentStatus.detail ??
            "The history document could not be used as written."
          }${
            writable
              ? " Its contents were set aside so new records can be kept."
              : " Its contents are untouched; this build will not overwrite a newer document."
          }`}
        />
      ) : null}

      {filter === "interrupted" ? (
        <Notice
          tone="info"
          title="Interrupted and recovered transfers"
          detail="These records come from recovery rather than from a job finishing on its own. Anything still waiting for a decision is on the Recovery page."
          action={
            <Button
              size="sm"
              variant="secondary"
              onClick={() => navigate("/recovery")}
            >
              Open Recovery
            </Button>
          }
        />
      ) : null}

      <HistoryFilters
        active={filter}
        disabled={loading}
        counts={{ all: total }}
        onChange={(next) => void load(next)}
      />

      <p className="history-page__retention">
        {total === 0
          ? `Keeping up to ${limit} records.`
          : `${total} record${total === 1 ? "" : "s"} kept, newest first, up to ${limit}.`}
      </p>

      {loading && records.length === 0 ? (
        <LoadingState label="Loading transfer history…" />
      ) : null}

      {error !== null ? (
        <Notice
          tone="danger"
          title="History could not be loaded"
          detail={`${error.message}${
            error.code === "unavailable"
              ? " History is stored by the desktop application."
              : ""
          }`}
          action={
            <Button variant="secondary" onClick={() => void load()}>
              Try again
            </Button>
          }
        />
      ) : null}

      {error === null && !loading && records.length === 0 ? (
        <EmptyState
          icon={History}
          title="No transfers match this filter"
          description={
            total === 0
              ? "Finished transfers are recorded here and survive a restart."
              : "Try another filter to see the rest of the history."
          }
        />
      ) : null}

      {records.length > 0 ? (
        <Section title="Recent transfers">
          <HistoryList
            records={records}
            selectedId={selectedId}
            onSelect={(id) => select(id === selectedId ? null : id)}
          />
        </Section>
      ) : null}

      {selected !== null ? (
        <Section title="Transfer details">
          <HistoryDetails
            record={selected}
            deleting={pending.includes(selected.id)}
            onClose={() => select(null)}
            onDelete={(id) => void remove(id)}
          />
        </Section>
      ) : null}
    </PageContainer>
  );
}
