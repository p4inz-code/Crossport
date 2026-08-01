/* ==========================================================================
 * Settings feature — placeholder page
 * ========================================================================== */

import { Settings } from "lucide-react";

import { EmptyState } from "@/components/ui";
import { PageContainer } from "@/layouts";

export function SettingsPage() {
  return (
    <PageContainer
      title="Settings"
      description="Configure application preferences."
    >
      <EmptyState
        icon={Settings}
        title="Settings coming soon"
        description="Theme, locale, and transfer preferences will be configurable here."
      />
    </PageContainer>
  );
}
