import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import App from "./App";
import "./index.css";

// Surface any webview JS failure through the debug bridge so it lands in
// /tmp/paste.log even when no devtools are attached (release installs).
const dbg = (line: string) => {
  void invoke("debug_log", { line }).catch(() => {});
};
window.addEventListener("error", (e) => {
  dbg(`[web] error: ${e.message} @ ${e.filename}:${e.lineno}`);
});
window.addEventListener("unhandledrejection", (e) => {
  dbg(`[web] rejection: ${String(e.reason?.stack ?? e.reason).slice(0, 500)}`);
});
// Page-liveness marker: when the panel ever appears completely blank, this
// line (or its absence) in /tmp/paste.log tells "page never loaded" apart
// from "page loaded but data hung" — the two failure classes look identical
// on screen.
dbg("[web] page up");

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
