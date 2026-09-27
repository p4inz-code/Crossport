/* ==========================================================================
 * Recovery banner
 * The startup prompt: when the application opens with interrupted transfers,
 * this states how many there are and links to the details. It performs no
 * action and removes nothing — the decision belongs to the user, on the
 * recovery page.
 * ========================================================================== */

import { ShieldAlert } from "lucide-react";
import { Link } from "react-router-dom";

import { useRecoveryStore } from "@/stores";
import "./RecoveryBanner.css";

export function RecoveryBanner() {
  const candidates = useRecoveryStore((state) => state.candidates);

  if (candidates.length === 0) {
    return null;
  }

  const restartable = candidates.filter(
    (candidate) => candidate.canRestart,
  ).length;

  return (
    <div className="recovery-banner" role="status">
      <ShieldAlert size={18} strokeWidth={1.75} aria-hidden="true" />
      <p className="recovery-banner__text">
        {candidates.length === 1
          ? "A transfer was interrupted when the application last stopped."
          : `${candidates.length} transfers were interrupted when the application last stopped.`}{" "}
        {restartable > 0
          ? `${restartable} can be run again from the beginning.`
          : "None of them can be run again, but they can be inspected or discarded."}
      </p>
      <Link className="recovery-banner__link" to="/recovery">
        Review
      </Link>
    </div>
  );
}
