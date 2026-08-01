/* ==========================================================================
 * Transfer feature — placeholder page
 * ========================================================================== */

import { ArrowLeftRight } from "lucide-react";

import { EmptyState } from "@/components/ui";
import { PageContainer } from "@/layouts";

export function TransferPage() {
  return (
    <PageContainer
      title="Transfers"
      description="Track copy and move operations."
    >
      <EmptyState
        icon={ArrowLeftRight}
        title="Transfers coming soon"
        description="Active transfers will appear here with progress, speed, and remaining time."
      />
    </PageContainer>
  );
}
