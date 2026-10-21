/**
 * UI constants measured from the original Paste.app screenshots
 * (design/主界面.png etc., 2x Retina — values below are logical points).
 */
export const THEME = {
  panel: {
    /** Full-width bottom panel height (screenshot: 332pt). */
    height: 332,
    /** Toolbar top padding + pill height (screenshot: cards start at 64pt). */
    toolbarHeight: 64,
    /** Bottom padding below the card strip (screenshot: ~20pt). */
    bottomPadding: 20,
    background: "rgba(38, 39, 42, 0.92)",
    borderTop: "1px solid rgba(255, 255, 255, 0.08)",
    radius: 10,
  },
  search: {
    bg: "#3d4246",
    icon: "#9e9fa2",
    text: "#e1e2e2",
    widthCollapsed: 78,
    widthOpen: 300,
    height: 24,
  },
  tab: {
    textActive: "#dedfdf",
    textInactive: "#a1a2a4",
    iconSize: 14,
  },
  card: {
    /** Screenshot pitch is 244pt with flush cards; 242 + 2 gap matches it. */
    width: 242,
    height: 248,
    gap: 2,
    bg: "#141414",
    radius: 10,
    selectedBorder: "#0077fc",
    selectedGlow: "rgba(0, 119, 252, 0.35)",
    hairline: "rgba(255, 255, 255, 0.05)",
    contentText: "#dcdcdc",
    footerText: "#8b8b8b",
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
