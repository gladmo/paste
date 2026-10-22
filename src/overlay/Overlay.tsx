import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { listen } from "@tauri-apps/api/event";

import { applyLanguageSetting, useT } from "../i18n";
import { THEME } from "../ui/theme";
import { packedToCss } from "../types";
import type { Clip, PanelView, Pinboard } from "../types";
import { initPanelListeners, usePanel } from "../stores/panel";
import { useSettings } from "../stores/settings";
import { ClipCard } from "./ClipCard";
import { ConfirmDialog, Menu } from "./Menu";
import type { MenuItem } from "./Menu";
import { SearchBar } from "./SearchBar";
import { TabBar } from "./TabBar";

/** Configured-modifier check ("cmd" | "ctrl" | "alt" | "shift"). */
function modActive(e: KeyboardEvent, mod: string): boolean {
  switch (mod) {
    case "ctrl":
      return e.ctrlKey;
    case "alt":
      return e.altKey;
    case "shift":
      return e.shiftKey;
    case "cmd":
    default:
      return e.metaKey;
  }
}

const MOD_KEY_NAMES: Record<string, string[]> = {
  cmd: ["Meta", "MetaLeft", "MetaRight"],
  ctrl: ["Control", "ControlLeft", "ControlRight"],
  alt: ["Alt", "AltLeft", "AltRight"],
  shift: ["Shift", "ShiftLeft", "ShiftRight"],
};

/** Fade the panel out, then run the action (hide / paste). */
function useFadeOut() {
  const closingRef = useRef(false);
  return useCallback((then: () => void) => {
    if (closingRef.current) return;
    closingRef.current = true;
    usePanel.setState({ anim: 0 });
    setTimeout(() => {
      closingRef.current = false;
      then();
    }, 160);
  }, []);
}

function EmptyState({ view, query }: { view: PanelView; query: string }) {
  const t = useT();
  const label = query.trim()
    ? t("general.nothing-found")
    : view === "history"
      ? t("general.history-empty")
      : t("general.pinboard-empty");
  return (
    <div className="flex h-full w-full flex-col items-center justify-center gap-3 text-neutral-500">
      <svg className="h-10 w-10 opacity-40" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round">
        <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
        <rect x="8" y="2" width="8" height="4" rx="1" />
      </svg>
      <p className="text-[13px] font-medium">{label}</p>
    </div>
  );
}

/** Selection toolbar replacing the normal toolbar while selecting. */
function SelectionBar() {
  const t = useT();
  const selected = usePanel((s) => s.selected);
  const clips = usePanel((s) => s.clips);
  const selectAll = usePanel((s) => s.selectAll);
  const clearSelection = usePanel((s) => s.clearSelection);
  const stopSelecting = usePanel((s) => s.stopSelecting);
  return (
    <div className="flex h-[60px] shrink-0 items-center justify-center gap-3 px-6 text-[12px] font-medium text-neutral-200">
      <span className="text-neutral-400">{t("general.select-items")}</span>
      {selected.length > 0 && (
        <span className="rounded-full bg-[rgba(10,132,255,0.22)] px-2 py-0.5 text-[11px] font-semibold text-[#7cb8ff]">
          {t("general.selected-format", selected.length)}
        </span>
      )}
      <span className="h-4 w-px bg-[rgba(255,255,255,0.14)]" />
      <button type="button" onClick={selectAll} className="cursor-pointer text-neutral-300 hover:text-white">
        {t("general.action.select-all")}
      </button>
      <button
        type="button"
        onClick={clearSelection}
        disabled={!selected.length}
        className="cursor-pointer text-neutral-300 hover:text-white disabled:opacity-40"
      >
        {t("general.action.deselect-all")}
      </button>
      <span className="h-4 w-px bg-[rgba(255,255,255,0.14)]" />
      <span className="text-[11px] text-neutral-500">
        {clips.length} {t("general.clipboard-history").toLowerCase()}
      </span>
      <button
        type="button"
        onClick={stopSelecting}
        className="cursor-pointer rounded-full bg-[rgba(255,255,255,0.1)] px-3 py-0.5 text-neutral-100 hover:bg-[rgba(255,255,255,0.16)]"
      >
        {t("general.action.done")}
      </button>
    </div>
  );
}

