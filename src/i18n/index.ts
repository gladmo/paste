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
  let s = TABLES[locale][key] ?? en[key] ?? key;
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
