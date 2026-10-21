import { useEffect, useMemo, useState } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";

import { useT } from "../i18n";
import { useSettings } from "../stores/settings";
import type { Settings } from "../stores/settings";
import { Group, RadioList, Row, ShortcutField, Toggle, formatShortcut } from "./controls";

type PaneId = "general" | "history" | "paste" | "rules" | "shortcuts" | "about";

interface RunningApp {
  bundleId: string;
  name: string;
}

function GeneralPane() {
  const t = useT();
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  return (
    <div>
      <Group title={t("settings.general.title")}>
        <Row title={t("settings.general.application.menu-bar-icon")}>
          <Toggle
            checked={settings.showMenuBarIcon}
            onChange={(v) => void update({ showMenuBarIcon: v })}
            label={t("settings.general.application.menu-bar-icon")}
          />
        </Row>
        <Row title={t("settings.general.application.open-at-login")}>
          <Toggle
            checked={settings.openAtLogin}
            onChange={(v) => void update({ openAtLogin: v })}
            label={t("settings.general.application.open-at-login")}
          />
        </Row>
        <Row title={t("settings.general.application.sound-effects")}>
          <Toggle
            checked={settings.soundEffects}
            onChange={(v) => void update({ soundEffects: v })}
            label={t("settings.general.application.sound-effects")}
          />
        </Row>
      </Group>
    </div>
  );
}

function HistoryPane() {
  const t = useT();
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  return (
    <div>
      <Group title={t("settings.general.history.title")}>
        <div className="pt-1">
          <RadioList
            value={settings.historyCapacity}
            onChange={(v) => void update({ historyCapacity: v })}
            options={[
              { value: "forever", label: t("settings.general.history.forever") },
              { value: "1month", label: t("replica.history.1month") },
              { value: "1week", label: t("replica.history.1week") },
              { value: "1day", label: t("replica.history.1day") },
            ]}
          />
        </div>
        <p className="px-4 pb-2 pt-2 text-[11px] leading-snug text-neutral-400">
          {t("onboarding.mac-os.clipboard-history.subtitle")} ·{" "}
          {t("onboarding.mac-os.clipboard-history.unlimited-tooltip")}
        </p>
      </Group>
    </div>
  );
}

function PasteItemsPane() {
  const t = useT();
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  return (
    <div>
      <Group title={t("settings.general.paste-items.title")}>
        <div className="pt-1">
          <RadioList
            value={settings.directPaste ? "app" : "clipboard"}
            onChange={(v) => void update({ directPaste: v === "app" })}
            options={[
              { value: "app", label: t("settings.general.paste-items.active-app.title") },
              { value: "clipboard", label: t("settings.general.paste-items.clipboard.title") },
            ]}
          />
        </div>
        <p className="px-4 pb-1.5 pt-1.5 text-[11px] leading-snug text-neutral-400">
          {settings.directPaste
            ? t("settings.general.paste-items.active-app.info")
            : t("settings.general.paste-items.clipboard.info")}
        </p>
        {settings.directPaste && (
          <Row title={t("accessibility-access.enable-accessibility-access.regular.title")}>
            <button
              type="button"
              onClick={() => void invoke("open_accessibility_settings")}
              className="cursor-pointer rounded-[5px] bg-[rgba(255,255,255,0.09)] px-2.5 py-1 text-[12px] text-neutral-100 ring-1 ring-[rgba(255,255,255,0.12)] hover:bg-[rgba(255,255,255,0.14)]"
            >
              {t("settings.general.paste-items.accessibility-access-request")}
            </button>
          </Row>
        )}
      </Group>
      <Group>
        <Row title={t("settings.general.paste-items.plain-text")}>
          <Toggle
            checked={settings.alwaysPlainText}
            onChange={(v) => void update({ alwaysPlainText: v })}
            label={t("settings.general.paste-items.plain-text")}
          />
        </Row>
      </Group>
    </div>
  );
}

