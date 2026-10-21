import { Overlay } from "./overlay/Overlay";
import { useT } from "./i18n";

/**
 * Route shell: every Tauri window loads index.html with a hash route.
 *   (none)      → clipboard history overlay (main panel)
 *   #/settings  → settings window (P2)
 *   #/preview   → item preview window (P3)
 *   #/create    → create text item window (P3)
 *   #/stack     → Paste Stack panel (P4)
 */
function SettingsPlaceholder() {
  const t = useT();
  return (
    <div className="flex h-full items-center justify-center bg-[#2a2b2e] text-neutral-400">
      {t("general.settings")}…
    </div>
  );
}

function CreatePlaceholder() {
  const t = useT();
  return (
    <div className="flex h-full items-center justify-center bg-[#2a2b2e] text-neutral-400">
      {t("general.new-text-item")}…
    </div>
  );
}

function PreviewPlaceholder() {
  const t = useT();
  return (
    <div className="flex h-full items-center justify-center bg-[#2a2b2e] text-neutral-400">
      {t("general.action.preview")}…
    </div>
  );
}

function StackPlaceholder() {
  return <div className="h-full bg-[#2a2b2e]" />;
}

export default function App() {
  const hash = window.location.hash;
  switch (hash) {
    case "#/settings":
      return <SettingsPlaceholder />;
    case "#/create":
      return <CreatePlaceholder />;
    case "#/preview":
      return <PreviewPlaceholder />;
    case "#/stack":
      return <StackPlaceholder />;
    default:
      return <Overlay />;
  }
}
