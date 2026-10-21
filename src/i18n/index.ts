// Lightweight i18n: keys/strings mirror the original Paste app so the
// replica renders identical copy. Locale follows the system language.
import { create } from "zustand";
import { useMemo } from "react";
import { en } from "./en";
import { zhHans } from "./zh-Hans";
import { zhHant } from "./zh-Hant";

export type Locale = "en" | "zh-Hans" | "zh-Hant";
export type StringKey = string & {}; // original keys; kept loose for lookups

export const LOCALES: readonly Locale[] = ["en", "zh-Hans", "zh-Hant"];

const TABLES: Record<Locale, Record<string, string>> = {
  en,
  "zh-Hans": zhHans,
  "zh-Hant": zhHant,
};

/** Replica-specific labels that have no original localization key. */
const REPLICA: Record<Locale, Record<string, string>> = {
  en: {
    "replica.tab.history": "History",
    "replica.tab.pinned": "Pinned",
    "replica.pinboard.new": "New Pinboard",
    "replica.pinboard.name-placeholder": "Name",
    "replica.pinboard.pick-color": "Color",
    "replica.pinboard.rename-title": "Rename Pinboard",
    "replica.time.now": "now",
    "replica.time.min-ago": "%@min ago",
    "replica.time.h-ago": "%@h ago",
    "replica.time.d-ago": "%@d ago",
    "replica.menu.open-link": "Open Link",
  },
  "zh-Hans": {
    "replica.tab.history": "历史记录",
    "replica.tab.pinned": "已固定",
    "replica.pinboard.new": "新建固定板",
    "replica.pinboard.name-placeholder": "名称",
    "replica.pinboard.pick-color": "颜色",
    "replica.pinboard.rename-title": "重命名固定板",
    "replica.time.now": "刚刚",
    "replica.time.min-ago": "%@分钟前",
    "replica.time.h-ago": "%@小时前",
    "replica.time.d-ago": "%@天前",
    "replica.menu.open-link": "打开链接",
  },
  "zh-Hant": {
    "replica.tab.history": "歷史記錄",
    "replica.tab.pinned": "已固定",
    "replica.pinboard.new": "新增固定板",
    "replica.pinboard.name-placeholder": "名稱",
    "replica.pinboard.pick-color": "顏色",
    "replica.pinboard.rename-title": "重新命名固定板",
    "replica.time.now": "剛剛",
    "replica.time.min-ago": "%@分鐘前",
    "replica.time.h-ago": "%@小時前",
    "replica.time.d-ago": "%@天前",
    "replica.menu.open-link": "打開連結",
  },
};

export const LOCALE_LABELS: Record<Locale, string> = {
  en: "English",
  "zh-Hans": "简体中文",
  "zh-Hant": "繁體中文",
};

/** Resolve the app locale from the system languages. */
export function detectLocale(): Locale {
  const langs =
    typeof navigator !== "undefined" &&
    ((navigator.languages && navigator.languages.length) || navigator.language)
      ? Array.from(navigator.languages ?? [navigator.language])
      : ["en"];
  for (const raw of langs) {
    const tag = raw.toLowerCase();
    if (tag.startsWith("zh")) {
      if (/(hant|tw|hk|mo)/.test(tag)) return "zh-Hant";
      return "zh-Hans";
    }
    if (tag.startsWith("en")) return "en";
  }
  return "en";
}

/** Translate `key`, substituting each %@ with the next positional arg. */
export function translate(locale: Locale, key: StringKey, ...args: (string | number)[]): string {
  let s = REPLICA[locale][key] ?? TABLES[locale][key] ?? en[key] ?? key;
  for (const arg of args) {
    s = s.replace("%@", String(arg));
  }
  return s;
}

interface I18nState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
}

export const useI18n = create<I18nState>((set) => ({
  locale: detectLocale(),
  setLocale: (locale) => set({ locale }),
}));

/** Reactive translator hook; re-renders when the locale changes. */
export function useT() {
  const locale = useI18n((s) => s.locale);
  return useMemo(
    () =>
      (key: StringKey, ...args: (string | number)[]) =>
        translate(locale, key, ...args),
    [locale],
  );
}
