import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { useT } from "../i18n";

/** Create Text Item window (create-item.* copy). */
export function CreateApp() {
  const t = useT();
  const [text, setText] = useState("");
  const areaRef = useRef<HTMLTextAreaElement>(null);

  async function create() {
    if (!text.trim()) return;
    await invoke("create_text_clip", { text }).catch(console.error);
    await getCurrentWindow().close();
  }

  return (
    <div className="flex h-full flex-col bg-[#2a2a2c] p-3 text-neutral-100">
      <p className="mb-2 text-[12px] font-semibold text-neutral-300">{t("general.new-text-item")}</p>
      <textarea
        ref={areaRef}
        autoFocus
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) void create();
          if (e.key === "Escape") void getCurrentWindow().close();
        }}
        placeholder={t("general.text")}
        spellCheck={false}
        className="min-h-0 flex-1 resize-none rounded-[6px] bg-[rgba(255,255,255,0.05)] p-3 text-[13px] leading-relaxed outline-none ring-1 ring-[rgba(255,255,255,0.1)] focus:ring-[#0077fc]"
      />
      <div className="mt-2.5 flex justify-end gap-2">
        <button
          type="button"
          onClick={() => void getCurrentWindow().close()}
          className="cursor-pointer rounded-[6px] border border-[rgba(255,255,255,0.14)] bg-[rgba(255,255,255,0.08)] px-3.5 py-1 text-[12px] text-neutral-100 hover:bg-[rgba(255,255,255,0.14)]"
        >
          {t("create-item.action-tooltip.cancel-creation")}
        </button>
        <button
          type="button"
          onClick={() => void create()}
          disabled={!text.trim()}
          className="cursor-pointer rounded-[6px] bg-[#0a84ff] px-3.5 py-1 text-[12px] font-semibold text-white hover:bg-[#2a95ff] disabled:opacity-40"
        >
          {t("create-item.action.create")}
        </button>
      </div>
    </div>
  );
}
