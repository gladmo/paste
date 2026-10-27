import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { Clip, PanelView, Pinboard } from "../types";

interface PanelState {
  view: PanelView;
  boardId: number | null;
  query: string;
  searchOpen: boolean;
  clips: Clip[];
  boards: Pinboard[];
  /** Index of the active (highlighted) card. */
  active: number;
  metaDown: boolean;
  /** Multi-selection mode (context menu "Select"). */
  selecting: boolean;
  selected: number[];
  /** 0 = hidden, 1 = shown; drives the fade animation. */
  anim: number;

  setView: (view: PanelView, boardId?: number | null) => void;
  setQuery: (q: string) => void;
  setSearchOpen: (open: boolean) => void;
  setActive: (i: number) => void;
  setMetaDown: (down: boolean) => void;
  setAnim: (a: number) => void;
  startSelecting: () => void;
  stopSelecting: () => void;
  toggleSelected: (id: number) => void;
  selectAll: () => void;
  clearSelection: () => void;
  refresh: () => Promise<void>;
  refreshBoards: () => Promise<void>;
}

/** Window-global event unsubs (registered once by Overlay). */
const listeners: Array<() => void> = [];

export function initPanelListeners(): () => void {
  void listen("clips:updated", () => {
    void usePanel.getState().refresh();
  });
  void listen("pinboards:updated", () => {
    void usePanel.getState().refreshBoards();
  });
  void listen("overlay:hide", () => {
    usePanel.setState({ anim: 0, metaDown: false });
  });
  return () => {
    for (const off of listeners.splice(0)) off();
  };
}

export const usePanel = create<PanelState>((set, get) => ({
  view: "history",
  boardId: null,
  query: "",
  searchOpen: false,
  clips: [],
  boards: [],
  active: 0,
  metaDown: false,
  selecting: false,
  selected: [],
  anim: 0,

  setView: (view, boardId = null) => {
    set({ view, boardId, active: 0, query: "", searchOpen: false });
    void get().refresh();
  },
  setQuery: (q) => set({ query: q, active: 0 }),
  setSearchOpen: (open) => set({ searchOpen: open, query: "", active: 0 }),
  setActive: (i) => set({ active: i }),
  setMetaDown: (down) => set({ metaDown: down }),
  setAnim: (a) => set({ anim: a }),
  startSelecting: () => set({ selecting: true, selected: [] }),
  stopSelecting: () => set({ selecting: false, selected: [] }),
  toggleSelected: (id) => {
    const { selected } = get();
    set({
      selecting: true,
      selected: selected.includes(id)
        ? selected.filter((x) => x !== id)
        : [...selected, id],
    });
  },
  selectAll: () => set({ selecting: true, selected: get().clips.map((c) => c.id) }),
  clearSelection: () => set({ selected: [] }),

  refresh: async () => {
    const { view, boardId, query } = get();
    const t0 = performance.now();
    try {
      const rows = await invoke<Clip[]>("view_clips", {
        view,
        pinboardId: view === "board" ? boardId : null,
        query: query.trim() ? query.trim() : null,
        limit: 200,
      });
      set({ clips: rows, active: 0 });
      // The panel reveals via overlay_ready as soon as this resolves; a load
      // slower than the 250ms Rust reveal fallback shows up as "panel opens
      // empty, cards pop in late" — make that state visible in /tmp/paste.log.
      const ms = performance.now() - t0;
      if (ms > 250) {
        void invoke("debug_log", {
          line: `[web] view_clips slow: ${rows.length} rows in ${ms.toFixed(0)}ms`,
        }).catch(() => {});
      }
    } catch (e) {
      console.error("view_clips failed", e);
      void invoke("debug_log", {
        line: `[web] view_clips FAILED after ${(performance.now() - t0).toFixed(0)}ms: ${String(e).slice(0, 300)}`,
      }).catch(() => {});
    }
  },

  refreshBoards: async () => {
    try {
      const boards = await invoke<Pinboard[]>("list_pinboards");
      set({ boards });
    } catch (e) {
      console.error("list_pinboards failed", e);
    }
  },
}));
