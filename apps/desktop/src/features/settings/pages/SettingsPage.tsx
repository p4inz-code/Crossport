/* ==========================================================================
 * Settings page
 * Edits the settings the Rust backend owns. Changes are validated locally,
 * sent over IPC, and persisted by the backend; the page reports the write
 * result instead of assuming success.
 * ========================================================================== */

import { AlertTriangle, Check } from "lucide-react";
import type { FormEvent } from "react";

import {
  Button,
  Card,
  CardBody,
  CardHeader,
  LoadingState,
  Section,
} from "@/components/ui";
import { PageContainer } from "@/layouts";
import { useSettingsStore } from "@/stores";
import {
  MAX_HISTORY_LIMIT,
  MIN_HISTORY_LIMIT,
  THEME_MODES,
  type ThemeMode,
  VERIFICATION_POLICIES,
  type VerificationPolicy,
  verificationPolicyLabel,
} from "@/types";
import "./SettingsPage.css";

const THEME_LABELS: Record<ThemeMode, string> = {
  light: "Light",
  dark: "Dark",
  system: "System",
};

/** What each verification policy actually does, in the backend's terms. */
const VERIFICATION_HINTS: Record<VerificationPolicy, string> = {
  none: "Nothing is checked. This is only appropriate when verification is impossible.",
  size: "Every copied file must exist and match the size that was measured. The default.",
  checksum:
    "Size checks plus a streaming SHA-256 comparison of the source bytes against the committed file. Strongest, and it costs a hash of every file.",
};

export function SettingsPage() {
  const theme = useSettingsStore((state) => state.theme);
  const locale = useSettingsStore((state) => state.locale);
  const status = useSettingsStore((state) => state.status);
  const error = useSettingsStore((state) => state.error);
  const saveStatus = useSettingsStore((state) => state.saveStatus);
  const verification = useSettingsStore((state) => state.verification);
  const historyLimit = useSettingsStore((state) => state.historyLimit);
  const setTheme = useSettingsStore((state) => state.setTheme);
  const setLocale = useSettingsStore((state) => state.setLocale);
  const setVerification = useSettingsStore((state) => state.setVerification);
  const setHistoryLimit = useSettingsStore((state) => state.setHistoryLimit);

  // The input is uncontrolled: its value is read on submit and re-keyed from
  // the persisted value, so hydration and rejected saves both stay in sync
  // without mirroring store state into React state.
  function handleLocaleSubmit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    const submitted = new FormData(event.currentTarget).get("locale");
    setLocale(typeof submitted === "string" ? submitted : "");
  }

  // The limit is validated by the backend as well; an out-of-range value never
  // leaves this form, so the user is told immediately instead of after a save.
  function handleHistoryLimitSubmit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    const submitted = new FormData(event.currentTarget).get("historyLimit");
    const parsed = Number.parseInt(
      typeof submitted === "string" ? submitted : "",
      10,
    );
    if (!Number.isFinite(parsed)) {
      setHistoryLimit(Number.NaN);
      return;
    }
    setHistoryLimit(parsed);
  }

  return (
    <PageContainer
      title="Settings"
      description="Preferences are owned and persisted by the CrossPort backend."
    >
      {status === "loading" ? <LoadingState label="Loading settings…" /> : null}

      <Section title="Appearance">
        <Card>
          <CardHeader
            title="Theme"
            description="Applied immediately and stored by the backend."
          />
          <CardBody>
            <div className="settings-choices">
              {THEME_MODES.map((mode) => (
                <Button
                  key={mode}
                  variant={mode === theme ? "primary" : "secondary"}
                  aria-pressed={mode === theme}
                  onClick={() => setTheme(mode)}
                >
                  {THEME_LABELS[mode]}
                </Button>
              ))}
            </div>
          </CardBody>
        </Card>
      </Section>

      <Section title="Region">
        <Card>
          <CardHeader
            title="Locale"
            description="A BCP-47 tag such as en, de, or pt-BR (2–16 characters)."
          />
          <CardBody>
            <form className="settings-field" onSubmit={handleLocaleSubmit}>
              <label className="settings-field__label" htmlFor="locale">
                Locale
              </label>
              <input
                key={locale}
                id="locale"
                className="settings-field__input"
                name="locale"
                defaultValue={locale}
                maxLength={16}
                autoComplete="off"
                spellCheck={false}
              />
              <Button type="submit" variant="secondary">
                Save locale
              </Button>
            </form>
          </CardBody>
        </Card>
      </Section>

      <Section title="Verification">
        <Card>
          <CardHeader
            title="How transfers verify what they write"
            description="Applied to transfers started from now on. A job already queued keeps the policy it was accepted with."
          />
          <CardBody>
            <div className="settings-choices">
              {VERIFICATION_POLICIES.map((policy) => (
                <Button
                  key={policy}
                  variant={policy === verification ? "primary" : "secondary"}
                  aria-pressed={policy === verification}
                  onClick={() => setVerification(policy)}
                >
                  {verificationPolicyLabel(policy)}
                </Button>
              ))}
            </div>
            <p className="settings-note">{VERIFICATION_HINTS[verification]}</p>
            <p className="settings-note settings-note--muted">
              Modified times and read-only attributes are not reapplied by this
              engine, so verification reports them as not preserved rather than
              claiming they are.
            </p>
          </CardBody>
        </Card>
      </Section>

      <Section title="History">
        <Card>
          <CardHeader
            title="Retention"
            description={`How many finished transfers are kept. Between ${MIN_HISTORY_LIMIT} and ${MAX_HISTORY_LIMIT}; shrinking it prunes the oldest records immediately.`}
          />
          <CardBody>
            <form
              className="settings-field"
              onSubmit={handleHistoryLimitSubmit}
            >
              <label className="settings-field__label" htmlFor="historyLimit">
                Records kept
              </label>
              <input
                key={historyLimit}
                id="historyLimit"
                className="settings-field__input"
                name="historyLimit"
                type="number"
                min={MIN_HISTORY_LIMIT}
                max={MAX_HISTORY_LIMIT}
                step={10}
                defaultValue={historyLimit}
              />
              <Button type="submit" variant="secondary">
                Save limit
              </Button>
            </form>
          </CardBody>
        </Card>
      </Section>

      <div className="settings-status" aria-live="polite">
        {saveStatus === "saving" ? (
          <p className="settings-status__note">Saving…</p>
        ) : null}
        {saveStatus === "saved" ? (
          <p className="settings-status__note settings-status__note--ok">
            <Check size={16} strokeWidth={2} aria-hidden="true" />
            Settings saved by the backend.
          </p>
        ) : null}
        {error !== null ? (
          <p className="settings-status__error" role="alert">
            <AlertTriangle size={16} strokeWidth={2} aria-hidden="true" />
            {error.message}
          </p>
        ) : null}
      </div>
    </PageContainer>
  );
}
