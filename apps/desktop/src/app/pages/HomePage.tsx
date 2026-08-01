/* ==========================================================================
 * Home page
 * Landing view rendered at the root route. Demonstrates the layout and
 * primitives while future features land.
 * ========================================================================== */

import { Activity, Layers, ShieldCheck } from "lucide-react";

import { Card, CardBody, CardHeader, Section } from "@/components/ui";
import { PageContainer } from "@/layouts";
import { useAppStore, useSettingsStore } from "@/stores";
import "./HomePage.css";

export function HomePage() {
  const appName = useAppStore((state) => state.name);
  const version = useAppStore((state) => state.version);
  const platform = useAppStore((state) => state.platform);
  const theme = useSettingsStore((state) => state.theme);

  return (
    <PageContainer
      title={`Welcome to ${appName}`}
      description="A fast, reliable, cross-platform file transfer utility."
    >
      <Section title="Application status">
        <div className="home-grid">
          <Card>
            <CardHeader title="Platform" description="Detected at runtime" />
            <CardBody>
              <p className="home-stat__value">{platform}</p>
            </CardBody>
          </Card>
          <Card>
            <CardHeader title="Version" description="Installed build" />
            <CardBody>
              <p className="home-stat__value">v{version}</p>
            </CardBody>
          </Card>
          <Card>
            <CardHeader title="Theme" description="Active preference" />
            <CardBody>
              <p className="home-stat__value">{theme}</p>
            </CardBody>
          </Card>
        </div>
      </Section>

      <Section title="Foundation status">
        <div className="home-grid">
          <Card>
            <CardHeader
              title="Architecture"
              description="Feature-based frontend with shared stores and services"
              action={<ShieldCheck size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Routing"
              description="React Router foundation with lazy-loadable pages"
              action={<Layers size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Backend"
              description="Modular Rust command layer ready for features"
              action={<Activity size={20} strokeWidth={1.75} />}
            />
          </Card>
        </div>
      </Section>
    </PageContainer>
  );
}
