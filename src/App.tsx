import { useEffect, useMemo, useRef, useState } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import DOMPurify from "dompurify";

type Clip = {
  id: number;
  contentType: string;
  content: string;
  plainContent?: string;
  title: string | null;
  copiedAt: number;
  pinned: boolean;
  sourceApp?: string | null;
  sourceName?: string | null;
  pinnedAt?: number | null;
};

type ClipType = "text" | "image" | "link" | "richtext";

const TYPE_STYLES: Record<ClipType, { label: string; chip: string }> = {
  text: { label: "Text", chip: "bg-blue-500" },
  image: { label: "Image", chip: "bg-emerald-500" },
  link: { label: "Link", chip: "bg-violet-500" },
  richtext: { label: "Rich", chip: "bg-amber-500" },
};

function clipType(t: string): ClipType {
  if (t === "image" || t === "link" || t === "richtext") return t;
  return "text";
}

/**
 * textutil emits HTML with a <style> block + classes (p.p1, span.s1).
 * DOMPurify strips <style>, so inline the rules into style attributes
 * before sanitizing — otherwise all colors/fonts are lost.
 */
function inlineStyles(html: string): string {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const styleEl = doc.querySelector("style");
  if (!styleEl) return html;
  const ruleRe = /([^{}]+)\{([^}]*)\}/g;
  let m: RegExpExecArray | null;
  while ((m = ruleRe.exec(styleEl.textContent ?? ""))) {
    const selector = m[1].trim();
    const decls = m[2].trim();
    if (!selector || !decls) continue;
    doc.querySelectorAll(selector).forEach((el) => {
      const existing = el.getAttribute("style") ?? "";
      el.setAttribute("style", existing ? `${existing}; ${decls}` : decls);
    });
  }
  styleEl.remove();
  return doc.body ? doc.body.innerHTML : html;
}

