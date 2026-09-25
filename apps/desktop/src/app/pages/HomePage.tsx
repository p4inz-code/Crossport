/* ==========================================================================
 * Home page
 * Landing view. Reports what the backend actually told us at startup and
 * summarizes the Phase 1 foundation without claiming unimplemented features.
 * ========================================================================== */

import {
  Activity,
  Cpu,
  FolderTree,
  HardDrive,
  Layers,
  ShieldCheck,
} from "lucide-react";

import { Card, CardBody, CardHeader, Section } from "@/components/ui";
import { PageContainer } from "@/layouts";
import { useAppStore, useSettingsStore, useSystemStore } from "@/stores";
import "./HomePage.css";

export function HomePage() {
  const appName = useAppStore((state) => state.name);
  const version = useAppStore((state) => state.version);
  const theme = useSettingsStore((state) => state.theme);
  const info = useSystemStore((state) => state.info);
  const systemStatus = useSystemStore((state) => state.status);

  const platformValue =
    info !== null
      ? `${info.os} · ${info.arch}`
      : systemStatus === "error"
        ? "Unavailable in the browser"
        : "Detecting…";

  return (
    <PageContainer
      title={`Welcome to ${appName}`}
      description="A fast, reliable, cross-platform file transfer utility."
    >
      <Section title="Application status">
        <div className="home-grid">
          <Card>
            <CardHeader
              title="Platform"
              description="Reported by the backend"
            />
            <CardBody>
              <p className="home-stat__value">{platformValue}</p>
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

      <Section
        title="Phase 1 foundation"
        description="What exists today, verified by tests and CI."
      >
        <div className="home-grid">
          <Card>
            <CardHeader
              title="Backend-owned settings"
              description="Validated, persisted in Rust, reached over typed IPC"
              action={<ShieldCheck size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Structured errors"
              description="Every failure crosses the boundary as code + message"
              action={<Activity size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Drive enumeration"
              description="Storage roots discovered on the host and validated"
              action={<HardDrive size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Filesystem foundation"
              description="Path normalization, directory validation, metadata"
              action={<FolderTree size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Platform abstraction"
              description="OS facts and app directories resolved in one module"
              action={<Cpu size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Native folder picker"
              description="Dialog hosted by Rust; the webview holds no fs capability"
              action={<Layers size={20} strokeWidth={1.75} />}
            />
          </Card>
        </div>
      </Section>
    </PageContainer>
  );
}