function RulesPane() {
  const t = useT();
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [apps, setApps] = useState<RunningApp[]>([]);

  useEffect(() => {
    if (!pickerOpen) return;
    void invoke<RunningApp[]>("running_apps").then(setApps).catch(console.error);
  }, [pickerOpen]);

  const ignored = useMemo(() => new Set(settings.ignoredApps), [settings.ignoredApps]);

  return (
    <div>
      <Group title={t("settings.rules.title")}>
        <Row title={t("settings.rules.autogenerated-content.title")} subtitle={t("settings.rules.autogenerated-content.subtitle")}>
          <Toggle
            checked={settings.ignoreTransientContent}
            onChange={(v) => void update({ ignoreTransientContent: v })}
            label={t("settings.rules.autogenerated-content.title")}
          />
        </Row>
        <Row title={t("settings.rules.confidential-content.title")} subtitle={t("settings.rules.confidential-content.subtitle")}>
          <Toggle
            checked={settings.ignoreConfidentialContent}
            onChange={(v) => void update({ ignoreConfidentialContent: v })}
            label={t("settings.rules.confidential-content.title")}
          />
        </Row>
        <Row title={t("settings.rules.link-preview.title")} subtitle={t("settings.rules.link-preview.subtitle")}>
          <Toggle
            checked={settings.linkPreviews}
            onChange={(v) => void update({ linkPreviews: v })}
            label={t("settings.rules.link-preview.title")}
          />
        </Row>
      </Group>
      <Group title={t("settings.rules.applications-history.title")}>
        <p className="px-4 pb-1.5 pt-1 text-[11px] text-neutral-400">
          {t("settings.rules.applications-history.subtitle")}
        </p>
        {settings.ignoredApps.length === 0 && !pickerOpen && (
          <p className="px-4 pb-2 text-[12px] text-neutral-500">{t("settings.rules.applications-history.empty")}</p>
        )}
        {settings.ignoredApps.map((id) => (
          <div key={id} className="flex items-center justify-between px-4 py-[4px]">
            <span className="truncate text-[12.5px] text-neutral-200">{id}</span>
            <button
              type="button"
              aria-label={t("general.action.delete")}
              onClick={() => void update({ ignoredApps: settings.ignoredApps.filter((x) => x !== id) })}
              className="cursor-pointer text-neutral-500 hover:text-[#ff6961]"
            >
              <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <path d="M18 6 6 18" />
                <path d="m6 6 12 12" />
              </svg>
            </button>
          </div>
        ))}
        <div className="relative px-4 py-1.5">
          <button
            type="button"
            onClick={() => setPickerOpen((v) => !v)}
            className="cursor-pointer text-[12.5px] font-medium text-[#4da3ff] hover:text-[#7cb8ff]"
          >
            {t("replica.rules.add-app")}
          </button>
          {pickerOpen && (
            <div
              className="absolute left-4 top-9 z-50 max-h-[220px] w-[300px] overflow-y-auto rounded-[8px] border border-[rgba(255,255,255,0.12)] py-1 shadow-2xl"
              style={{ backgroundColor: "rgba(42,43,46,0.98)" }}
            >
              {apps
                .filter((a) => !ignored.has(a.bundleId))
                .map((a) => (
                  <button
                    key={a.bundleId}
                    type="button"
                    onClick={() => {
                      void update({ ignoredApps: [...settings.ignoredApps, a.bundleId] });
                      setPickerOpen(false);
                    }}
                    className="flex w-full cursor-pointer items-center justify-between px-3 py-[3px] text-left text-[12.5px] text-neutral-100 hover:bg-[#0077fc] hover:text-white"
                  >
                    <span className="truncate">{a.name}</span>
                    <span className="ml-3 shrink-0 text-[10px] opacity-50">{a.bundleId}</span>
                  </button>
                ))}
            </div>
          )}
        </div>
      </Group>
    </div>
  );
}

