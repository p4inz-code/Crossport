/* ==========================================================================
 * History feature — placeholder page
 * ========================================================================== */

import { History } from "lucide-react";

import { EmptyState } from "@/components/ui";
import { PageContainer } from "@/layouts";

export function HistoryPage() {
  return (
    <PageContainer title="History" description="Review past transfer activity.">
      <EmptyState
        icon={History}
        title="History coming soon"
        description="Completed transfers will appear here with timestamps and status details."
      />
    </PageContainer>
  );
}
