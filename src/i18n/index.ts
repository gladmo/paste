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
    "replica.tab.history": "Clipboard History",
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
    "replica.history.1month": "1 Month",
    "replica.history.1week": "1 Week",
    "replica.history.1day": "1 Day",
    "replica.rules.add-app": "Add Application…",
    "replica.shortcut.modifier-only": "Modifier keys only",
    "replica.settings.language": "Language",
    "replica.settings.language-system": "System Default",
    "replica.tray.show": "Show Paste",
    "replica.tray.pause-1h": "For 1 Hour",
    "replica.tray.pause-until-tomorrow": "Until Tomorrow",
    "replica.tray.pause-indefinitely": "Indefinitely",
    "replica.tray.resume": "Resume Paste",
    "replica.menu.more": "More",
    "replica.preview.unavailable": "Preview unavailable",
    "replica.type.text": "Text",
    "replica.type.link": "Link",
    "replica.type.file": "File",
    "replica.type.image": "Image",
    "replica.type.color": "Color",
    "replica.meta.characters": "%@ characters",
    "replica.meta.character": "%@ character",
    "replica.meta.files": "%@ files",
    "replica.meta.file": "File",
    "replica.meta.items": "%@ items",
    "replica.meta.item": "%@ item",
    "replica.meta.lines": "%@ lines",
    "replica.meta.line": "%@ line",
    "replica.meta.words": "%@ words",
    "replica.meta.word": "%@ word",
  },
  "zh-Hans": {
    "replica.tab.history": "剪贴板历史记录",
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
    "replica.history.1month": "1 个月",
    "replica.history.1week": "1 周",
    "replica.history.1day": "1 天",
    "replica.rules.add-app": "添加应用程序…",
    "replica.shortcut.modifier-only": "仅修饰键",
    "replica.settings.language": "语言",
    "replica.settings.language-system": "跟随系统",
    "replica.tray.show": "显示 Paste",
    "replica.tray.pause-1h": "1 小时",
    "replica.tray.pause-until-tomorrow": "直到明天",
    "replica.tray.pause-indefinitely": "无限期",
    "replica.tray.resume": "恢复 Paste",
    "replica.menu.more": "更多",
    "replica.preview.unavailable": "无法预览此项目",
    "replica.type.text": "文本",
    "replica.type.link": "链接",
    "replica.type.file": "文件",
    "replica.type.image": "图片",
    "replica.type.color": "颜色",
    "replica.meta.characters": "%@ 个字符",
    "replica.meta.character": "%@ 个字符",
    "replica.meta.files": "%@ 个文件",
    "replica.meta.file": "文件",
    "replica.meta.items": "%@项",
    "replica.meta.item": "%@项",
    "replica.meta.lines": "%@ 行数",
    "replica.meta.line": "%@ 行数",
    "replica.meta.words": "%@ 单词们",
    "replica.meta.word": "%@ 单词们",
  },
  "zh-Hant": {
    "replica.tab.history": "剪貼簿記錄",
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
    "replica.history.1month": "1 個月",
    "replica.history.1week": "1 週",
    "replica.history.1day": "1 天",
    "replica.rules.add-app": "加入應用程式…",
    "replica.shortcut.modifier-only": "僅修飾鍵",
    "replica.settings.language": "語言",
    "replica.settings.language-system": "跟隨系統",
    "replica.tray.show": "顯示 Paste",
    "replica.tray.pause-1h": "1 小時",
    "replica.tray.pause-until-tomorrow": "直到明天",
    "replica.tray.pause-indefinitely": "無限期",
    "replica.tray.resume": "恢復 Paste",
    "replica.menu.more": "更多",
    "replica.preview.unavailable": "無法預覽此項目",
    "replica.type.text": "文字",
    "replica.type.link": "連結",
    "replica.type.file": "檔案",
    "replica.type.image": "圖片",
    "replica.type.color": "顏色",
    "replica.meta.characters": "%@ 個字元",
    "replica.meta.character": "%@ 個字元",
    "replica.meta.files": "%@ 個檔案",
    "replica.meta.file": "檔案",
    "replica.meta.items": "%@ 項",
    "replica.meta.item": "%@ 項",
    "replica.meta.lines": "%@線",
    "replica.meta.line": "%@線",
    "replica.meta.words": "%@ 字詞",
    "replica.meta.word": "%@ 字詞",
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

/** Apply the persisted language setting ("" = follow the system). */
export function applyLanguageSetting(lang: string | null | undefined) {
  if (lang === "en" || lang === "zh-Hans" || lang === "zh-Hant") {
    useI18n.setState({ locale: lang });
  } else {
    useI18n.setState({ locale: detectLocale() });
  }
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
