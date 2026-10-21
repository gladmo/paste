import { useEffect, useRef, useState } from "react";

import { useT } from "../i18n";
import { THEME } from "../ui/theme";
import { usePanel } from "../stores/panel";

/** Search pill (#3d4246) matching the original toolbar: 190pt at rest,
 *  expanding while typing. */
export function SearchBar({ focusRef }: { focusRef: React.RefObject<HTMLInputElement | null> }) {
  const t = useT();
  const query = usePanel((s) => s.query);
  const setQuery = usePanel((s) => s.setQuery);
  const inputRef = useRef<HTMLInputElement>(null);
  const [focused, setFocused] = useState(false);

  useEffect(() => {
    if (focusRef) focusRef.current = inputRef.current;
  });

  const width = query || focused ? THEME.search.widthOpen : THEME.search.widthCollapsed;

  return (
    <div
      className="flex shrink-0 items-center gap-2 rounded-full px-3 transition-[width] duration-200"
      style={{
        width,
        height: THEME.search.height + 4,
        backgroundColor: THEME.search.bg,
      }}
      onClick={() => inputRef.current?.focus()}
    >
      <svg
        className="h-3.5 w-3.5 shrink-0"
        viewBox="0 0 24 24"
        fill="none"
        stroke={THEME.search.icon}
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <circle cx="11" cy="11" r="8" />
        <path d="m21 21-4.3-4.3" />
      </svg>
      <input
        ref={inputRef}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        placeholder={t("general.search")}
        spellCheck={false}
        className="w-full min-w-0 bg-transparent text-[12px] font-medium outline-none"
        style={{ color: THEME.search.text }}
      />
      {query && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            setQuery("");
            inputRef.current?.focus();
          }}
          className="shrink-0 cursor-pointer transition-colors hover:opacity-100"
          style={{ color: THEME.search.icon, opacity: 0.8 }}
          aria-label={t("general.action.cancel")}
        >
          <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M18 6 6 18" />
            <path d="m6 6 12 12" />
          </svg>
        </button>
      )}
    </div>
  );
}
