/** Shared frontend types mirroring the Rust structs (camelCase serde). */

export interface Clip {
  id: number;
  contentType: string;
  content: string;
  plainContent?: string | null;
  rtf?: string | null;
  title?: string | null;
  copiedAt: number;
  pinned: boolean;
  sourceApp?: string | null;
  sourceName?: string | null;
  pinnedAt?: number | null;
  pinboardId?: number | null;
  displayOrder?: number | null;
}

export interface Pinboard {
  id: number;
  name: string;
  /** Packed 0xRRGGBB. */
  color: number;
  displayOrder: number;
  createdAt: number;
}

export type PanelView = "history" | "pinned" | "board";

/** BUILTIN_PINNED_ID on the Rust side. */
export const BUILTIN_PINNED_ID = 1;

export function packedToCss(color: number): string {
  const r = (color >> 16) & 0xff;
  const g = (color >> 8) & 0xff;
  const b = color & 0xff;
  return `rgb(${r}, ${g}, ${b})`;
}
