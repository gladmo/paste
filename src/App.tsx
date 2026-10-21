import { Overlay } from "./overlay/Overlay";
import { SettingsApp } from "./settings/SettingsApp";
import { PreviewApp } from "./preview/PreviewApp";
import { CreateApp } from "./create/CreateApp";
import { StackApp } from "./stack/StackApp";

/**
 * Route shell: every Tauri window loads index.html with a hash route.
 *   (none)      → clipboard history overlay (main panel)
 *   #/settings  → settings window
 *   #/preview   → item preview window
 *   #/create    → create text item window
 *   #/stack     → Paste Stack panel
 */
export default function App() {
  const hash = window.location.hash;
  switch (hash.split("?")[0]) {
    case "#/settings":
      return <SettingsApp />;
    case "#/create":
      return <CreateApp />;
    case "#/preview":
      return <PreviewApp />;
    case "#/stack":
      return <StackApp />;
    default:
      return <Overlay />;
  }
}
