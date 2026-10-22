import { useEffect, useState } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import DOMPurify from "dompurify";

import { useT } from "../i18n";
import type { Clip } from "../types";

/**
 * textutil emits HTML with a <style> block + classes (p.p1, span.s1).
 * DOMPurify strips <style>, so inline the rules into style attributes
 * before sanitizing — otherwise all colors/fonts are lost.
 */
export function inlineStyles(html: string): string {
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

/** Read the background color of the copied content (e.g. GoLand's dark
 *  editor bg), so the card content area matches the original editor. */
export function extractBgColor(html: string): string | undefined {
  const doc = new DOMParser().parseFromString(html, "text/html");
  const firstStyled = doc.body.querySelector("[style*='background']");
  if (!firstStyled) return undefined;
  const m = (firstStyled.getAttribute("style") ?? "").match(
    /background-color:\s*([^;]+)/,
  );
  const color = m?.[1]?.trim();
  return color && color !== "transparent" ? color : undefined;
}

export function linkDomain(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

/** Relative time caption using localized replica strings. */
export function formatTime(ts: number, t: ReturnType<typeof useT>): string {
  const diff = Date.now() - ts;
  const min = Math.floor(diff / 60000);
  if (min < 1) return t("replica.time.now");
  if (min < 60) return t("replica.time.min-ago", min);
  const hours = Math.floor(min / 60);
  if (hours < 24) return t("replica.time.h-ago", hours);
  const days = Math.floor(hours / 24);
  if (days < 30) return t("replica.time.d-ago", days);
  return new Date(ts).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}

/**
 * Content-meta caption rendered at the bottom of every card, matching the
 * original app's Localizable.stringsdict keys:
 *   %d characters / %d files / %d items / %d lines / %d words
 * (e.g. a text clip shows "59 characters", a single file just "File").
 */
export function clipMeta(clip: Clip, t: ReturnType<typeof useT>): string {
  if (clip.contentType === "file") {
    const paths = (clip.plainContent ?? clip.content).split("\n").filter(Boolean);
    return paths.length > 1
      ? t("replica.meta.files", paths.length)
      : t("replica.meta.file");
  }
  if (
    clip.contentType === "text" ||
    clip.contentType === "richtext" ||
    clip.contentType === "link"
  ) {
    const n = (clip.plainContent ?? clip.content).length;
    return n === 1 ? t("replica.meta.character", n) : t("replica.meta.characters", n);
  }
  // Images and colours have no localized plural form in the original app.
  return "";
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
      setMeta((prev) => (prev ? `${img.naturalWidth}×${img.naturalHeight} · ${prev}` : `${img.naturalWidth}×${img.naturalHeight}`));
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

/** Source app icon (hidden when unknown). */
export function AppIcon({ bundleId, name, size = 38 }: { bundleId?: string | null; name?: string | null; size?: number }) {
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
      })
      .catch(() => {});
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
      className="rounded-[7px] object-contain"
      style={{ width: size, height: size }}
      draggable={false}
    />
  );
}

/** Card body: content preview for each clip type (text/rich/link/image). */
export function CardPreview({ clip }: { clip: Clip }) {
  if (clip.contentType === "image") {
    return (
      <div className="min-h-0 flex-1 overflow-hidden rounded-[7px]">
        <img
          src={convertFileSrc(clip.content)}
          alt="clip"
          className="h-full w-full object-contain"
          draggable={false}
        />
      </div>
    );
  }
  if (clip.contentType === "richtext") {
    // Clipboard HTML is untrusted: sanitize before rendering so copied
    // content can never inject scripts into the panel.
    const sanitized = DOMPurify.sanitize(inlineStyles(clip.content));
    const bg = extractBgColor(sanitized);
    return (
      <div
        className="min-h-0 flex-1 overflow-hidden rounded-[7px] p-2 pb-[26px]"
        style={{ backgroundColor: bg ?? "rgba(255,255,255,0.04)" }}
      >
        <div
          className="rich-preview h-full overflow-hidden text-[13px] leading-snug"
          style={{ color: "#dcdcdc" }}
          dangerouslySetInnerHTML={{ __html: sanitized }}
        />
      </div>
    );
  }
  if (clip.contentType === "link") {
    return (
      <div className="min-h-0 flex-1 space-y-1.5 overflow-hidden p-2 pb-[26px]">
        {clip.title && (
          <p className="line-clamp-2 text-[13px] font-semibold" style={{ color: "#dcdcdc" }}>
            {clip.title}
          </p>
        )}
        <p className="truncate text-[12px] font-medium text-[#60a5fa]">{linkDomain(clip.content)}</p>
        <p className="line-clamp-3 break-all text-[11px] leading-snug" style={{ color: "#dcdcdc" }}>
          {clip.content}
        </p>
      </div>
    );
  }
  if (clip.contentType === "file") {
    const paths = (clip.plainContent ?? clip.content).split("\n").filter(Boolean);
    return (
      <div className="min-h-0 flex-1 space-y-1 overflow-hidden p-2 pb-[26px]">
        {paths.slice(0, 4).map((p) => (
          <p key={p} className="truncate text-[12px]" style={{ color: "#dcdcdc" }}>
            {p.split("/").pop()}
          </p>
        ))}
        {paths.length > 4 && (
          <p className="text-[11px] text-neutral-500">+{paths.length - 4}</p>
        )}
      </div>
    );
  }
  if (clip.contentType === "color") {
    let hex = "#4da3ff";
    try {
      const m = clip.meta ? JSON.parse(clip.meta) : null;
      if (m?.color) hex = m.color;
    } catch {
      /* keep default */
    }
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center p-3 pb-[26px]">
        <div
          className="h-full w-full rounded-[8px] ring-1 ring-inset ring-[rgba(255,255,255,0.2)]"
          style={{ backgroundColor: hex }}
        />
      </div>
    );
  }
  return (
    <div className="min-h-0 flex-1 overflow-hidden p-2.5 pb-[26px]">
      <p
        className="line-clamp-7 whitespace-pre-wrap break-all text-[13px] leading-snug"
        style={{ color: "#dcdcdc" }}
      >
        {clip.content}
      </p>
    </div>
  );
}

/** Card footer meta: image dims + byte size / char count / link domain / file info. */
export function CardContentMeta({ clip }: { clip: Clip }) {
  if (clip.contentType === "image") {
    return <ImageMeta src={convertFileSrc(clip.content)} />;
  }
  if (clip.contentType === "link") {
    return <span className="truncate">{clip.title || linkDomain(clip.content)}</span>;
  }
  if (clip.contentType === "file") {
    const paths = (clip.plainContent ?? clip.content).split("\n").filter(Boolean);
    let size = "";
    try {
      const m = clip.meta ? JSON.parse(clip.meta) : null;
      if (m?.totalSize) size = ` · ${formatSize(m.totalSize as number)}`;
    } catch {
      /* ignore */
    }
    const label =
      paths.length > 1 ? `${paths.length} files` : (paths[0]?.split(".").pop() ?? "file");
    return (
      <span className="truncate">
        {label}
        {size}
      </span>
    );
  }
  if (clip.contentType === "color") {
    let hex = clip.content;
    try {
      const m = clip.meta ? JSON.parse(clip.meta) : null;
      if (m?.color) hex = m.color as string;
    } catch {
      /* ignore */
    }
    return <span className="truncate uppercase">{hex}</span>;
  }
  return (
    <span className="truncate">
      {(clip.plainContent ?? clip.content).length} chars
    </span>
  );
}
