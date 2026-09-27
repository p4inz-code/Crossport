/* ==========================================================================
 * Not found page
 * Reached only when a route does not exist — a stale hash link, or a path
 * typed by hand. Instead of an empty frame it names what happened and offers
 * the route the user most likely wanted.
 * ========================================================================== */

import { Compass } from "lucide-react";
import { Link } from "react-router-dom";

import { Button, EmptyState } from "@/components/ui";
import { PageContainer } from "@/layouts";

export function NotFoundPage() {
  return (
    <PageContainer title="Page not found">
      <EmptyState
        icon={Compass}
        title="This page does not exist"
        description="The address you followed is not part of CrossPort. Nothing was changed on disk."
        action={
          <Link to="/drives">
            <Button>Go to Drives</Button>
          </Link>
        }
      />
    </PageContainer>
  );
}
