import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "@/app";
import "@/styles/index.css";

const rootElement = document.getElementById("root");

if (rootElement === null) {
  throw new Error("Root element #root was not found in the document.");
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
