// Main-thread responsiveness probe for the W0 throughput measurement protocol.
//
// AC2 gates on "no main-thread interaction stall > 100 ms during the scripted
// load" (Apple responsiveness guidance). The browser exposes longtasks — work
// on the main thread that runs ≥ 50 ms uninterrupted — via PerformanceObserver.
// We forward stalls above the 100 ms threshold to the backend latency rollup
// (`frontend.main_thread_stall`) so the gate can pass/fail without scraping
// devtools or relying on per-event console output.
//
// Best-effort: PerformanceObserver throws if `longtask` is unsupported; we
// catch and no-op so unsupported runtimes (WebKit < 16 historically) don't
// break boot. The forwarding `invoke` is also fire-and-forget.

import { invoke } from "@tauri-apps/api/core";

const STALL_THRESHOLD_MS = 100;
const COALESCE_WINDOW_MS = 50;

let installed = false;
let pendingForwardId: number | undefined;
let pendingMaxDurationMs = 0;

function scheduleForward(durationMs: number) {
  pendingMaxDurationMs = Math.max(pendingMaxDurationMs, durationMs);
  if (pendingForwardId !== undefined) return;
  pendingForwardId = window.setTimeout(() => {
    const ms = pendingMaxDurationMs;
    pendingForwardId = undefined;
    pendingMaxDurationMs = 0;
    invoke("record_frontend_main_thread_stall", { durationMs: ms }).catch(() => {
      // Silent: telemetry must never break a foreground action.
    });
  }, COALESCE_WINDOW_MS);
}

export function installLongtaskObserver(): void {
  if (installed) return;
  if (typeof PerformanceObserver === "undefined") return;
  const supported = PerformanceObserver.supportedEntryTypes ?? [];
  if (!supported.includes("longtask")) return;
  try {
    const observer = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        if (entry.duration > STALL_THRESHOLD_MS) {
          scheduleForward(Math.round(entry.duration));
        }
      }
    });
    observer.observe({ type: "longtask", buffered: true });
    installed = true;
  } catch {
    // Browser doesn't support longtask or the observe options shape;
    // continue without the probe.
  }
}
