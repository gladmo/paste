import { useCallback, useEffect, useState } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { useT } from "../i18n";
import type { Clip } from "../types";
import { TypeIcon } from "../overlay/ClipCard";
import { formatTime } from "../overlay/preview";

const STACK_H = 438;
const STACK_W = 264;

function RowPreview({ clip }: { clip: Clip }) {
  if (clip.contentType === "image") {
    return (
      <img
        src={convertFileSrc(clip.content)}
        alt=""
        className="h-[28px] w-[28px] shrink-0 rounded-[4px] object-cover"
        draggable={false}
      />
    );
  }
  const text =
    clip.contentType === "link"
      ? clip.title || clip.content
      : (clip.plainContent ?? clip.content).replace(/\s+/g, " ");
  return <span className="truncate text-[12px] text-neutral-200">{text}</span>;
}

/** Paste Stack panel (storyboard geometry: 264×438, 44pt rows, vibrancy). */
export function StackApp() {
  const t = useT();
  const [clips, setClips] = useState<Clip[]>([]);
  const [horizontal, setHorizontal] = useState(
    () => localStorage.getItem("paste:stack-direction") === "horizontal",
  );

  const refresh = useCallback(async () => {
    try {
      const rows = await invoke<Clip[]>("view_clips", {
        view: "history",
        pinboardId: null,
        query: null,
        limit: 15,
      });
      setClips(rows);
    } catch (e) {
      console.error("stack refresh failed", e);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const off1 = listen("clips:updated", () => void refresh());
    const off2 = listen("stack:show", () => void refresh());
    return () => {
      off1.then((fn) => fn());
      off2.then((fn) => fn());
    };
  }, [refresh]);

  // Direction toggle swaps the panel dimensions (docked right vs bottom).
  useEffect(() => {
    localStorage.setItem("paste:stack-direction", horizontal ? "horizontal" : "vertical");
    const w = horizontal ? STACK_H : STACK_W;
    const h = horizontal ? STACK_W : STACK_H;
    void getCurrentWindow().setSize({
      type: "Logical",
      data: { width: w, height: h },
    } as never).catch(() => {});
  }, [horizontal]);

  return (
    <div
      className="flex h-full flex-col overflow-hidden rounded-[10px] border border-[rgba(255,255,255,0.12)]"
      style={{ backgroundColor: "rgba(38,39,42,0.9)", backdropFilter: "blur(24px)" }}
    >
      {/* item rows (44pt each, storyboard value) */}
      <div className={`min-h-0 flex-1 overflow-y-auto p-1 ${horizontal ? "flex flex-row gap-1 overflow-x-auto" : ""}`}>
        {clips.length === 0 ? (
          <div className="flex h-full items-center justify-center text-[12px] text-neutral-500">
            {t("general.history-empty")}
          </div>
        ) : (
          clips.map((clip) => (
            <button
              key={clip.id}
              type="button"
              onClick={() => void invoke("stack_paste", { id: clip.id }).catch(console.error)}
              className={`group flex shrink-0 cursor-pointer items-center gap-2 rounded-[6px] px-2 text-left hover:bg-[rgba(0,119,252,0.35)] ${
                horizontal ? "h-full w-[132px] flex-col justify-center gap-1" : "h-[44px] w-full"
              }`}
            >
              <span className="flex h-[20px] w-[20px] shrink-0 items-center justify-center rounded-[4px] bg-[rgba(255,255,255,0.08)] text-neutral-300">
                <TypeIcon type={clip.contentType} size={12} />
              </span>
              {horizontal ? (
                <span className="line-clamp-3 w-full break-all text-center text-[10.5px] leading-tight text-neutral-200">
                  {(clip.plainContent ?? clip.content).replace(/\s+/g, " ").slice(0, 80)}
                </span>
              ) : (
                <>
                  <span className="min-w-0 flex-1">
                    <RowPreview clip={clip} />
                  </span>
                  <span className="shrink-0 text-[10px] tabular-nums text-neutral-500">
                    {formatTime(clip.copiedAt, t)}
                  </span>
                </>
              )}
            </button>
          ))
        )}
      </div>

      {/* bottom bar: direction toggle + close (left), empty label (right) */}
      <div className="flex h-[30px] shrink-0 items-center justify-between border-t border-[rgba(255,255,255,0.1)] px-1.5">
        <div className="flex items-center gap-0.5">
          <button
            type="button"
            title={t("settings.shortcuts.activate-paste-stack")}
            onClick={() => setHorizontal((v) => !v)}
            className="flex h-[22px] w-[22px] cursor-pointer items-center justify-center rounded-[5px] text-neutral-400 hover:bg-[rgba(255,255,255,0.1)] hover:text-neutral-100"
          >
            {horizontal ? (
              <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="m18 15-6-6-6 6" />
              </svg>
            ) : (
              <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <path d="m6 9 6 6 6-6" />
              </svg>
            )}
          </button>
          <button
            type="button"
            aria-label={t("general.action.cancel")}
            onClick={() => void invoke("stack_hide").catch(console.error)}
            className="flex h-[22px] w-[22px] cursor-pointer items-center justify-center rounded-[5px] text-neutral-400 hover:bg-[rgba(255,255,255,0.1)] hover:text-neutral-100"
          >
            <svg className="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round">
              <path d="M18 6 6 18" />
              <path d="m6 6 12 12" />
            </svg>
          </button>
        </div>
        <span className="pr-1 text-[10.5px] text-neutral-500">
          {clips.length === 0 ? t("general.history-empty") : ""}
        </span>
      </div>
    </div>
  );
}
