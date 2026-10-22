import React from "react";

import { useT } from "../i18n";
import { THEME } from "../ui/theme";
import type { Clip } from "../types";
import { AppIcon, CardPreview, clipMeta, formatTime } from "./preview";

/** Small monochrome content-type glyph (Paste Stack rows reuse it). */
export function TypeIcon({ type, size = 12 }: { type: string; size?: number }) {
  const common = {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 2.2,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
  };
  if (type === "link") {
    return (
      <svg {...common}>
        <path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71" />
        <path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71" />
      </svg>
    );
  }
  if (type === "image") {
    return (
      <svg {...common}>
        <rect x="3" y="3" width="18" height="18" rx="2" />
        <circle cx="9" cy="9" r="2" />
        <path d="m21 15-3.09-3.09a2 2 0 0 0-2.82 0L6 21" />
      </svg>
    );
  }
  if (type === "file") {
    return (
      <svg {...common}>
        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
        <path d="M14 2v6h6" />
      </svg>
    );
  }
  if (type === "color") {
    return (
      <svg {...common}>
        <rect x="3" y="3" width="18" height="18" rx="3" />
      </svg>
    );
  }
  // text / richtext — "T" glyph
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round">
      <path d="M5 5h14" />
      <path d="M12 5v15" />
    </svg>
  );
}

/** Monogram fallback for source apps without a bundled icon (GO / S3 / GH …). */
const MONOGRAM_COLORS = ["#22d3ee", "#f59e0b", "#a78bfa", "#34d399", "#f472b6", "#60a5fa", "#fb923c"];

function monogram(name: string): { text: string; color: string } {
  const letters = name.replace(/[^\p{L}\p{N} ]/gu, "").trim();
  const words = letters.split(/\s+/).filter(Boolean);
  const text = words.length >= 2 ? words[0][0] + words[1][0] : letters.slice(0, 2);
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.charCodeAt(0)) >>> 0;
  return { text: (text || "?").toUpperCase(), color: MONOGRAM_COLORS[h % MONOGRAM_COLORS.length] };
}

function SourceBadge({ clip }: { clip: Clip }) {
  const mono = monogram(clip.sourceName ?? "?");
  return (
    <div className="relative flex h-7 w-7 shrink-0 items-center justify-center overflow-hidden rounded-[6px] bg-black/60 shadow-inner">
      <span className="absolute inset-0 flex items-center justify-center text-[11px] font-black tracking-tight" style={{ color: mono.color }}>
        {mono.text}
      </span>
      {clip.sourceApp && (
        <div className="absolute inset-0">
          <AppIcon bundleId={clip.sourceApp} name={clip.sourceName} size={28} />
        </div>
      )}
    </div>
  );
}

export interface CardHandlers {
  onActivate: () => void;
  onOpen: () => void;
  onContext: (e: React.MouseEvent) => void;
  onTogglePin: () => void;
  onSelectionClick: (e: React.MouseEvent) => void;
}

/**
 * Card-shelf design (paste-ui.html): 240×280 rounded-2xl card with a colored
 * type header (type name, relative time, source-app badge), a full preview
 * body and a meta footer ("59 characters" / domain + index number). The
 * active card is highlighted with a 2px #007aff ring.
 */
