import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { PanelApp } from "./PanelApp";
import "./styles.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Missing root element");
}

const RootComponent =
  new URLSearchParams(window.location.search).get("window") === "panel" ? PanelApp : App;

createRoot(root).render(
  <StrictMode>
    <RootComponent />
  </StrictMode>,
);
