import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

/** Mirrors src-tauri/src/settings.rs (serde camelCase). */
export interface Settings {
  activationShortcut: string;
  pasteStackShortcut: string;
  nextPinboardShortcut: string;
  previousPinboardShortcut: string;
  plainTextModifier: string;
  quickPasteModifier: string;
  showMenuBarIcon: boolean;
  soundEffects: boolean;
  directPaste: boolean;
  alwaysPlainText: boolean;
  historyCapacity: "forever" | "1month" | "1week" | "1day";
  ignoredApps: string[];
  ignoreTransientContent: boolean;
  ignoreConfidentialContent: boolean;
  linkPreviews: boolean;
  openAtLogin: boolean;
  /** "" = follow system; otherwise a concrete Locale. */
  language: string;
}

export const DEFAULT_SETTINGS: Settings = {
  activationShortcut: "cmd+shift+v",
  pasteStackShortcut: "ctrl+cmd+v",
  nextPinboardShortcut: "alt+cmd+arrow_right",
  previousPinboardShortcut: "alt+cmd+arrow_left",
  plainTextModifier: "alt",
  quickPasteModifier: "cmd",
  showMenuBarIcon: true,
  soundEffects: true,
  directPaste: true,
  alwaysPlainText: false,
  historyCapacity: "forever",
  ignoredApps: [],
  ignoreTransientContent: true,
  ignoreConfidentialContent: true,
  linkPreviews: true,
  openAtLogin: false,
  language: "",
};

interface SettingsState {
  settings: Settings;
  loaded: boolean;
  load: () => Promise<void>;
  update: (patch: Partial<Settings>) => Promise<void>;
  resetShortcuts: () => Promise<void>;
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  loaded: false,

  load: async () => {
    try {
      const s = await invoke<Settings>("get_settings");
      set({ settings: { ...DEFAULT_SETTINGS, ...s }, loaded: true });
    } catch (e) {
      console.error("get_settings failed", e);
      set({ loaded: true });
    }
  },

  update: async (patch) => {
    const next = { ...get().settings, ...patch };
    set({ settings: next });
    try {
      await invoke("set_settings", { settings: next });
    } catch (e) {
      console.error("set_settings failed", e);
    }
  },

  resetShortcuts: async () => {
    await get().update({
      activationShortcut: DEFAULT_SETTINGS.activationShortcut,
      pasteStackShortcut: DEFAULT_SETTINGS.pasteStackShortcut,
      nextPinboardShortcut: DEFAULT_SETTINGS.nextPinboardShortcut,
      previousPinboardShortcut: DEFAULT_SETTINGS.previousPinboardShortcut,
      plainTextModifier: DEFAULT_SETTINGS.plainTextModifier,
      quickPasteModifier: DEFAULT_SETTINGS.quickPasteModifier,
    });
  },
}));