export function ClipCard({
  clip,
  index,
  isActive,
  showIndex,
  metaDown,
  selecting,
  selected,
  handlers,
}: {
  clip: Clip;
  index: number;
  isActive: boolean;
  showIndex: boolean;
  metaDown: boolean;
  selecting: boolean;
  selected: boolean;
  handlers: CardHandlers;
}) {
  const t = useT();
  const typeKey = `replica.type.${clip.contentType === "richtext" ? "text" : clip.contentType}`;
  const headerBg = THEME.headerBg[clip.contentType] ?? THEME.headerBg.default;
  const meta = clipMeta(clip, t);

  return (
    <article
      data-index={index}
      role="option"
      aria-selected={isActive}
      onMouseEnter={handlers.onActivate}
      onClick={(e) => {
        if (selecting || e.metaKey || e.ctrlKey) {
          handlers.onSelectionClick(e);
        } else {
          handlers.onActivate();
        }
      }}
      onDoubleClick={handlers.onOpen}
      onContextMenu={handlers.onContext}
      className={`group relative flex shrink-0 cursor-pointer flex-col overflow-hidden transition-all duration-150 ${
        isActive
          ? "rounded-2xl shadow-2xl ring-2 ring-[#007aff]"
          : "rounded-2xl border shadow-xl hover:border-[rgba(255,255,255,0.2)]"
      }`}
      style={{
        width: THEME.card.width,
        height: THEME.card.height,
        backgroundColor: THEME.card.bg,
        ...(isActive ? {} : { borderColor: "rgba(255,255,255,0.05)" }),
      }}
    >
      {/* type header: label + time + source app badge */}
      <header className="flex items-center justify-between px-3.5 py-2 text-white" style={{ backgroundColor: headerBg }}>
        <div className="flex min-w-0 flex-col">
          <span className="truncate text-[13px] font-bold leading-tight">{t(typeKey)}</span>
          <span className="text-[11px] leading-tight" style={{ color: "rgba(255,255,255,0.8)" }}>
            {formatTime(clip.copiedAt, t)}
          </span>
        </div>
        <SourceBadge clip={clip} />
      </header>

      {/* preview body + meta footer */}
      <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
        <div className={`relative min-h-0 flex-1 overflow-hidden ${clip.contentType === "image" ? "" : "p-3"}`}>
          <CardPreview clip={clip} />

          {/* selection checkbox */}
          {(selecting || selected) && (
            <div
              className="absolute left-[8px] top-[8px] z-10 flex h-[18px] w-[18px] items-center justify-center rounded-full"
              style={{
                backgroundColor: selected ? "#007aff" : "rgba(0,0,0,0.45)",
                boxShadow: "inset 0 0 0 1.5px rgba(255,255,255,0.6)",
              }}
            >
              {selected && (
                <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="3.5" strokeLinecap="round" strokeLinejoin="round">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              )}
            </div>
          )}

          {/* hover pin toggle */}
          {!selecting && (
            <button
              type="button"
              aria-label={clip.pinned ? t("general.action.unpin") : t("general.action.pin")}
              title={clip.pinned ? t("general.action.unpin") : t("general.action.pin")}
              onClick={(e) => {
                e.stopPropagation();
                handlers.onTogglePin();
              }}
              className={`absolute right-[8px] top-[8px] z-10 cursor-pointer rounded-md p-1 transition-opacity ${
                clip.pinned
                  ? "text-blue-300"
                  : "text-neutral-300 opacity-0 group-hover:opacity-70 hover:!opacity-100"
              }`}
              style={{ backgroundColor: "rgba(0,0,0,0.35)" }}
            >
              <svg
                className="h-3.5 w-3.5"
                viewBox="0 0 24 24"
                fill={clip.pinned ? "currentColor" : "none"}
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M12 17v5" />
                <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1z" />
              </svg>
            </button>
          )}
        </div>

        {/* meta footer: content meta + index */}
        <footer
          className="flex shrink-0 items-center justify-between border-t pt-1.5"
          style={{ borderColor: "rgba(255,255,255,0.05)", paddingInline: 12, paddingBottom: 10, color: THEME.card.footerText, fontSize: THEME.card.footerFontSize }}
        >
          <span className="truncate font-medium">{meta}</span>
          {showIndex && (
            <span
              className={`ml-2 shrink-0 tabular-nums ${
                metaDown && index < 9 ? "rounded-full bg-[rgba(255,255,255,0.22)] px-1.5 font-bold text-white" : ""
              }`}
            >
              {index + 1}
            </span>
          )}
        </footer>
      </div>
    </article>
  );
}
