import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

import { useT } from "../i18n";
import { PINBOARD_COLORS, THEME } from "../ui/theme";
import { BUILTIN_PINNED_ID, packedToCss } from "../types";
import type { Pinboard } from "../types";
import { usePanel } from "../stores/panel";
import { ConfirmDialog } from "./Menu";

function ClockIcon({ size = 14 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="10" />
      <polyline points="12 6 12 12 16 14" />
    </svg>
  );
}

function PinIcon({ size = 13 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 17v5" />
      <path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V6h1a2 2 0 0 0 0-4H8a2 2 0 0 0 0 4h1z" />
    </svg>
  );
}

/** One toolbar tab: colored icon + label (no pill background, like the
 *  original; active tab text is brighter). */
function Tab({
  label,
  active,
  onClick,
  onContext,
  onDouble,
  icon,
  color,
  editingLabel,
  onRename,
  onCancelRename,
}: {
  label: string;
  active: boolean;
  onClick: () => void;
  onContext?: (e: React.MouseEvent) => void;
  onDouble?: () => void;
  icon?: React.ReactNode;
  color?: string;
  editingLabel?: string | null;
  onRename?: (name: string) => void;
  onCancelRename?: () => void;
}) {
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (editingLabel != null) inputRef.current?.select();
  }, [editingLabel]);

  return (
    <button
      type="button"
      onClick={onClick}
      onContextMenu={onContext}
      onDoubleClick={onDouble}
      className={`flex shrink-0 cursor-pointer items-center gap-1.5 px-2 py-0.5 text-[12.5px] font-medium transition-colors ${
        active ? "" : "hover:opacity-90"
      }`}
      style={{ color: active ? THEME.tab.textActive : THEME.tab.textInactive }}
    >
      {icon ?? (
        <span
          className="flex h-[13px] w-[13px] shrink-0 items-center justify-center rounded-[3px]"
          style={{ color }}
        >
          <PinIcon />
        </span>
      )}
      {editingLabel != null ? (
        <input
          ref={inputRef}
          defaultValue={editingLabel}
          autoFocus
          onBlur={(e) => onRename?.(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") onRename?.((e.target as HTMLInputElement).value);
            if (e.key === "Escape") onCancelRename?.();
            e.stopPropagation();
          }}
          onContextMenu={(e) => e.stopPropagation()}
          className="w-[90px] rounded-[4px] bg-[rgba(255,255,255,0.1)] px-1 text-[12px] text-neutral-100 outline-none ring-1 ring-[#0077fc]"
          onClick={(e) => e.stopPropagation()}
        />
      ) : (
        <span className="max-w-[140px] truncate">{label}</span>
      )}
    </button>
  );
}

