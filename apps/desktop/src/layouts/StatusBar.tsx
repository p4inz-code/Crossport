/* ==========================================================================
 * StatusBar component
 * Bottom status strip showing the host platform reported by the backend and
 * the installed version. Outside the desktop app the platform is unknown by
 * definition, so the strip says so instead of guessing.
 * ========================================================================== */

import { APP_NAME } from "@/lib";
import { useAppStore, useSystemStore } from "@/stores";
import "./StatusBar.css";

export function StatusBar() {
  const version = useAppStore((state) => state.version);
  const info = useSystemStore((state) => state.info);
  const status = useSystemStore((state) => state.status);

  const platformLabel =
    info === null ? (status === "error" ? "browser preview" : "…") : info.os;

  return (
    <footer className="status-bar">
      <div className="status-bar__item">
        <span className="status-bar__platform">{platformLabel}</span>
        {info !== null ? <span>{info.arch}</span> : null}
      </div>
      <div className="status-bar__item status-bar__item--meta">
        <span>{APP_NAME}</span>
        <span aria-hidden="true">·</span>
        <span>v{version}</span>
      </div>
    </footer>
  );
}
