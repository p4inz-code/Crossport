/* ==========================================================================
 * Home page
 * The launch view: the state of the application as the backend reported it,
 * the shortcuts into the two journeys that matter (start a transfer, finish an
 * interrupted one), and a plain description of what CrossPort does.
 *
 * Every number here comes from a store that is fed by the backend — nothing is
 * estimated, and a count of zero is shown as zero rather than hidden.
 * ========================================================================== */

import {
  ArrowRightLeft,
  FolderTree,
  HardDrive,
  History,
  Keyboard,
  MonitorSmartphone,
  ShieldAlert,
  ShieldCheck,
} from "lucide-react";
import { Link } from "react-router-dom";

import { Button, Card, CardBody, CardHeader, Section } from "@/components/ui";
import { isFinished } from "@/features/transfers/presentation";
import { PageContainer } from "@/layouts";
import {
  useAppStore,
  useRecoveryStore,
  useSettingsStore,
  useSystemStore,
  useTransferStore,
} from "@/stores";
import { verificationPolicyLabel } from "@/types";
import "./HomePage.css";

/** The journeys the home page sends the user into. */
const SHORTCUTS = [
  {
    keys: "Ctrl+1…6",
    action:
      "Move between Home, Drives, Transfers, History, Recovery, and Settings",
  },
  {
    keys: "Alt+← / Alt+→",
    action: "Walk back and forward through the folders you have visited",
  },
  { keys: "Alt+↑", action: "Open the parent folder in the browser" },
  { keys: "↑ / ↓", action: "Move through the entries of the open folder" },
  { keys: "Space", action: "Check or uncheck the focused entry" },
  { keys: "Enter", action: "Open the focused folder" },
  { keys: "Esc", action: "Close the open dialog without starting anything" },
];

/** `windows` → `Windows`, leaving the rest of the string alone. */
function capitalizeFirst(value: string): string {
  return `${value.charAt(0).toUpperCase()}${value.slice(1)}`;
}

export function HomePage() {
  const appName = useAppStore((state) => state.name);
  const version = useAppStore((state) => state.version);
  const info = useSystemStore((state) => state.info);
  const systemStatus = useSystemStore((state) => state.status);
  const verification = useSettingsStore((state) => state.verification);
  const historyLimit = useSettingsStore((state) => state.historyLimit);
  const jobs = useTransferStore((state) => state.jobs);
  const interrupted = useRecoveryStore((state) => state.candidates.length);

  const active = jobs.filter((job) => !isFinished(job)).length;
  // The backend reports the platform keyed the way the toolchain spells it
  // (`windows`); the interface shows it the way a person writes it.
  const platformValue =
    info !== null
      ? `${capitalizeFirst(info.os)} · ${info.arch}`
      : systemStatus === "error"
        ? "Unavailable in the browser"
        : "Detecting…";

  return (
    <PageContainer
      title={`Welcome to ${appName}`}
      description="A fast, reliable, cross-platform file transfer utility."
      width="wide"
    >
      <Section
        title="Start here"
        description="The two things CrossPort is for: moving files, and settling work that was interrupted."
      >
        <div className="home-grid">
          <Card>
            <CardHeader
              title="Transfer files"
              description="Browse any volume, select what to move, review the job, then start it."
              action={<FolderTree size={20} strokeWidth={1.75} />}
            />
            <CardBody>
              <Link to="/drives">
                <Button>Choose files</Button>
              </Link>
            </CardBody>
          </Card>
          <Card>
            <CardHeader
              title="Transfers"
              description={
                active === 0
                  ? "Nothing is moving right now."
                  : `${active} job${active === 1 ? "" : "s"} not finished.`
              }
              action={<ArrowRightLeft size={20} strokeWidth={1.75} />}
            />
            <CardBody>
              <Link to="/transfers">
                <Button variant="secondary">Open the queue</Button>
              </Link>
            </CardBody>
          </Card>
          <Card>
            <CardHeader
              title="Interrupted work"
              description={
                interrupted === 0
                  ? "No transfer was left unfinished by a previous session."
                  : `${interrupted} transfer${interrupted === 1 ? "" : "s"} need${interrupted === 1 ? "s" : ""} a decision.`
              }
              action={<ShieldAlert size={20} strokeWidth={1.75} />}
            />
            <CardBody>
              <Link to="/recovery">
                <Button variant={interrupted === 0 ? "secondary" : "primary"}>
                  Review recovery
                </Button>
              </Link>
            </CardBody>
          </Card>
        </div>
      </Section>

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
            <CardHeader
              title="Verification"
              description="Applied to transfers started from now on"
            />
            <CardBody>
              <p className="home-stat__value">
                {verificationPolicyLabel(verification)}
              </p>
            </CardBody>
          </Card>
          <Card>
            <CardHeader
              title="History retention"
              description="Finished transfers kept on disk"
            />
            <CardBody>
              <p className="home-stat__value">{historyLimit}</p>
            </CardBody>
          </Card>
        </div>
      </Section>

      <Section
        title="What CrossPort does"
        description="Everything below runs in the Rust backend; the interface only reports it."
      >
        <div className="home-grid">
          <Card>
            <CardHeader
              title="Copy and move"
              description="One engine, one queue, pause and resume, conflict handling decided per job."
              action={<ArrowRightLeft size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Verification"
              description="Each written file is checked against what was measured before the copy is accepted."
              action={<ShieldCheck size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="History"
              description="Finished and accounted-for jobs are recorded durably and pruned to the limit you set."
              action={<History size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Recovery"
              description="A transfer cut short by a crash is classified, never silently treated as complete."
              action={<ShieldAlert size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Platform abstraction"
              description="Volumes, paths, and permissions resolved per operating system in one module."
              action={<MonitorSmartphone size={20} strokeWidth={1.75} />}
            />
          </Card>
          <Card>
            <CardHeader
              title="Native folder picker"
              description="The dialog is hosted by Rust; the webview itself holds no filesystem capability."
              action={<HardDrive size={20} strokeWidth={1.75} />}
            />
          </Card>
        </div>
      </Section>

      <Section
        title="Keyboard"
        description="CrossPort is usable without a mouse."
      >
        <Card>
          <CardHeader
            title="Shortcuts"
            description="Navigation, browsing, and dismissal"
            action={<Keyboard size={20} strokeWidth={1.75} />}
          />
          <CardBody>
            <dl className="home-keys">
              {SHORTCUTS.map((shortcut) => (
                <div className="home-keys__row" key={shortcut.keys}>
                  <dt>
                    <kbd className="home-keys__key">{shortcut.keys}</kbd>
                  </dt>
                  <dd>{shortcut.action}</dd>
                </div>
              ))}
            </dl>
          </CardBody>
        </Card>
      </Section>
    </PageContainer>
  );
}
