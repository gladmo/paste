/**
 * UI constants measured pixel-by-pixel from design/主界面.png (2x Retina).
 * All values are logical points.
 */
export const THEME = {
  panel: {
    /** Full-width bottom panel height. */
    height: 332,
    /** Flat near-opaque dark background (screenshot: #252728 → #2a2b2e). */
    background: "#272a2c",
    /** Square corners, no top border (measured). */
  },
  toolbar: {
    /** Controls (search icon / pill / dots) are vertically centered at ~29.75pt. */
    height: 60,
    /** Distance between toolbar items. */
    gap: 18,
    /** Right margin of the "⋯" panel menu button. */
    dotsRight: 22.5,
  },
  search: {
    /** Collapsed state is a bare 16pt magnifier icon. */
    iconSize: 16,
    icon: "#9da1a2",
    /** Expanded input pill. */
    bg: "#3d4245",
    text: "#e1e2e2",
    widthOpen: 300,
    height: 24,
  },
  tab: {
    /** Active tab background chip (#3d4245, 27.5pt tall, radius 5). */
    pillBg: "#3d4245",
    pillHeight: 27.5,
    pillRadius: 5,
    pillPadLeft: 15,
    pillPadRight: 12.5,
    /** Icon-to-label gap inside a tab. */
    innerGap: 8,
    textActive: "#e1e2e2",
    textInactive: "#9a9c9e",
    fontSize: 13,
    /** History tab clock icon; pinboard tabs are 13pt solid color dots. */
    iconSize: 13,
    dotSize: 13,
  },
  card: {
    /** Outer 248pt box includes a 4pt border slot on every card. */
    outer: 248,
    border: 4,
    content: 240,
    /** Gap between card boxes (pitch 264pt). */
    gap: 16,
    radius: 11,
    bg: "#141414",
    selectedBorder: "#0077fc",
    /** Footer: centered single gray line. */
    footerText: "#959595",
    footerFontSize: 11,
    footerHeight: 24,
  },
  strip: {
    paddingLeft: 20,
    paddingTop: 4,
    paddingBottom: 20,
  },
  glyphs: {
    gray: "#9da1a2",
  },
} as const;

/** Default pinboard palette (from the original's tab colors). */
export const PINBOARD_COLORS: number[] = [
  0xef554d, // red (default Pinned)
  0xb663e0, // purple
  0x4285f4, // blue
  0x34c759, // green
  0xff9a00, // orange
  0x64d2ff, // cyan
  0xff375f, // pink
  0x8e8e93, // gray
];
