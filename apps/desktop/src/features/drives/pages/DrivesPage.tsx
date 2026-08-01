/* ==========================================================================
 * Drives feature — placeholder page
 * ========================================================================== */

import { HardDrive } from "lucide-react";

import { EmptyState } from "@/components/ui";
import { PageContainer } from "@/layouts";

export function DrivesPage() {
  return (
    <PageContainer
      title="Drives"
      description="Manage and monitor connected drives."
    >
      <EmptyState
        icon={HardDrive}
        title="Drive detection coming soon"
        description="Connected drives will appear here with capacity, filesystem, and status information."
      />
    </PageContainer>
  );
}
