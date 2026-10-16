import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/manrope";
import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-mono/400.css";
import "./index.css";
import "./i18n";
import App from "./App";

if (window.__TAURI_INTERNALS__) document.body.dataset.native = "true";

const root = document.getElementById("root");
if (root) {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
