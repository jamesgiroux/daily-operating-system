import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";
import { installLongtaskObserver } from "./lib/longtaskObserver";

// W0-B: surface main-thread stalls > 100 ms to the backend latency rollup
// (AC2 gate of the throughput measurement protocol).
installLongtaskObserver();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