function ShortcutsPane() {
  const t = useT();
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const resetShortcuts = useSettings((s) => s.resetShortcuts);
  const [confirmReset, setConfirmReset] = useState(false);

  const rows: Array<{ key: keyof Settings; label: string; modifiersOnly?: boolean }> = [
    { key: "activationShortcut", label: t("settings.shortcuts.activate-paste") },
    { key: "pasteStackShortcut", label: t("settings.shortcuts.activate-paste-stack") },
    { key: "nextPinboardShortcut", label: t("settings.shortcuts.next-pinboard") },
    { key: "previousPinboardShortcut", label: t("settings.shortcuts.previous-pinboard") },
    { key: "quickPasteModifier", label: t("settings.shortcuts.quick-paste"), modifiersOnly: true },
    { key: "plainTextModifier", label: t("settings.shortcuts.plain-text"), modifiersOnly: true },
  ];

  return (
    <div>
      <Group title={t("settings.shortcuts.title")}>
        {rows.map((row) => (
          <Row
            key={row.key}
            title={row.label}
            subtitle={row.modifiersOnly ? t("replica.shortcut.modifier-only") : undefined}
          >
            <ShortcutField
              value={settings[row.key] as string}
              modifiersOnly={row.modifiersOnly}
              onChange={(v) => void update({ [row.key]: v } as Partial<Settings>)}
            />
          </Row>
        ))}
      </Group>
      <div className="flex items-center justify-between px-4">
        <p className="text-[11px] text-neutral-500">
          {formatShortcut(settings.activationShortcut)} · {formatShortcut(settings.pasteStackShortcut)}
        </p>
        <div className="relative">
          <button
            type="button"
            onClick={() => setConfirmReset(true)}
            className="cursor-pointer rounded-[5px] bg-[rgba(255,255,255,0.09)] px-2.5 py-1 text-[12px] text-neutral-100 ring-1 ring-[rgba(255,255,255,0.12)] hover:bg-[rgba(255,255,255,0.14)]"
          >
            {t("settings.shortcuts.reset-shortcuts")}
          </button>
          {confirmReset && (
            <div
              className="absolute bottom-9 right-0 z-50 w-[240px] rounded-[10px] border border-[rgba(255,255,255,0.12)] p-3 shadow-2xl"
              style={{ backgroundColor: "rgba(42,43,46,0.98)" }}
            >
              <p className="mb-2.5 text-[12px] leading-snug text-neutral-200">
                {t("settings.shortcuts.reset-shortcuts-confirmation")}
              </p>
              <div className="flex justify-end gap-2">
                <button
                  type="button"
                  onClick={() => setConfirmReset(false)}
                  className="cursor-pointer rounded-[6px] bg-[rgba(255,255,255,0.08)] px-3 py-1 text-[12px] text-neutral-100"
                >
                  {t("general.action.cancel")}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    void resetShortcuts();
                    setConfirmReset(false);
                  }}
                  className="cursor-pointer rounded-[6px] bg-[#0a84ff] px-3 py-1 text-[12px] font-semibold text-white"
                >
                  {t("settings.shortcuts.reset-shortcuts")}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function AboutPane() {
  const t = useT();
  const [version, setVersion] = useState("0.1.0");
  useEffect(() => {
    void import("@tauri-apps/api/app").then(({ getVersion }) => getVersion().then(setVersion).catch(() => {}));
  }, []);
  return (
    <div className="flex h-full flex-col items-center pt-8">
      <img src={convertFileSrc("icons/app-256.png")} alt="Paste" className="h-[128px] w-[128px]" draggable={false} />
      <p className="mt-3 text-[17px] font-semibold text-neutral-100">Paste</p>
      <p className="mt-0.5 text-[12px] text-neutral-400">{t("general.version-format", version)}</p>
      <div className="mt-6 flex flex-col items-center gap-1.5">
        <a
          className="cursor-pointer text-[12.5px] text-[#4da3ff] hover:underline"
          onClick={() => openUrl("https://pasteapp.io/help")}
        >
          {t("general.help-center")}
        </a>
        <a
          className="cursor-pointer text-[12.5px] text-[#4da3ff] hover:underline"
          onClick={() => openUrl("https://pasteapp.io/privacy-policy")}
        >
          {t("general.privacy-policy")}
        </a>
        <a
          className="cursor-pointer text-[12.5px] text-[#4da3ff] hover:underline"
          onClick={() => openUrl("https://pasteapp.io/terms-of-use")}
        >
          {t("general.terms-of-use")}
        </a>
      </div>
      <p className="mt-auto pb-4 text-[11px] text-neutral-500">{t("preferences.made-with-love.title")}</p>
    </div>
  );
}

/** Settings window: 200pt sidebar + pane content (measured layout). */
export function SettingsApp() {
  const t = useT();
  const [pane, setPane] = useState<PaneId>("general");
  const load = useSettings((s) => s.load);

  useEffect(() => {
    void load();
  }, [load]);

  const panes: Array<{ id: PaneId; label: string }> = [
    { id: "general", label: t("settings.general.title") },
    { id: "history", label: t("settings.general.history.title") },
    { id: "paste", label: t("settings.general.paste-items.title") },
    { id: "rules", label: t("settings.rules.title") },
    { id: "shortcuts", label: t("settings.shortcuts.title") },
    { id: "about", label: t("general.about-paste") },
  ];

  return (
    <div className="flex h-full bg-[#2a2a2c] text-neutral-100">
      <aside className="flex w-[200px] shrink-0 flex-col border-r border-[rgba(255,255,255,0.07)] bg-[#333336] pt-3">
        {panes.map((p) => (
          <button
            key={p.id}
            type="button"
            onClick={() => setPane(p.id)}
            className={`mx-2 flex cursor-pointer items-center rounded-[6px] px-3 py-[5px] text-left text-[13px] ${
              pane === p.id ? "bg-[#0a84ff] text-white" : "text-neutral-200 hover:bg-[rgba(255,255,255,0.07)]"
            }`}
          >
            {p.label}
          </button>
        ))}
      </aside>
      <main className="min-w-0 flex-1 overflow-y-auto px-5 py-4">
        {pane === "general" && <GeneralPane />}
        {pane === "history" && <HistoryPane />}
        {pane === "paste" && <PasteItemsPane />}
        {pane === "rules" && <RulesPane />}
        {pane === "shortcuts" && <ShortcutsPane />}
        {pane === "about" && <AboutPane />}
      </main>
    </div>
  );
}