function formatTime(ts: number): string {
  const diff = Date.now() - ts;
  const min = Math.floor(diff / 60000);
  if (min < 1) return "now";
  if (min < 60) return `${min}min ago`;
  const hours = Math.floor(min / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days}d ago`;
  return new Date(ts).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}

function linkDomain(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

function formatSize(bytes: number): string {
  return bytes >= 1048576
    ? `${(bytes / 1048576).toFixed(1)} MB`
    : `${Math.max(1, Math.round(bytes / 1024))} KB`;
}

/** Image meta line: dimensions + file size, resolved from the asset URL. */
function ImageMeta({ src }: { src: string }) {
  const [meta, setMeta] = useState("");

  useEffect(() => {
    let alive = true;
    const img = new Image();
    img.onload = () => {
      if (!alive) return;
      const dims = `${img.naturalWidth}×${img.naturalHeight}`;
      setMeta((prev) => (prev ? `${dims} · ${prev}` : dims));
    };
    img.src = src;
    fetch(src)
      .then((r) => r.blob())
      .then((b) => {
        if (!alive) return;
        setMeta((prev) => (prev ? `${prev} · ${formatSize(b.size)}` : formatSize(b.size)));
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [src]);

  return <span className="truncate">{meta}</span>;
}

/** Card bottom line: char count / image size / link page title. */
function CardMeta({ clip, ctype }: { clip: Clip; ctype: ClipType }) {
  if (ctype === "text" || ctype === "richtext") {
    return (
      <span className="truncate">
        {(clip.plainContent ?? clip.content).length} chars
      </span>
    );
  }
  if (ctype === "image") {
    return <ImageMeta src={convertFileSrc(clip.content)} />;
  }
  return <span className="truncate">{clip.title || linkDomain(clip.content)}</span>;
}

/** Source app icon in the card's top-right corner (hidden when unknown). */
function AppIcon({ bundleId, name }: { bundleId?: string | null; name?: string | null }) {
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    if (!bundleId) {
      setSrc(null);
      return;
    }
    invoke<string | null>("app_icon", { bundleId })
      .then((p) => {
        if (alive && p) setSrc(convertFileSrc(p));
        if (import.meta.env.DEV) {
          void invoke("debug_log", { line: `app_icon ${bundleId} -> ${p ?? "null"}` });
        }
      })
      .catch((e) => {
        if (import.meta.env.DEV) {
          void invoke("debug_log", { line: `app_icon FAILED ${bundleId} ${e}` });
        }
      });
    return () => {
      alive = false;
    };
  }, [bundleId]);

  if (!src) return null;
  return (
    <img
      src={src}
      alt={name ?? "source app"}
      title={name ?? undefined}
      className="h-[38px] w-[38px] rounded-[8px] object-contain"
      draggable={false}
    />
  );
}

type CardHandlers = {
  onActivate: () => void;
  onPaste: () => void;
  onContext: (e: React.MouseEvent, clip: Clip) => void;
  onTogglePin: () => void;
};

/** A single clip card; shared by the history and pinned rows. */
function ClipCard({
  clip,
  index,
  isActive,
  showIndex,
  metaDown,
  handlers,
}: {
  clip: Clip;
  index: number;
  isActive: boolean;
  showIndex: boolean;
  metaDown: boolean;
  handlers: CardHandlers;
}) {
  const ctype = clipType(clip.contentType);
  const style = TYPE_STYLES[ctype];
  return (
    <div
      data-index={index}
      role="option"
      aria-selected={isActive}
      onMouseEnter={handlers.onActivate}
      onClick={handlers.onActivate}
      onDoubleClick={handlers.onPaste}
      onContextMenu={(e) => handlers.onContext(e, clip)}
      className={`group flex h-[245px] w-[245px] shrink-0 cursor-pointer flex-col rounded-xl border p-2 transition-colors ${
        isActive
          ? "border-[rgba(96,165,250,0.7)] bg-[rgba(59,130,246,0.15)] ring-1 ring-[rgba(96,165,250,0.3)]"
          : "border-[rgba(255,255,255,0.06)] bg-[rgba(255,255,255,0.06)] hover:border-[rgba(96,165,250,0.35)]"
      }`}
    >
      <div className="flex items-start justify-between px-0.5 pb-1.5">
        <div className="flex flex-col items-start gap-1">
          <span
            className={`rounded-full px-2 py-0.5 text-[10px] font-bold uppercase tracking-wide text-white ${style.chip}`}
          >
            {style.label}
          </span>
          <span className="text-[10px] font-medium tabular-nums text-white">
            {formatTime(clip.copiedAt)}
          </span>
        </div>
        <div className="flex items-center gap-1.5">
          <button
            type="button"
            aria-label={clip.pinned ? "取消固定" : "固定"}
            title={clip.pinned ? "取消固定" : "固定"}
            onClick={(e) => {
              e.stopPropagation();
              handlers.onTogglePin();
            }}
            className={`cursor-pointer transition-colors ${
              clip.pinned
                ? "text-blue-300"
                : "text-neutral-400 opacity-0 group-hover:opacity-70 hover:opacity-100 hover:text-blue-200"
            }`}
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
          <AppIcon bundleId={clip.sourceApp} name={clip.sourceName} />
        </div>
      </div>
      <CardPreview clip={clip} ctype={ctype} />
      <div className="mt-1.5 flex items-center justify-between px-0.5 text-[10px] font-medium tabular-nums text-white">
        <CardMeta clip={clip} ctype={ctype} />
        {showIndex && metaDown && index < 9 && (
          <span className="flex h-3.5 w-3.5 shrink-0 items-center justify-center rounded-full bg-[rgba(255,255,255,0.25)] text-[9px] font-bold text-white ring-1 ring-[rgba(255,255,255,0.3)]">
            {index + 1}
          </span>
        )}
      </div>
    </div>
  );
}

/** Read the background color of the copied content (e.g. GoLand's dark
 *  editor bg), so the card content area matches the original editor. */
function extractBgColor(html: string): string | undefined {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const firstStyled = doc.body.querySelector("[style*='background']");
  if (!firstStyled) return undefined;
  const m = (firstStyled.getAttribute("style") ?? "").match(
    /background-color:\s*([^;]+)/,
  );
  const color = m?.[1]?.trim();
  return color && color !== "transparent" ? color : undefined;
}

function CardPreview({ clip, ctype }: { clip: Clip; ctype: ClipType }) {
  if (ctype === "image") {
    return (
      <div className="min-h-0 flex-1 overflow-hidden rounded-lg">
        <img
          src={convertFileSrc(clip.content)}
          alt="clip"
          className="h-full w-full object-cover"
          draggable={false}
        />
      </div>
    );
  }
  if (ctype === "richtext") {
    // Clipboard HTML is untrusted: sanitize before rendering so copied
    // content can never inject scripts into the panel.
    const sanitized = DOMPurify.sanitize(inlineStyles(clip.content));
    const bg = extractBgColor(sanitized);
    if (import.meta.env.DEV) {
      void invoke("debug_log", { line: `bg=${bg} | ${sanitized}` });
    }
    return (
      <div
        className="min-h-0 flex-1 overflow-hidden rounded-lg bg-[rgba(255,255,255,0.04)] p-2"
        style={bg ? { backgroundColor: bg } : undefined}
      >
        <div
          className="rich-preview h-full overflow-hidden text-[13px] leading-snug text-white"
          dangerouslySetInnerHTML={{ __html: sanitized }}
        />
      </div>
    );
  }
  if (ctype === "link") {
    return (
      <div className="min-h-0 flex-1 space-y-1">
        <p className="truncate text-[13px] font-semibold text-white">
          {linkDomain(clip.content)}
        </p>
        <p className="line-clamp-3 break-all text-[11px] leading-snug text-white">
          {clip.content}
        </p>
      </div>
    );
  }
  return (
    <div className="min-h-0 flex-1 overflow-hidden rounded-lg bg-[rgba(255,255,255,0.04)] p-2">
      <p className="line-clamp-4 whitespace-pre-wrap break-all text-[13px] leading-snug text-white">
        {clip.content}
      </p>
    </div>
  );
}

export default function App() {
  const isSettings = window.location.hash === "#/settings";
  if (isSettings) return <SettingsPlaceholder />;
  return <Overlay />;
}

function SettingsPlaceholder() {
  return (
    <div className="flex h-full items-center justify-center bg-zinc-900 text-neutral-400">
      Settings window (coming in Phase 4)
    </div>
  );
}

function Overlay() {
  // Dev-only: the hidden panel's WebView misses HMR updates (page suspended),
  // so every summon reloads to guarantee fresh code. Production builds never
  // reload (import.meta.env.DEV is false).
  const DEV_RELOAD = import.meta.env.DEV;
  const reloadedOnShow = typeof sessionStorage !== "undefined"
    && sessionStorage.getItem("paste:dev-reloaded") === "1";
  if (reloadedOnShow) sessionStorage.removeItem("paste:dev-reloaded");

  useEffect(() => {
    if (import.meta.env.DEV) {
      const nav = performance.getEntriesByType("navigation")[0] as
        | PerformanceNavigationTiming
        | undefined;
      void invoke("debug_log", {
        line: `ui mounted reloaded=${reloadedOnShow} nav=${nav?.type ?? "unknown"}`,
      });
    }
  }, []);

  const [query, setQuery] = useState("");
  const [anim, setAnim] = useState(reloadedOnShow ? 1 : 0);
  const [clips, setClips] = useState<Clip[]>([]);
  const [pinned, setPinned] = useState<Clip[]>([]);
  const [active, setActive] = useState(0);
  const [searchOpen, setSearchOpen] = useState(false);
  const [tab, setTab] = useState<"history" | "pinned">("history");
  const [metaDown, setMetaDown] = useState(false);
  const [menu, setMenu] = useState<{
    x: number;
    y: number;
    id: number;
    pinned: boolean;
  } | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const closingRef = useRef(false);
  const queryRef = useRef(query);
  queryRef.current = query;

  /** Cards currently on display: history/search results, or the pinned list. */
  const visible = useMemo(() => {
    if (tab !== "pinned") return clips;
    const q = query.trim().toLowerCase();
    if (!q) return pinned;
    return pinned.filter((c) =>
      (c.plainContent ?? c.content).toLowerCase().includes(q),
    );
  }, [tab, clips, pinned, query]);

  async function refresh(q: string) {
    try {
      const rows = q.trim()
        ? await invoke<Clip[]>("search_clips", { query: q.trim() })
        : await invoke<Clip[]>("recent_clips", { limit: 100 });
      setClips(rows);
      setActive(0);
      if (import.meta.env.DEV) {
        void invoke("debug_log", { line: `refresh clips=${rows.length}` });
      }
    } catch (e) {
      console.error("refresh clips failed", e);
      if (import.meta.env.DEV) {
        void invoke("debug_log", { line: `refresh clips FAILED ${e}` });
      }
    }
    try {
      const p = await invoke<Clip[]>("pinned_clips", { limit: 50 });
      setPinned(p);
      if (import.meta.env.DEV) {
        void invoke("debug_log", { line: `pinned n=${p.length}` });
      }
    } catch (e) {
      console.error("refresh pinned failed", e);
      if (import.meta.env.DEV) {
        void invoke("debug_log", { line: `pinned FAILED ${e}` });
      }
    }
  }

  /** Open the pin/unpin context menu at the cursor. */
  const openMenu = (e: React.MouseEvent, clip: Clip) => {
    e.preventDefault();
    setMenu({
      x: Math.min(e.clientX, window.innerWidth - 140),
      y: Math.min(e.clientY, window.innerHeight - 80),
      id: clip.id,
      pinned: clip.pinned,
    });
  };

  /** Fade the panel out, then run the action (hide / paste). */
  const fadeOut = (then: () => void) => {
    if (closingRef.current) return;
    closingRef.current = true;
    setAnim(0);
    setTimeout(() => {
      closingRef.current = false;
      then();
    }, 160);
  };

  /** Open the search drawer (optionally pre-filled) and focus its input. */
  const openSearch = (initial = "") => {
    setTab("history");
    setSearchOpen(true);
    setActive(0);
    setQuery(initial);
    requestAnimationFrame(() => inputRef.current?.focus());
  };

  /** Close the search drawer and reset the query. */
  const closeSearch = () => {
    setSearchOpen(false);
    setQuery("");
    setActive(0);
  };

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    timer = setTimeout(() => void refresh(query), 150);
    return () => {
      if (timer) clearTimeout(timer);
    };
  }, [query]);

  useEffect(() => {
    const unlisten = listen("overlay:show", () => {
      if (DEV_RELOAD) {
        sessionStorage.setItem("paste:dev-reloaded", "1");
        window.location.reload();
        return;
      }
      setQuery("");
      setActive(0);
      setSearchOpen(false);
      setTab("history");
      setMetaDown(false);
      closingRef.current = false;
      setAnim(0);
      if (listRef.current) listRef.current.scrollLeft = 0;
      void refresh("");
      requestAnimationFrame(() =>
        requestAnimationFrame(() => setAnim(1)),
      );
    });
    const unlistenUpdated = listen("clips:updated", () => {
      void refresh(queryRef.current);
    });
    const unlistenHide = listen("overlay:hide", () => {
      setMetaDown(false);
      setAnim(0);
    });
    return () => {
      unlisten.then((fn) => fn());
      unlistenUpdated.then((fn) => fn());
      unlistenHide.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-index="${active}"]`)
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [active]);

  useEffect(() => {
    const onMeta = (e: KeyboardEvent) => {
      if (e.key === "Meta" || e.key === "MetaLeft" || e.key === "MetaRight") {
        setMetaDown(e.type === "keydown");
      }
    };
    window.addEventListener("keydown", onMeta);
    window.addEventListener("keyup", onMeta);
    return () => {
      window.removeEventListener("keydown", onMeta);
      window.removeEventListener("keyup", onMeta);
    };
  }, []);

  useEffect(() => {
    const closeMenu = () => setMenu(null);
    window.addEventListener("mousedown", closeMenu);
    window.addEventListener("blur", closeMenu);
    return () => {
      window.removeEventListener("mousedown", closeMenu);
      window.removeEventListener("blur", closeMenu);
    };
  }, []);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (menu) {
          setMenu(null);
        } else if (searchOpen) {
          closeSearch();
        } else {
          fadeOut(() => void invoke("hide_overlay"));
        }
      } else if (e.metaKey && e.key >= "1" && e.key <= "9") {
        e.preventDefault();
        const idx = Number(e.key) - 1;
        if (visible[idx]) {
          fadeOut(() => void invoke("paste_clip", { id: visible[idx].id }));
        }
      } else if (
        e.key.length === 1 &&
        e.key !== "Tab" &&
        !e.metaKey &&
        !e.ctrlKey &&
        !e.altKey &&
        !searchOpen
      ) {
        e.preventDefault();
        openSearch(e.key);
      } else if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        e.preventDefault();
        setActive((i) => Math.min(i + 1, Math.max(visible.length - 1, 0)));
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        e.preventDefault();
        setActive((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter" && visible[active]) {
        fadeOut(() => void invoke("paste_clip", { id: visible[active].id }));
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [active, visible, searchOpen, menu]);

  return (
    <div
      className={`relative flex h-full flex-col overflow-hidden rounded-2xl border border-[rgba(255,255,255,0.1)] bg-[rgba(0,0,0,0.8)] shadow-2xl backdrop-blur-2xl transition-all duration-150 ease-out ${
        anim ? "translate-y-0 opacity-100" : "translate-y-4 opacity-0"
      }`}
    >
      <div className="flex items-center justify-center gap-2 px-5 pt-2.5 pb-2.5">
        <div
          role="button"
          tabIndex={0}
          onClick={searchOpen ? undefined : () => openSearch()}
          className={`flex items-center gap-2 overflow-hidden rounded-full border py-1 transition-all duration-200 ease-out ${
            searchOpen
              ? "w-[300px] border-[rgba(96,165,250,0.7)] bg-[rgba(59,130,246,0.15)] text-blue-200"
              : "w-[78px] cursor-pointer border-[rgba(255,255,255,0.1)] bg-[rgba(255,255,255,0.06)] text-neutral-300 hover:bg-[rgba(255,255,255,0.12)]"
          }`}
        >
          <svg
            className="ml-3.5 h-3.5 w-3.5 shrink-0"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" />
          </svg>
          {searchOpen ? (
            <>
              <input
                ref={inputRef}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="搜索剪切板…"
                className="w-full min-w-0 bg-transparent text-[12px] font-semibold text-neutral-100 outline-none placeholder:text-neutral-400"
                spellCheck={false}
              />
              {query && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    setQuery("");
                    inputRef.current?.focus();
                  }}
                  className="mr-3 shrink-0 cursor-pointer text-neutral-400 transition-colors hover:text-neutral-200"
                  aria-label="清除搜索"
                >
                  <svg
                    className="h-3.5 w-3.5"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                  >
                    <path d="M18 6 6 18" />
                    <path d="m6 6 12 12" />
                  </svg>
                </button>
              )}
            </>
          ) : (
            <span className="whitespace-nowrap pr-3.5 text-[12px] font-semibold">搜索</span>
          )}
        </div>
        <button
          type="button"
          onClick={() => {
            setTab("history");
            closeSearch();
            setActive(0);
          }}
          className={`flex cursor-pointer items-center gap-1.5 rounded-full border px-3.5 py-1 text-[12px] font-semibold transition-colors ${
            tab === "history"
              ? "border-[rgba(96,165,250,0.7)] bg-[rgba(59,130,246,0.15)] text-blue-200"
              : "border-[rgba(255,255,255,0.1)] bg-[rgba(255,255,255,0.06)] text-neutral-300 hover:bg-[rgba(255,255,255,0.12)]"
          }`}
        >
          <svg
            className="h-3.5 w-3.5"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="12" cy="12" r="10" />
            <polyline points="12 6 12 12 16 14" />
          </svg>
          历史剪切板
        </button>
        <button
          type="button"
          onClick={() => {
            setTab("pinned");
            closeSearch();
            setActive(0);
          }}
          className={`flex cursor-pointer items-center gap-1.5 rounded-full border px-3.5 py-1 text-[12px] font-semibold transition-colors ${
            tab === "pinned"
              ? "border-[rgba(96,165,250,0.7)] bg-[rgba(59,130,246,0.15)] text-blue-200"
              : "border-[rgba(255,255,255,0.1)] bg-[rgba(255,255,255,0.06)] text-neutral-300 hover:bg-[rgba(255,255,255,0.12)]"
          }`}
        >
          <svg
            className="h-3.5 w-3.5"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M12 17v5" />
            <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1z" />
          </svg>
          固定
        </button>
      </div>

      <div className="flex min-h-0 flex-1">
        {visible.length === 0 ? (
          <div className="flex flex-1 flex-col items-center justify-center gap-2 py-10 text-center">
            <svg
              className="h-10 w-10 text-neutral-500"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              {tab === "pinned" ? (
                <>
                  <path d="M12 17v5" />
                  <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1z" />
                </>
              ) : (
                <>
                  <rect x="8" y="2" width="8" height="4" rx="1" />
                  <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
                </>
              )}
            </svg>
            <p className="text-sm font-medium text-neutral-400">
              {query.trim()
                ? "No matches"
                : tab === "pinned"
                  ? "Nothing pinned yet"
                  : "Nothing copied yet"}
            </p>
            <p className="text-xs text-neutral-500">
              {query.trim()
                ? "Try a different search"
                : tab === "pinned"
                  ? "Hover a clip and click the pin"
                  : "Copy anything and it will appear here"}
            </p>
          </div>
        ) : (
          <div
            ref={listRef}
            className="card-scroll flex min-w-0 flex-1 items-center gap-[25px] overflow-x-auto px-5 py-1.5"
            role="listbox"
            aria-label={tab === "pinned" ? "Pinned clips" : "Clipboard history"}
          >
            {visible.map((clip, i) => (
              <ClipCard
                key={clip.id}
                clip={clip}
                index={i}
                isActive={i === active}
                showIndex={tab === "history"}
                metaDown={metaDown}
                handlers={{
                  onActivate: () => setActive(i),
                  onPaste: () => fadeOut(() => void invoke("paste_clip", { id: clip.id })),
                  onContext: openMenu,
                  onTogglePin: () => void invoke("toggle_pin", { id: clip.id }),
                }}
              />
            ))}
          </div>
        )}
      </div>

      {menu && (
        <div
          className="fixed z-50 w-[130px] overflow-hidden rounded-lg border border-[rgba(255,255,255,0.12)] bg-[rgba(24,24,27,0.98)] py-1 text-[12px] shadow-2xl"
          style={{ left: menu.x, top: menu.y }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <button
            type="button"
            onClick={() => {
              void invoke("toggle_pin", { id: menu.id });
              setMenu(null);
            }}
            className="flex w-full cursor-pointer items-center gap-2 px-3 py-1.5 text-left text-neutral-200 transition-colors hover:bg-[rgba(96,165,250,0.2)]"
          >
            <svg
              className="h-3.5 w-3.5"
              viewBox="0 0 24 24"
              fill={menu.pinned ? "currentColor" : "none"}
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <path d="M12 17v5" />
              <path d="M6 13h12l-1.5-5H7.5L6 13Z" />
              <path d="M9 8V4h6v4" />
            </svg>
            {menu.pinned ? "取消固定" : "固定"}
          </button>
        </div>
      )}

      <div className="flex items-center justify-between px-5 py-1.5 text-[10px] font-medium text-neutral-400">
        <span>← → Navigate</span>
        <div className="flex items-center gap-3">
          <span>单击选择 · 双击粘贴</span>
          <span className="tabular-nums" title="debug">⌘:{metaDown ? "ON" : "off"}</span>
          <span>
            <kbd className="rounded border border-[rgba(255,255,255,0.1)] px-1.5 py-0.5 font-sans text-neutral-400">
              ↵
            </kbd>{" "}
            Paste
          </span>
          <span>
            <kbd className="rounded border border-[rgba(255,255,255,0.1)] px-1.5 py-0.5 font-sans text-neutral-400">
              ⌘⇧V
            </kbd>{" "}
            Toggle
          </span>
        </div>
      </div>
    </div>
  );
}
