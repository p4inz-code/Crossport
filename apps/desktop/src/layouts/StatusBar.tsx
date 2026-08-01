/* ==========================================================================
 * StatusBar component
 * Bottom status strip showing application state and version.
 * ========================================================================== */

import { APP_NAME } from "@/lib";
import { useAppStore } from "@/stores";
import "./StatusBar.css";

const STATUS_META = {
  ready: { label: "Ready", className: "status-bar__dot--ready" },
  busy: { label: "Working", className: "status-bar__dot--busy" },
  error: { label: "Error", className: "status-bar__dot--error" },
} as const;

export function StatusBar() {
  const status = useAppStore((state) => state.status);
  const version = useAppStore((state) => state.version);
  const platform = useAppStore((state) => state.platform);

  const statusMeta = STATUS_META[status];

  return (
    <footer className="status-bar">
      <div className="status-bar__item">
        <span
          className={`status-bar__dot ${statusMeta.className}`}
          aria-hidden="true"
        />
        <span>{statusMeta.label}</span>
      </div>
      <div className="status-bar__item status-bar__item--meta">
        <span>{platform}</span>
        <span aria-hidden="true">·</span>
        <span>
          {APP_NAME} v{version}
        </span>
      </div>
    </footer>
  );
}
