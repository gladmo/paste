import { useEffect, useRef } from "react";

import { useT } from "../i18n";
import { THEME } from "../ui/theme";
import { usePanel } from "../stores/panel";

function MagnifierIcon({ size }: { size: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <circle cx="11" cy="11" r="7" />
      <path d="m20.5 20.5-4.6-4.6" />
    </svg>
  );
}

/**
 * Two-state search control matching the original toolbar:
 * collapsed = bare 16pt magnifier icon; expanded = input pill (opens on
 * click, on type-to-search, or when a query is present; collapses on
 * blur/Esc with an empty query).
 */
export function SearchBar({ focusRef }: { focusRef: React.RefObject<HTMLInputElement | null> }) {
  const t = useT();
  const query = usePanel((s) => s.query);
  const setQuery = usePanel((s) => s.setQuery);
  const searchOpen = usePanel((s) => s.searchOpen);
  const setSearchOpen = usePanel((s) => s.setSearchOpen);
  const inputRef = useRef<HTMLInputElement>(null);

  const expanded = searchOpen || query.length > 0;
  // Focus only when the field opens (not on every render, which would steal
  // focus back from the panel whenever it re-renders).
  const wasExpanded = useRef(false);

  useEffect(() => {
    if (focusRef) focusRef.current = inputRef.current;
    if (expanded && !wasExpanded.current) {
      inputRef.current?.focus();
      inputRef.current?.select();
    }
    wasExpanded.current = expanded;
  });

  if (!expanded) {
    return (
      <button
        type="button"
        aria-label={t("general.search")}
        title={t("general.search")}
        onClick={() => setSearchOpen(true)}
        className="flex h-[32px] w-[32px] shrink-0 cursor-pointer items-center justify-center rounded-[6px] transition-colors hover:bg-[rgba(255,255,255,0.08)]"
        style={{ color: THEME.search.icon }}
      >
        <MagnifierIcon size={THEME.search.iconSize} />
      </button>
    );
  }

  return (
    <div
      className="flex shrink-0 items-center gap-2 rounded-[6px] px-3"
      style={{
        width: THEME.search.widthOpen,
        height: THEME.search.height + 4,
        backgroundColor: THEME.search.bg,
      }}
    >
      <MagnifierIcon size={14} />
      <input
        ref={inputRef}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onBlur={() => {
          if (!query) setSearchOpen(false);
        }}
        placeholder={t("general.search")}
        spellCheck={false}
        className="w-full min-w-0 bg-transparent text-[13px] font-medium outline-none"
        style={{ color: THEME.search.text }}
      />
      {query && (
        <button
          type="button"
          aria-label={t("general.action.cancel")}
          onClick={(e) => {
            e.stopPropagation();
            setQuery("");
            inputRef.current?.focus();
          }}
          className="shrink-0 cursor-pointer"
          style={{ color: THEME.search.icon }}
        >
          <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <path d="M18 6 6 18" />
            <path d="m6 6 12 12" />
          </svg>
        </button>
      )}
    </div>
  );
}
