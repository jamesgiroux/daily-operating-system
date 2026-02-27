# I481 — Connector Gating + Mode Switching + Settings UI

**Status:** Done
**Priority:** P1
**Version:** 0.15.2
**Area:** Backend / Connectors + Frontend / Settings
**ADR:** 0095, 0096

## Summary

When context mode switches to Glean Governed, certain connectors become redundant or conflicting — Glean is the authoritative source. This issue implements connector gating by context mode, the `set_context_mode` command that triggers a full re-enrichment sweep, and the Settings UI for configuring the context source.

## Acceptance Criteria

### Connector gating

1. Gmail poller is disabled when context mode is Governed. Switching to Governed stops email polling; switching back to Local or Additive re-enables it.
2. Google Drive poller is disabled when context mode is Governed.
3. Clay and Gravatar enrichment are disabled in any Glean mode (Additive or Governed). Glean replaces these as the profile data source.
4. Linear poller remains active in all modes — it provides project signals not covered by Glean.
5. Connector state changes take effect immediately on mode switch without requiring an app restart.

### Signal bus integration

6. Signals originating from Glean sources are weighted at 0.7 confidence (below user corrections at 1.0, above AI-inferred at 0.5).
7. Glean-sourced signals have a 60-day half-life in the decay function.

### Mode switching

8. `set_context_mode` Tauri command exists and accepts mode + strategy parameters.
9. Switching mode triggers a full re-enrichment sweep: all entities with stale or missing intelligence are re-queued through the new context mode.
10. Mode switch is logged in the audit log (if AuditLogger is available) with `context_mode_changed` event.

### Settings UI

11. `ContextSourceSection.tsx` component exists in the Settings page under a "Context Source" or equivalent section.
12. Mode selector: Local / Glean radio or toggle. Selecting Glean reveals endpoint and token fields.
13. Glean endpoint URL field — validated as a URL on blur.
14. OAuth token field — masked input, stored securely.
15. Strategy selector (visible only in Glean mode): Additive / Governed with brief explanation of each.
16. Saving configuration persists to DB and takes effect immediately (connectors gated, context provider switched).

### Tests

17. `cargo test` passes.
18. `cargo clippy -- -D warnings` passes.

## Files

### New
- `src/components/settings/ContextSourceSection.tsx` — Settings UI for context mode configuration

### Modified
- `src-tauri/src/commands.rs` — `set_context_mode` command
- `src-tauri/src/state.rs` — mode switching logic, connector gating
- `src-tauri/src/google.rs` — Gmail/Drive poller respects context mode
- `src-tauri/src/enrichment.rs` — Clay/Gravatar gating
- `src-tauri/src/intel_queue.rs` — re-enrichment sweep on mode change
- `src-tauri/src/signals/` — Glean source weight and decay configuration
