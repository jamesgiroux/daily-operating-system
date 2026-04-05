# Reconnection Audit — v1.0.2

**Date:** 2026-03-21
**Methodology:** Automated grep for exported-but-unused components, signal type emitter verification, propagation rule audit. Manual review of key pages.

---

## Findings

### 1. WatchListPrograms — exported but never wired (FIXED)

- **Type:** Missing feature (regression)
- **File:** `src/components/account/WatchListPrograms.tsx:26`
- **Problem:** Component fully built and exported. `WatchList` accepts `bottomSection` prop. `useAccountDetail` returns `programs`, `handleProgramUpdate`, `handleProgramDelete`, `handleAddProgram`. But `AccountDetailEditorial.tsx` never connected them.
- **Fix:** Wired `WatchListPrograms` into `WatchList`'s `bottomSection` prop in `AccountDetailEditorial.tsx`.
- **Status:** Fixed in this audit.

### 2. Stale I377 comment on rule_meeting_frequency_drop (FIXED)

- **Type:** Stale documentation
- **File:** `src-tauri/src/signals/rules.rs:62-65`
- **Problem:** Comment claims rule is "dead" and unregistered, but I555 re-activated it. `services::meetings::process_transcript` now emits `meeting_frequency` signals, and the rule is re-registered in `default_engine()`.
- **Fix:** Updated comment to reflect I555 re-activation.
- **Status:** Fixed in this audit.

### 3. All propagation rules verified active

All 12 propagation rules in `signals/propagation.rs` have confirmed signal emitters:
- `rule_meeting_frequency_drop` — emitter in `services/meetings.rs`
- `rule_champion_change` — emitter in `processor/transcript.rs`
- `rule_contract_proximity` — emitter in `services/accounts.rs`
- `rule_stakeholder_departure` — emitter in `services/people.rs`
- `rule_email_sentiment_cascade` — emitter in `signals/email_bridge.rs`
- All others verified via grep.

No dead rules found.

### 4. All exported dashboard/meeting/entity components verified wired

Grep of `export function` and `export const` across `src/components/` confirmed all exports are imported by at least one consumer. No orphaned components beyond Finding #1.

### 5. Signal types — all have emitters

All 9 signal types referenced in propagation rules have corresponding `emit_signal` or `emit_signal_and_propagate` calls in the codebase. `person_departed` is intentionally absent — `company_change` covers it by design.

---

## Summary

| # | Finding | Type | Severity | Status |
|---|---------|------|----------|--------|
| 1 | WatchListPrograms not wired | Missing feature | Medium | Fixed |
| 2 | Stale I377 comment | Stale docs | Low | Fixed |
| 3 | Propagation rules | Verification | — | All active |
| 4 | Exported components | Verification | — | All wired |
| 5 | Signal emitters | Verification | — | All present |

**No new issues filed.** All findings resolved in this audit.
