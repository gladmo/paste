import { useCallback, useEffect, useRef, useState } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import DOMPurify from "dompurify";

import { useT } from "../i18n";
import type { Clip } from "../types";
import { inlineStyles, linkDomain } from "../overlay/preview";

function parseId(): number {
  const m = window.location.hash.match(/[?&]id=(\d+)/);
  return m ? Number(m[1]) : 0;
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = src;
  });
}

async function rotateToBase64(src: string, dir: 1 | -1): Promise<string> {
  const img = await loadImage(src);
  const canvas = document.createElement("canvas");
  canvas.width = img.naturalHeight;
  canvas.height = img.naturalWidth;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("no 2d context");
  ctx.translate(canvas.width / 2, canvas.height / 2);
  ctx.rotate((dir * Math.PI) / 2);
  ctx.drawImage(img, -img.naturalWidth / 2, -img.naturalHeight / 2);
  const dataUrl = canvas.toDataURL("image/png");
  return dataUrl.slice(dataUrl.indexOf(",") + 1);
}

function ToolButton({
  label,
  onClick,
  active,
}: {
  label: React.ReactNode;
  onClick: () => void;
  active?: boolean;
}) {
  return (
    <button
      type="button"
      onMouseDown={(e) => e.preventDefault()}
      onClick={onClick}
      className={`flex h-[26px] min-w-[28px] cursor-pointer items-center justify-center rounded-[5px] px-1.5 text-[12px] font-semibold transition-colors ${
        active ? "bg-[#0a84ff] text-white" : "text-neutral-200 hover:bg-[rgba(255,255,255,0.1)]"
      }`}
    >
      {label}
    </button>
  );
}

/** Item preview window: text/rich editing with the original toolbar,
 *  image rotation, link details. */
export function PreviewApp() {
  const t = useT();
  const [clip, setClip] = useState<Clip | null>(null);
  const [dirty, setDirty] = useState(false);
  const editorRef = useRef<HTMLDivElement>(null);
  const areaRef = useRef<HTMLTextAreaElement>(null);
  const [saving, setSaving] = useState(false);

  const load = useCallback(async () => {
    try {
      const c = await invoke<Clip | null>("get_clip", { id: parseId() });
      setClip(c);
      setDirty(false);
    } catch (e) {
      console.error(e);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  if (!clip) {
    return <div className="flex h-full items-center justify-center bg-[#2a2a2c] text-neutral-500">…</div>;
  }

  const current = clip;

  async function save() {
    if (!dirty || !current) return;
    setSaving(true);
    try {
      if (current.contentType === "richtext" && editorRef.current) {
        const html = editorRef.current.innerHTML;
        const plain = editorRef.current.innerText ?? "";
        await invoke("update_clip_content", { id: current.id, content: html, plainContent: plain });
      } else if (current.contentType === "text" && areaRef.current) {
        await invoke("update_clip_content", {
          id: current.id,
          content: areaRef.current.value,
          plainContent: areaRef.current.value,
        });
      }
      setDirty(false);
    } finally {
      setSaving(false);
    }
  }

  function exec(cmd: string) {
    document.execCommand(cmd);
    editorRef.current?.focus();
    setDirty(true);
  }

  const isRich = clip.contentType === "richtext";
  const isText = clip.contentType === "text";

  return (
    <div className="flex h-full flex-col bg-[#2a2a2c] text-neutral-100">
      {/* toolbar */}
      <div className="flex h-[38px] shrink-0 items-center gap-1 border-b border-[rgba(255,255,255,0.08)] px-2.5">
        {(isRich || isText) && (
          <>
            {isRich && (
              <>
                <ToolButton label="B" onClick={() => exec("bold")} />
                <ToolButton label="I" onClick={() => exec("italic")} />
                <ToolButton label="U" onClick={() => exec("underline")} />
                <ToolButton label="S" onClick={() => exec("strikeThrough")} />
                <span className="mx-1 h-4 w-px bg-[rgba(255,255,255,0.12)]" />
              </>
            )}
          </>
        )}
        {clip.contentType === "image" && (
          <>
            <ToolButton
              label={
                <span className="flex items-center gap-1">
                  <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M2.5 2v6h6" />
                    <path d="M2.5 8a10 10 0 1 1 1.07 5.25" />
                  </svg>
                  {t("edit-item.toolbar.image.rotate-left")}
                </span>
              }
              onClick={async () => {
                const b64 = await rotateToBase64(convertFileSrc(clip.content), -1);
                await invoke("save_clip_image", { id: clip.id, pngBase64: b64 });
                await load();
              }}
            />
            <ToolButton
              label={
                <span className="flex items-center gap-1">
                  <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                    <path d="M21.5 2v6h-6" />
                    <path d="M21.5 8a10 10 0 1 0-1.07 5.25" />
                  </svg>
                  {t("edit-item.toolbar.image.rotate-right")}
                </span>
              }
              onClick={async () => {
                const b64 = await rotateToBase64(convertFileSrc(clip.content), 1);
                await invoke("save_clip_image", { id: clip.id, pngBase64: b64 });
                await load();
              }}
            />
          </>
        )}
        <div className="ml-auto flex items-center gap-1.5">
          <button
            type="button"
            onClick={() => void load()}
            className="cursor-pointer rounded-[5px] px-2.5 py-1 text-[12px] text-neutral-300 hover:bg-[rgba(255,255,255,0.08)]"
          >
            {t("edit-item.toolbar.discard-changes")}
          </button>
          <button
            type="button"
            disabled={!dirty || saving}
            onClick={() => void save()}
            className="cursor-pointer rounded-[5px] bg-[#0a84ff] px-2.5 py-1 text-[12px] font-semibold text-white hover:bg-[#2a95ff] disabled:opacity-40"
          >
            {t("edit-item.toolbar.save-changes")}
          </button>
        </div>
      </div>

      {/* content */}
      <div className="min-h-0 flex-1 overflow-auto p-3">
        {clip.contentType === "image" && (
          <div className="flex h-full items-center justify-center">
            <img
              src={convertFileSrc(clip.content)}
              alt="clip"
              className="max-h-full max-w-full rounded-[6px] object-contain"
              draggable={false}
            />
          </div>
        )}
        {clip.contentType === "link" && (
          <div className="flex flex-col gap-2">
            {clip.title && <p className="text-[15px] font-semibold">{clip.title}</p>}
            <p className="text-[13px] text-[#4da3ff]">{linkDomain(clip.content)}</p>
            <p className="break-all text-[12px] text-neutral-300">{clip.content}</p>
          </div>
        )}
        {clip.contentType === "text" && (
          <textarea
            ref={areaRef}
            defaultValue={clip.content}
            onChange={() => setDirty(true)}
            spellCheck={false}
            className="h-full w-full resize-none rounded-[6px] bg-[rgba(255,255,255,0.05)] p-3 text-[13px] leading-relaxed text-neutral-100 outline-none ring-1 ring-[rgba(255,255,255,0.1)] focus:ring-[#0077fc]"
          />
        )}
        {isRich && (
          <div
            ref={editorRef}
            contentEditable
            suppressContentEditableWarning
            onInput={() => setDirty(true)}
            className="rich-preview min-h-full rounded-[6px] bg-[rgba(255,255,255,0.05)] p-3 text-[13px] leading-relaxed text-neutral-100 outline-none ring-1 ring-[rgba(255,255,255,0.1)] focus:ring-[#0077fc]"
            dangerouslySetInnerHTML={{ __html: DOMPurify.sanitize(inlineStyles(clip.content)) }}
          />
        )}
      </div>
    </div>
  );
}
