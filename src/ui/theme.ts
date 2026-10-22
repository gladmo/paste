/**
 * UI constants for the card-shelf design (paste-ui.html):
 * glass panel, 240×280 cards with colored type headers, source-app badges
 * and a content-meta footer.
 */
export const THEME = {
  panel: {
    /** Full-width bottom panel height: 12 + 28 toolbar + 12 + 8 + 280 + 8 + 20. */
    height: 368,
    /** Glass tint over the macOS vibrancy blur. */
    background: "rgba(22, 23, 26, 0.88)",
    borderTop: "1px solid rgba(255, 255, 255, 0.10)",
    paddingTop: 12,
    /** Toolbar row height + its bottom margin. */
    headerHeight: 28,
    headerMargin: 12,
    paddingBottom: 20,
    stripPaddingY: 8,
    stripPaddingX: 24,
  },
  toolbar: {
    gap: 8,
    /** Right margin of the "⋯" options button (right-7). */
    dotsRight: 28,
  },
  search: {
    iconSize: 16,
    icon: "#a3a3a3",
    bg: "#2b2d33",
    text: "#f5f5f5",
    widthOpen: 300,
    height: 28,
  },
  tab: {
    /** Active tab pill. */
    pillBg: "#2b2d33",
    fontSize: 12,
    iconSize: 14,
    /** Pinboard dot size (w-2 h-2). */
    dotSize: 8,
    textActive: "#ffffff",
    textInactive: "#d4d4d4",
  },
  card: {
    width: 240,
    height: 280,
    gap: 14,
    radius: 16,
    bg: "#1d1f23",
    active: "#007aff",
    /** Footer meta line. */
    footerText: "#a3a3a3",
    footerFontSize: 11,
  },
  /** Card header background per content type. */
  headerBg: {
    text: "#1b72e8",
    richtext: "#1b72e8",
    link: "#f05252",
    file: "#64748b",
    image: "#f08800",
    color: "#8b5cf6",
    default: "#1b72e8",
  } as Record<string, string>,
  glyphs: {
    gray: "#a3a3a3",
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
