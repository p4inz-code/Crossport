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
import { THEME_MODES, type ThemeMode } from "@/types";
import "./SettingsPage.css";

const THEME_LABELS: Record<ThemeMode, string> = {
  light: "Light",
  dark: "Dark",
  system: "System",
};

export function SettingsPage() {
  const theme = useSettingsStore((state) => state.theme);
  const locale = useSettingsStore((state) => state.locale);
  const status = useSettingsStore((state) => state.status);
  const error = useSettingsStore((state) => state.error);
  const saveStatus = useSettingsStore((state) => state.saveStatus);
  const setTheme = useSettingsStore((state) => state.setTheme);
  const setLocale = useSettingsStore((state) => state.setLocale);

  // The input is uncontrolled: its value is read on submit and re-keyed from
  // the persisted value, so hydration and rejected saves both stay in sync
  // without mirroring store state into React state.
  function handleLocaleSubmit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    const submitted = new FormData(event.currentTarget).get("locale");
    setLocale(typeof submitted === "string" ? submitted : "");
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
