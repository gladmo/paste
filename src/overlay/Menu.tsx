import { useEffect, useRef, useState } from "react";

export interface MenuItem {
  key: string;
  label?: string;
  onClick?: () => void;
  disabled?: boolean;
  danger?: boolean;
  separator?: boolean;
  submenu?: MenuItem[];
  /** Optional color dot shown before the label (pinboard colors). */
  swatch?: string;
}

function MenuList({ items, onClose }: { items: MenuItem[]; onClose: () => void }) {
  const [openSub, setOpenSub] = useState<string | null>(null);
  return (
    <>
      {items.map((item) =>
        item.separator ? (
          <div key={item.key} className="my-1 h-px bg-[rgba(255,255,255,0.12)]" />
        ) : (
          <div
            key={item.key}
            className="relative"
            onMouseEnter={() => setOpenSub(item.submenu ? item.key : null)}
          >
            <button
              type="button"
              disabled={item.disabled}
              onClick={() => {
                if (item.submenu) return;
                item.onClick?.();
                onClose();
              }}
              className={`flex w-full items-center gap-2 px-3 py-[3px] text-left text-[13px] ${
                item.disabled
                  ? "cursor-default text-neutral-500"
                  : item.danger
                    ? "cursor-pointer text-[#ff6961] hover:bg-[#0077fc] hover:text-white"
                    : "cursor-pointer text-neutral-100 hover:bg-[#0077fc] hover:text-white"
              }`}
            >
              <span
                className="h-[8px] w-[8px] shrink-0 rounded-full"
                style={{ backgroundColor: item.swatch ?? "transparent" }}
              />
              <span className="min-w-0 flex-1 truncate">{item.label}</span>
              {item.submenu && <span className="shrink-0 text-[10px] opacity-70">▸</span>}
            </button>
            {item.submenu && openSub === item.key && (
              <div
                className="absolute left-full top-0 z-50 ml-0.5 min-w-[180px] rounded-[8px] border border-[rgba(255,255,255,0.12)] py-1 shadow-2xl"
                style={{ backgroundColor: "rgba(42,43,46,0.97)", backdropFilter: "blur(20px)" }}
              >
                <MenuList items={item.submenu} onClose={onClose} />
              </div>
            )}
          </div>
        ),
      )}
    </>
  );
}

/** macOS-style context menu with nested submenus. */
export function Menu({
  x,
  y,
  items,
  onClose,
}: {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x, y });

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    let nx = x;
    let ny = y;
    if (x + r.width > window.innerWidth - 8) nx = Math.max(8, window.innerWidth - r.width - 8);
    if (y + r.height > window.innerHeight - 8) ny = Math.max(8, window.innerHeight - r.height - 8);
    setPos({ x: nx, y: ny });
  }, [x, y]);

  return (
    <div
      ref={ref}
      className="fixed z-50 min-w-[210px] rounded-[8px] border border-[rgba(255,255,255,0.12)] py-1 shadow-2xl"
      style={{ left: pos.x, top: pos.y, backgroundColor: "rgba(42,43,46,0.97)", backdropFilter: "blur(20px)" }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <MenuList items={items} onClose={onClose} />
    </div>
  );
}

/** Small centered alert dialog (original alert.* copy). */
export function ConfirmDialog({
  title,
  text,
  confirmLabel,
  cancelLabel,
  danger,
  onConfirm,
  onCancel,
}: {
  title: string;
  text?: string;
  confirmLabel: string;
  cancelLabel: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="absolute inset-0 z-[60] flex items-center justify-center bg-[rgba(0,0,0,0.35)]" onMouseDown={(e) => e.stopPropagation()}>
      <div
        className="w-[320px] rounded-[12px] border border-[rgba(255,255,255,0.12)] p-4 shadow-2xl"
        style={{ backgroundColor: "rgba(42,43,46,0.98)" }}
      >
        <p className="text-center text-[13px] font-semibold text-neutral-100">{title}</p>
        {text && <p className="mt-1.5 text-center text-[12px] leading-relaxed text-neutral-300">{text}</p>}
        <div className="mt-4 flex justify-center gap-2">
          <button
            type="button"
            onClick={onCancel}
            className="min-w-[96px] cursor-pointer rounded-[6px] border border-[rgba(255,255,255,0.14)] bg-[rgba(255,255,255,0.08)] py-1 text-[12px] font-medium text-neutral-100 hover:bg-[rgba(255,255,255,0.14)]"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            onClick={onConfirm}
            className={`min-w-[96px] cursor-pointer rounded-[6px] py-1 text-[12px] font-semibold text-white ${
              danger ? "bg-[#d5473f] hover:bg-[#e05a52]" : "bg-[#0a84ff] hover:bg-[#2a95ff]"
            }`}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
