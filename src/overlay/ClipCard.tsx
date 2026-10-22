import React from "react";

import { useT } from "../i18n";
import { THEME } from "../ui/theme";
import type { Clip } from "../types";
import { CardPreview, formatTime } from "./preview";

/** Small monochrome type glyph (Paste Stack rows; kept for reuse). */
export function TypeIcon({ type, size = 12 }: { type: string; size?: number }) {
  const common = {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 2,
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
  if (type === "richtext") {
    return (
      <svg {...common}>
        <path d="M4 7V5h16v2" />
        <path d="M12 5v14" />
        <path d="M9 19h6" />
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
  // text
  return (
    <svg {...common}>
      <path d="M17 6.1H3" />
      <path d="M21 12.1H3" />
      <path d="M15.1 18H3" />
    </svg>
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
 * One history card (measured geometry): 248pt outer box with a 4pt border
 * slot (blue when active, transparent otherwise), 240pt content, 11pt
 * radius, no glow; footer is a single centered gray line ("app · time").
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
  const c = THEME.card;

  const footerCaption = clip.sourceName
    ? `${clip.sourceName} · ${formatTime(clip.copiedAt, t)}`
    : formatTime(clip.copiedAt, t);

  return (
    <div
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
      className="group relative flex shrink-0 cursor-pointer flex-col box-border"
      style={{
        width: c.outer,
        height: c.outer,
        borderRadius: c.radius,
        border: `${c.border}px solid ${isActive ? c.selectedBorder : "transparent"}`,
        backgroundColor: c.bg,
      }}
    >
      {/* content (240pt inside the border) */}
      <div className="relative min-h-0 flex-1 overflow-hidden" style={{ borderRadius: c.radius - c.border }}>
        <CardPreview clip={clip} />

        {/* selection checkbox overlay */}
        {(selecting || selected) && (
          <div
            className="absolute left-[8px] top-[8px] z-10 flex h-[18px] w-[18px] items-center justify-center rounded-full"
            style={{
              backgroundColor: selected ? "#0077fc" : "rgba(0,0,0,0.45)",
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
            className={`absolute right-[8px] top-[8px] z-10 cursor-pointer rounded-md p-1 transition-colors ${
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

      {/* footer: centered single gray line */}
      <div
        className="flex shrink-0 items-center justify-center gap-1.5"
        style={{ height: c.footerHeight, color: c.footerText, fontSize: c.footerFontSize }}
      >
        <span className="truncate font-medium tabular-nums">{footerCaption}</span>
        {showIndex && metaDown && index < 9 && (
          <span className="flex h-[15px] w-[15px] shrink-0 items-center justify-center rounded-full bg-[rgba(255,255,255,0.22)] text-[9px] font-bold text-white ring-1 ring-[rgba(255,255,255,0.35)]">
            {index + 1}
          </span>
        )}
      </div>
    </div>
  );
}
