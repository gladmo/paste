import { useEffect, useRef, useState } from "react";

import { useT } from "../i18n";

/** macOS-style switch. */
export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className="relative h-[20px] w-[34px] shrink-0 cursor-pointer rounded-full transition-colors"
      style={{ backgroundColor: checked ? "#0a84ff" : "rgba(255,255,255,0.18)" }}
    >
      <span
        className="absolute top-[2px] h-[16px] w-[16px] rounded-full bg-white shadow transition-all"
        style={{ left: checked ? 16 : 2 }}
      />
    </button>
  );
}

/** Settings row: title + optional subtitle on the left, control on the right. */
export function Row({
  title,
  subtitle,
  children,
}: {
  title: string;
  subtitle?: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between gap-6 px-4 py-[7px]">
      <div className="min-w-0">
        <p className="text-[13px] text-neutral-100">{title}</p>
        {subtitle && <p className="mt-0.5 text-[11px] leading-snug text-neutral-400">{subtitle}</p>}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

export function Group({ title, children }: { title?: string; children: React.ReactNode }) {
  return (
    <div className="mb-4">
      {title && (
        <p className="mb-1.5 px-4 text-[11px] font-semibold uppercase tracking-wide text-neutral-500">
          {title}
        </p>
      )}
      <div className="rounded-[8px] bg-[rgba(255,255,255,0.045)] py-1">{children}</div>
    </div>
  );
}

/** Radio option list (Keep History retention, paste mode). */
export function RadioList<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T;
  options: Array<{ value: T; label: string; hint?: string }>;
  onChange: (v: T) => void;
}) {
  return (
    <div className="flex flex-col gap-0.5">
      {options.map((opt) => (
        <button
          key={opt.value}
          type="button"
          onClick={() => onChange(opt.value)}
          className="flex cursor-pointer items-center justify-between px-4 py-[6px] text-left hover:bg-[rgba(255,255,255,0.03)]"
        >
          <span className="text-[13px] text-neutral-100">{opt.label}</span>
          <span
            className="flex h-[15px] w-[15px] items-center justify-center rounded-full"
            style={{ boxShadow: "inset 0 0 0 1px rgba(255,255,255,0.35)" }}
          >
            {value === opt.value && <span className="h-[7px] w-[7px] rounded-full bg-[#0a84ff]" />}
          </span>
        </button>
      ))}
    </div>
  );
}

const SYMBOLS: Record<string, string> = {
  cmd: "⌘",
  shift: "⇧",
  alt: "⌥",
  ctrl: "⌃",
  arrow_left: "←",
  arrow_right: "→",
  arrow_up: "↑",
  arrow_down: "↓",
  enter: "↩",
  space: "Space",
};

/** Render a shortcut string ("cmd+shift+v") as macOS symbols. */
export function formatShortcut(s: string): string {
  const parts = s.split("+").filter(Boolean);
  const mods = ["ctrl", "alt", "shift", "cmd"];
  const sorted = [
    ...parts.filter((p) => mods.includes(p)),
    ...parts.filter((p) => !mods.includes(p)),
  ];
  return sorted
    .map((p) => SYMBOLS[p] ?? p.toUpperCase())
    .join(" ");
}

/** Interactive shortcut recorder field. */
export function ShortcutField({
  value,
  onChange,
  modifiersOnly = false,
}: {
  value: string;
  onChange: (v: string) => void;
  modifiersOnly?: boolean;
}) {
  const t = useT();
  const [recording, setRecording] = useState(false);
  const ref = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setRecording(false);
        return;
      }
      const mods: string[] = [];
      if (e.metaKey) mods.push("cmd");
      if (e.ctrlKey) mods.push("ctrl");
      if (e.altKey) mods.push("alt");
      if (e.shiftKey) mods.push("shift");
      if (modifiersOnly) {
        if (mods.length) {
          onChange(mods.join("+"));
          setRecording(false);
        }
        return;
      }
      const keyMap: Record<string, string> = {
        ArrowLeft: "arrow_left",
        ArrowRight: "arrow_right",
        ArrowUp: "arrow_up",
        ArrowDown: "arrow_down",
        Enter: "enter",
        " ": "space",
      };
      const key = e.key.length === 1 ? e.key.toLowerCase() : keyMap[e.key];
      if (!key) return;
      // Require at least one modifier for a global shortcut.
      if (!mods.length) return;
      onChange([...mods, key].join("+"));
      setRecording(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange, modifiersOnly]);

  return (
    <button
      ref={ref}
      type="button"
      onClick={() => setRecording((v) => !v)}
      className={`flex h-[24px] min-w-[110px] cursor-pointer items-center justify-center rounded-[5px] px-2.5 text-[12px] font-medium tabular-nums transition-colors ${
        recording
          ? "bg-[#0a84ff] text-white"
          : "bg-[rgba(255,255,255,0.09)] text-neutral-100 ring-1 ring-[rgba(255,255,255,0.12)] hover:bg-[rgba(255,255,255,0.14)]"
      }`}
    >
      {recording ? t("shortcut-recorder.recording") : formatShortcut(value) || t("shortcut-recorder.none")}
    </button>
  );
}