/** History | Pinned | user pinboards toolbar with inline create popover. */
export function TabBar() {
  const t = useT();
  const view = usePanel((s) => s.view);
  const boardId = usePanel((s) => s.boardId);
  const boards = usePanel((s) => s.boards);
  const setView = usePanel((s) => s.setView);
  const refreshBoards = usePanel((s) => s.refreshBoards);

  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newColor, setNewColor] = useState<number>(PINBOARD_COLORS[1]);
  const [renaming, setRenaming] = useState<number | null>(null);
  const [deleting, setDeleting] = useState<Pinboard | null>(null);
  const [boardMenu, setBoardMenu] = useState<{ x: number; y: number; board: Pinboard } | null>(null);

  async function create() {
    const name = newName.trim();
    if (!name) return;
    await invoke("create_pinboard", { name, color: newColor }).catch(console.error);
    setCreating(false);
    setNewName("");
    await refreshBoards();
  }

  return (
    <>
      <div className="flex min-w-0 items-center gap-1 overflow-x-auto">
        <Tab
          label={t("replica.tab.history")}
          active={view === "history"}
          onClick={() => setView("history")}
          icon={
            <span className="flex h-[13px] w-[13px] shrink-0 items-center justify-center">
              <ClockIcon />
            </span>
          }
        />
        {boards.map((board) => {
          const isBuiltin = board.id === BUILTIN_PINNED_ID;
          const active = isBuiltin ? view === "pinned" : view === "board" && boardId === board.id;
          return (
            <Tab
              key={board.id}
              label={isBuiltin ? t("replica.tab.pinned") : board.name}
              active={active}
              color={packedToCss(board.color)}
              onClick={() => setView(isBuiltin ? "pinned" : "board", board.id)}
              onContext={(e) => {
                e.preventDefault();
                setBoardMenu({ x: e.clientX, y: e.clientY, board });
              }}
              onDouble={() => !isBuiltin && setRenaming(board.id)}
              editingLabel={renaming === board.id ? board.name : null}
              onRename={async (name) => {
                setRenaming(null);
                const trimmed = name.trim();
                if (trimmed && trimmed !== board.name) {
                  await invoke("rename_pinboard", { id: board.id, name: trimmed }).catch(console.error);
                  await refreshBoards();
                }
              }}
              onCancelRename={() => setRenaming(null)}
            />
          );
        })}
      </div>

      {/* "+" create pinboard button (far right of the tab cluster) */}
      <div className="relative shrink-0">
        <button
          type="button"
          title={t("general.action.create-pinboard")}
          aria-label={t("general.action.create-pinboard")}
          onClick={() => setCreating((v) => !v)}
          className="flex h-[22px] w-[22px] cursor-pointer items-center justify-center rounded-full text-neutral-300 transition-colors hover:bg-[rgba(255,255,255,0.1)] hover:text-white"
        >
          <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round">
            <path d="M12 5v14" />
            <path d="M5 12h14" />
          </svg>
        </button>
        {creating && (
          <div
            className="absolute right-0 top-8 z-50 w-[228px] rounded-[10px] border border-[rgba(255,255,255,0.12)] p-3 shadow-2xl"
            style={{ backgroundColor: "rgba(42,43,46,0.98)", backdropFilter: "blur(20px)" }}
            onMouseDown={(e) => e.stopPropagation()}
          >
            <p className="mb-2 text-[12px] font-semibold text-neutral-100">{t("replica.pinboard.new")}</p>
            <input
              autoFocus
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void create();
                if (e.key === "Escape") setCreating(false);
              }}
              placeholder={t("replica.pinboard.name-placeholder")}
              className="mb-2.5 w-full rounded-[6px] bg-[rgba(255,255,255,0.08)] px-2 py-1 text-[12px] text-neutral-100 outline-none ring-1 ring-[rgba(255,255,255,0.12)] focus:ring-[#0077fc]"
            />
            <p className="mb-1 text-[10px] font-medium text-neutral-400">{t("replica.pinboard.pick-color")}</p>
            <div className="mb-3 flex gap-1.5">
              {PINBOARD_COLORS.map((c) => (
                <button
                  key={c}
                  type="button"
                  onClick={() => setNewColor(c)}
                  className="h-[16px] w-[16px] cursor-pointer rounded-full transition-transform hover:scale-110"
                  style={{
                    backgroundColor: packedToCss(c),
                    boxShadow: newColor === c ? "0 0 0 2px rgba(255,255,255,0.8)" : "none",
                  }}
                  aria-label={`color-${c}`}
                />
              ))}
            </div>
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={() => setCreating(false)}
                className="cursor-pointer rounded-[6px] border border-[rgba(255,255,255,0.14)] bg-[rgba(255,255,255,0.08)] px-3 py-1 text-[12px] text-neutral-100 hover:bg-[rgba(255,255,255,0.14)]"
              >
                {t("general.action.cancel")}
              </button>
              <button
                type="button"
                onClick={() => void create()}
                disabled={!newName.trim()}
                className="cursor-pointer rounded-[6px] bg-[#0a84ff] px-3 py-1 text-[12px] font-semibold text-white hover:bg-[#2a95ff] disabled:opacity-40"
              >
                {t("general.action.create")}
              </button>
            </div>
          </div>
        )}
      </div>

      {/* pinboard context menu (rename / color / delete) */}
      {boardMenu && (
        <div
          className="fixed z-50 min-w-[170px] rounded-[8px] border border-[rgba(255,255,255,0.12)] py-1 text-[13px] shadow-2xl"
          style={{ left: boardMenu.x, top: boardMenu.y, backgroundColor: "rgba(42,43,46,0.97)", backdropFilter: "blur(20px)" }}
          onMouseDown={(e) => e.stopPropagation()}
        >
          <button
            type="button"
            onClick={() => {
              setRenaming(boardMenu.board.id);
              setBoardMenu(null);
            }}
            className="flex w-full cursor-pointer px-3 py-[3px] text-left text-neutral-100 hover:bg-[#0077fc] hover:text-white"
          >
            {t("general.action.rename")}
          </button>
          <div className="flex items-center gap-1.5 px-3 py-1.5">
            {PINBOARD_COLORS.map((c) => (
              <button
                key={c}
                type="button"
                onClick={() => {
                  const b = boardMenu.board;
                  void invoke("set_pinboard_color", { id: b.id, color: c }).then(refreshBoards);
                  setBoardMenu(null);
                }}
                className="h-[13px] w-[13px] cursor-pointer rounded-full transition-transform hover:scale-125"
                style={{ backgroundColor: packedToCss(c) }}
                aria-label={`color-${c}`}
              />
            ))}
          </div>
          <div className="my-1 h-px bg-[rgba(255,255,255,0.12)]" />
          <button
            type="button"
            onClick={() => {
              setDeleting(boardMenu.board);
              setBoardMenu(null);
            }}
            className="flex w-full cursor-pointer px-3 py-[3px] text-left text-[#ff6961] hover:bg-[#0077fc] hover:text-white"
          >
            {t("general.action.delete")}
          </button>
        </div>
      )}

      {deleting && (
        <ConfirmDialog
          title={t("alert.delete-pinboard.title", deleting.name)}
          text={t("alert.delete-pinboard.text")}
          confirmLabel={t("general.action.delete")}
          cancelLabel={t("general.action.cancel")}
          danger
          onConfirm={async () => {
            const b = deleting;
            setDeleting(null);
            await invoke("delete_pinboard", { id: b.id }).catch(console.error);
            if (usePanel.getState().boardId === b.id) setView("history");
            await refreshBoards();
            await usePanel.getState().refresh();
          }}
          onCancel={() => setDeleting(null)}
        />
      )}
    </>
  );
}