export function Overlay() {
  // Dev-only: the hidden panel's WebView misses HMR updates (page suspended),
  // so every summon reloads to guarantee fresh code.
  const reloadedOnShow =
    typeof sessionStorage !== "undefined" && sessionStorage.getItem("paste:dev-reloaded") === "1";
  if (reloadedOnShow) sessionStorage.removeItem("paste:dev-reloaded");

  const t = useT();
  const fadeOut = useFadeOut();
  const inputRef = useRef<HTMLInputElement | null>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const [canScroll, setCanScroll] = useState({ left: false, right: false });

  const anim = usePanel((s) => s.anim);
  const query = usePanel((s) => s.query);
  const view = usePanel((s) => s.view);
  const boardId = usePanel((s) => s.boardId);
  const clips = usePanel((s) => s.clips);
  const boards = usePanel((s) => s.boards);
  const active = usePanel((s) => s.active);
  const metaDown = usePanel((s) => s.metaDown);
  const selecting = usePanel((s) => s.selecting);
  const selected = usePanel((s) => s.selected);
  const setActive = usePanel((s) => s.setActive);
  const setMetaDown = usePanel((s) => s.setMetaDown);

  const [clipMenu, setClipMenu] = useState<{ x: number; y: number; clip: Clip } | null>(null);
  const [toolbarMenu, setToolbarMenu] = useState<{ x: number; y: number } | null>(null);
  const [confirmDeleteSelected, setConfirmDeleteSelected] = useState(false);
  const [confirmErase, setConfirmErase] = useState(false);

  const visible = clips;

  // ---- data refresh -------------------------------------------------------
  useEffect(() => {
    void usePanel.getState().refreshBoards();
    void useSettings
      .getState()
      .load()
      .then(() => applyLanguageSetting(useSettings.getState().settings.language));
    const off = listen("settings:updated", () =>
      void useSettings
        .getState()
        .load()
        .then(() => applyLanguageSetting(useSettings.getState().settings.language)),
    );
    return () => {
      off.then((fn) => fn());
    };
  }, []);
  useEffect(() => {
    const timer = setTimeout(() => void usePanel.getState().refresh(), 120);
    return () => clearTimeout(timer);
  }, [query, view, boardId]);
  useEffect(() => initPanelListeners(), []);

  // overlay:show resets the whole panel state.
  useEffect(() => {
    const off = listen("overlay:show", () => {
      if (import.meta.env.DEV) {
        sessionStorage.setItem("paste:dev-reloaded", "1");
        window.location.reload();
        return;
      }
      usePanel.setState({
        query: "",
        searchOpen: false,
        view: "history",
        boardId: null,
        active: 0,
        metaDown: false,
        selecting: false,
        selected: [],
        anim: 0,
      });
      void usePanel.getState().refresh();
      void usePanel.getState().refreshBoards();
      if (listRef.current) listRef.current.scrollLeft = 0;
      requestAnimationFrame(() => requestAnimationFrame(() => usePanel.setState({ anim: 1 })));
    });
    const offErase = listen("menu:erase-history", () => {
      setConfirmErase(true);
    });
    return () => {
      off.then((fn) => fn());
      offErase.then((fn) => fn());
    };
  }, []);

  // Keep the active card in view.
  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-index="${active}"]`)
      ?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [active]);

  // Track scroll arrow availability.
  useEffect(() => {
    const el = listRef.current;
    if (!el) return;
    const update = () =>
      setCanScroll({
        left: el.scrollLeft > 8,
        right: el.scrollLeft + el.clientWidth < el.scrollWidth - 8,
      });
    update();
    el.addEventListener("scroll", update);
    window.addEventListener("resize", update);
    return () => {
      el.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
    };
  }, [visible.length]);

  // The configured quick-paste modifier drives the number badges.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = useSettings.getState().settings.quickPasteModifier;
      const names = MOD_KEY_NAMES[mod] ?? MOD_KEY_NAMES.cmd;
      if (names.includes(e.key)) {
        setMetaDown(e.type === "keydown");
      }
    };
    window.addEventListener("keydown", onKey);
    window.addEventListener("keyup", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("keyup", onKey);
    };
  }, [setMetaDown]);

  // Close floating menus on outside interaction / blur.
  useEffect(() => {
    const close = () => {
      setClipMenu(null);
      setToolbarMenu(null);
    };
    window.addEventListener("mousedown", close);
    window.addEventListener("blur", close);
    return () => {
      window.removeEventListener("mousedown", close);
      window.removeEventListener("blur", close);
    };
  }, []);

  const pasteClip = useCallback(
    (id: number, plain = false) => {
      fadeOut(() => void invoke("paste_clip", { id, plain }));
    },
    [fadeOut],
  );

  const switchViewDelta = useCallback(
    (delta: 1 | -1) => {
      const order: Array<{ view: PanelView; boardId: number | null }> = [
        { view: "history", boardId: null },
        { view: "pinned", boardId: null },
        ...boards
          .filter((b) => b.id !== 1)
          .map((b) => ({ view: "board" as PanelView, boardId: b.id })),
      ];
      const cur = order.findIndex(
        (o) => o.view === usePanel.getState().view && o.boardId === usePanel.getState().boardId,
      );
      const next = order[Math.max(0, Math.min(order.length - 1, cur + delta))];
      if (next) usePanel.getState().setView(next.view, next.boardId);
    },
    [boards],
  );

  // Global next/previous pinboard shortcuts and tray "Manage Pinboards".
  useEffect(() => {
    const offCycle = listen<number>("panel:cycle", (e) => switchViewDelta(e.payload >= 1 ? 1 : -1));
    const offManage = listen("panel:manage-pinboards", () => {
      usePanel.getState().setView("pinned");
    });
    return () => {
      offCycle.then((fn) => fn());
      offManage.then((fn) => fn());
    };
  }, [switchViewDelta]);

  // ---- keyboard -----------------------------------------------------------
  const quickMod = (e: KeyboardEvent) =>
    modActive(e, useSettings.getState().settings.quickPasteModifier);
  const plainMod = (e: KeyboardEvent) =>
    modActive(e, useSettings.getState().settings.plainTextModifier);
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const state = usePanel.getState();
      if (e.key === "Escape") {
        if (clipMenu) {
          setClipMenu(null);
        } else if (state.selecting) {
          state.stopSelecting();
        } else if (state.query) {
          state.setQuery("");
        } else if (state.searchOpen) {
          state.setSearchOpen(false);
        } else {
          fadeOut(() => void invoke("hide_overlay"));
        }
      } else if (e.key === "a" && e.metaKey) {
        // Select All / Deselect All
        e.preventDefault();
        if (state.selected.length === state.clips.length && state.selected.length > 0) {
          state.clearSelection();
        } else {
          state.selectAll();
        }
      } else if ((e.key === "Backspace" || e.key === "Delete") && state.selecting && state.selected.length) {
        e.preventDefault();
        setConfirmDeleteSelected(true);
      } else if (e.key >= "1" && e.key <= "9" && quickMod(e)) {
        e.preventDefault();
        const idx = Number(e.key) - 1;
        const clip = state.clips[idx];
        if (clip) pasteClip(clip.id, plainMod(e));
      } else if (e.key.length === 1 && !e.metaKey && !e.ctrlKey && !e.altKey) {
        // Type-to-search: focus the field and insert the character manually
        // (focusing during keydown can swallow the first keystroke in WKWebView).
        if (document.activeElement !== inputRef.current) {
          e.preventDefault();
          inputRef.current?.focus();
          usePanel.getState().setQuery(e.key);
        }
      } else if (e.key === "ArrowRight" || e.key === "ArrowDown") {
        if (e.altKey && e.metaKey) {
          e.preventDefault();
          switchViewDelta(1);
        } else {
          e.preventDefault();
          setActive(Math.min(state.active + 1, Math.max(state.clips.length - 1, 0)));
        }
      } else if (e.key === "ArrowLeft" || e.key === "ArrowUp") {
        if (e.altKey && e.metaKey) {
          e.preventDefault();
          switchViewDelta(-1);
        } else {
          e.preventDefault();
          setActive(Math.max(state.active - 1, 0));
        }
      } else if (e.key === "Enter") {
        const clip = state.clips[state.active];
        if (clip) pasteClip(clip.id, plainMod(e));
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [clipMenu, fadeOut, pasteClip, setActive, switchViewDelta]);

  // ---- clip context menu ---------------------------------------------------
  const clipMenuItems = useMemo(() => {
    if (!clipMenu) return [];
    const clip = clipMenu.clip;
    const boardsForSub: Pinboard[] = boards.filter((b) => b.id !== 1);
    const many = selected.length > 1 && selected.includes(clip.id);
    const items = [
      { key: "paste", label: t("general.action.paste"), onClick: () => pasteClip(clip.id) },
      {
        key: "paste-plain",
        label: t("general.action.paste-as-plain-text"),
        onClick: () => pasteClip(clip.id, true),
      },
      { key: "sep1", separator: true, label: "" },
      {
        key: "copy",
        label: t("general.action.copy"),
        onClick: () => void invoke("copy_clip", { id: clip.id }).catch(console.error),
      },
      {
        key: "copy-plain",
        label: t("general.action.copy-as-plain-text"),
        onClick: () => void invoke("copy_clip", { id: clip.id, plain: true }).catch(console.error),
      },
      ...(clip.contentType === "link"
        ? [
            {
              key: "open",
              label: t("replica.menu.open-link"),
              onClick: () => openUrl(clip.content).catch(console.error),
            },
          ]
        : []),
      ...(clip.contentType === "file"
        ? [
            {
              key: "reveal",
              label: t("preview.reveal-in-finder.title"),
              onClick: () => void invoke("reveal_in_finder", { path: clip.content }).catch(console.error),
            },
          ]
        : []),
      { key: "sep2", separator: true, label: "" },
      {
        key: "preview",
        label: t("general.action.preview"),
        onClick: () => void invoke("open_preview", { id: clip.id }).catch(console.error),
      },
      {
        key: "pin",
        label: clip.pinned ? t("general.action.unpin") : t("general.action.pin"),
        onClick: () => void invoke("toggle_pin", { id: clip.id }).catch(console.error),
      },
      {
        key: "pinboard",
        label: t("general.action.select-pinboard"),
        submenu: boardsForSub.map((b) => ({
          key: `board-${b.id}`,
          label: b.name,
          swatch: packedToCss(b.color),
          onClick: () =>
            void invoke("move_clip_to_pinboard", { id: clip.id, pinboardId: b.id }).catch(console.error),
        })),
      },
      { key: "sep3", separator: true, label: "" },
      {
        key: "select",
        label: t("general.action.select"),
        onClick: () => {
          usePanel.getState().startSelecting();
          usePanel.getState().toggleSelected(clip.id);
        },
      },
      { key: "select-all", label: t("general.action.select-all"), onClick: () => usePanel.getState().selectAll() },
      { key: "sep4", separator: true, label: "" },
      {
        key: "delete",
        label: many
          ? `${t("general.action.delete")} (${selected.length})`
          : t("general.action.delete"),
        danger: true,
        onClick: () => {
          if (many) {
            setConfirmDeleteSelected(true);
          } else {
            void invoke("delete_clip", { id: clip.id }).catch(console.error);
          }
        },
      },
    ];
    return items;
  }, [clipMenu, boards, selected, pasteClip, t]);

  const scrollBy = (dir: 1 | -1) => {
    listRef.current?.scrollBy({ left: dir * 500, behavior: "smooth" });
  };

  return (
    <div
      className={`relative flex h-full flex-col overflow-hidden transition-all duration-150 ease-out ${
        anim ? "translate-y-0 opacity-100" : "translate-y-4 opacity-0"
      }`}
      style={{
        backgroundColor: THEME.panel.background,
      }}
    >
      {selecting ? (
        <SelectionBar />
      ) : (
        <div
          className="relative flex h-[60px] shrink-0 items-center justify-center px-10"
          style={{ gap: THEME.toolbar.gap }}
          onContextMenu={(e) => {
            // Panel-level menu: Erase History (tray menu lands in P5).
            e.preventDefault();
            setToolbarMenu({ x: e.clientX, y: e.clientY });
          }}
        >
          <SearchBar focusRef={inputRef} />
          <TabBar />

          {/* "⋯" panel menu button (far right, measured 22.5pt margin) */}
          <button
            type="button"
            aria-label={t("replica.menu.more")}
            title={t("replica.menu.more")}
            onClick={(e) => {
              const r = e.currentTarget.getBoundingClientRect();
              setToolbarMenu({ x: r.left - 180, y: r.bottom + 6 });
            }}
            className="absolute top-1/2 flex h-[32px] w-[28px] -translate-y-1/2 cursor-pointer items-center justify-center rounded-[6px] transition-colors hover:bg-[rgba(255,255,255,0.08)]"
            style={{ right: THEME.toolbar.dotsRight, color: THEME.glyphs.gray }}
          >
            <svg width="16" height="4" viewBox="0 0 16 4" fill="currentColor">
              <circle cx="2" cy="2" r="1.6" />
              <circle cx="8" cy="2" r="1.6" />
              <circle cx="14" cy="2" r="1.6" />
            </svg>
          </button>
        </div>
      )}

      <div className="relative min-h-0 flex-1">
        {visible.length === 0 ? (
          <EmptyState view={view} query={query} />
        ) : (
          <>
            <div
              ref={listRef}
              className="group/strip flex h-full items-start overflow-x-auto overflow-y-hidden"
              style={{
                scrollbarWidth: "none",
                paddingLeft: THEME.strip.paddingLeft,
                paddingTop: THEME.strip.paddingTop,
                paddingBottom: THEME.strip.paddingBottom,
                gap: THEME.card.gap,
              }}
              onScroll={(e) => {
                const el = e.currentTarget;
                setCanScroll({
                  left: el.scrollLeft > 8,
                  right: el.scrollLeft + el.clientWidth < el.scrollWidth - 8,
                });
              }}
            >
              {visible.map((clip, i) => (
                <ClipCard
                  key={clip.id}
                  clip={clip}
                  index={i}
                  isActive={i === active}
                  showIndex
                  metaDown={metaDown}
                  selecting={selecting}
                  selected={selected.includes(clip.id)}
                  handlers={{
                    onActivate: () => setActive(i),
                    onOpen: () => pasteClip(clip.id),
                    onContext: (e) => {
                      e.preventDefault();
                      if (!selecting) setActive(i);
                      setClipMenu({ x: e.clientX, y: e.clientY, clip });
                    },
                    onTogglePin: () => void invoke("toggle_pin", { id: clip.id }).catch(console.error),
                    onSelectionClick: () => usePanel.getState().toggleSelected(clip.id),
                  }}
                />
              ))}
            </div>

            {/* edge scroll arrows */}
            {canScroll.left && (
              <button
                type="button"
                aria-label="scroll left"
                onClick={() => scrollBy(-1)}
                className="absolute left-1 top-1/2 z-10 flex h-10 w-7 -translate-y-1/2 cursor-pointer items-center justify-center rounded-md bg-[rgba(20,20,20,0.55)] text-white/70 opacity-0 backdrop-blur transition-opacity hover:text-white group-hover/strip:opacity-100"
              >
                <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
                  <path d="m15 18-6-6 6-6" />
                </svg>
              </button>
            )}
            {canScroll.right && (
              <button
                type="button"
                aria-label="scroll right"
                onClick={() => scrollBy(1)}
                className="absolute right-1 top-1/2 z-10 flex h-10 w-7 -translate-y-1/2 cursor-pointer items-center justify-center rounded-md bg-[rgba(20,20,20,0.55)] text-white/70 opacity-0 backdrop-blur transition-opacity hover:text-white group-hover/strip:opacity-100"
              >
                <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
                  <path d="m9 18 6-6-6-6" />
                </svg>
              </button>
            )}
          </>
        )}
      </div>

      {/* clip context menu */}
      {clipMenu && (
        <Menu x={clipMenu.x} y={clipMenu.y} items={clipMenuItems} onClose={() => setClipMenu(null)} />
      )}

      {/* toolbar context menu (create item / erase history) */}
      {toolbarMenu && (
        <Menu
          x={toolbarMenu.x}
          y={toolbarMenu.y}
          items={
            [
              {
                key: "create",
                label: t("general.new-text-item"),
                onClick: () => void invoke("open_create").catch(console.error),
              },
              {
                key: "manage",
                label: t("general.action.manage-pinboards"),
                onClick: () => usePanel.getState().setView("pinned"),
              },
              { key: "sep", separator: true, label: "" },
              {
                key: "erase",
                label: `${t("general.action.erase-history")}…`,
                danger: true,
                onClick: () => setConfirmErase(true),
              },
              {
                key: "settings",
                label: `${t("general.settings")}…`,
                onClick: () => void invoke("open_settings").catch(console.error),
              },
            ] satisfies MenuItem[]
          }
          onClose={() => setToolbarMenu(null)}
        />
      )}

      {/* delete selected confirmation */}
      {confirmDeleteSelected && (
        <ConfirmDialog
          title={t("alert.delete-selected-items.title")}
          confirmLabel={`${t("general.action.delete")} (${selected.length})`}
          cancelLabel={t("general.action.cancel")}
          danger
          onConfirm={async () => {
            setConfirmDeleteSelected(false);
            await invoke("delete_clips", { ids: usePanel.getState().selected }).catch(console.error);
            usePanel.getState().stopSelecting();
          }}
          onCancel={() => setConfirmDeleteSelected(false)}
        />
      )}

      {/* erase history confirmation (reachable from the toolbar context
          menu until the tray menu lands in P5) */}
      {confirmErase && (
        <ConfirmDialog
          title={t("alert.erase-history.title")}
          text={t("alert.erase-history.text")}
          confirmLabel={t("general.action.erase")}
          cancelLabel={t("general.action.cancel")}
          danger
          onConfirm={async () => {
            setConfirmErase(false);
            await invoke("erase_history").catch(console.error);
          }}
          onCancel={() => setConfirmErase(false)}
        />
      )}
    </div>
  );
}
