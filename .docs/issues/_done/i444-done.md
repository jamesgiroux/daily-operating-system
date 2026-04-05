# I444 — lifecycle_events Wired

**Status:** Open
**Priority:** P1
**Version:** 0.14.1
**Area:** Backend / Frontend

## Summary

The `lifecycle_events` field is defined in all 9 preset JSONs, typed in TypeScript, and validated — but consumed by nothing. The lifecycle stage picker on account detail pages uses a hardcoded list of stages, and the lifecycle stage stored in account metadata is never injected into entity intelligence prompts. This issue has two parts: wire the UI picker to the active preset's `lifecycle_events` array, and inject the account's current lifecycle stage into entity intelligence prompt assembly so the AI has role-appropriate context when analysing an account.

## Acceptance Criteria

1. The lifecycle stage picker on account detail pages shows stages from the active preset's `lifecycle_events` array. With Customer Success preset: "Renewal," "Expansion," "Contraction," "Churn," "Escalation," "Executive Review" etc. With Product preset: "Discovery," "Beta," "Launched," "Deprecated" etc.
2. The current lifecycle stage is stored as a string in the account's metadata. Switching presets does not erase or invalidate existing lifecycle stages — they display as-is even if the new preset does not define that stage.
3. The lifecycle stage context is injected into entity intelligence prompts alongside vocabulary and briefing emphasis. Verify: `grep -n "lifecycle" src-tauri/src/intelligence/prompts.rs` — the lifecycle stage from account metadata appears in the assembled prompt when a stage is set.

## Dependencies

- I441 (useActivePreset cache) — the lifecycle stage picker needs the active preset from context.
- The backend prompt injection (criterion 3) benefits from the pattern established in I439 (personality into prompts) but is not strictly blocked by it.

## Notes / Rationale

**UI part:** Same pattern as I442 and I443. The lifecycle stage picker on account detail pages has a hardcoded options list. Replace with `activePreset.lifecycle_events`. The lifecycle stage is already stored in account `metadata` JSON as a string — this part of the storage model does not change.

**Backend part:** The entity intelligence prompt builder in `src-tauri/src/intelligence/prompts.rs` assembles a prompt fragment that includes vocabulary and `briefing_emphasis`. It should also include the account's current lifecycle stage when one is set. The injection format should be a single line appended to the account context block: "Account lifecycle stage: [stage]". If no lifecycle stage is set (`metadata.lifecycle` is null or empty), nothing is injected — no empty section header.

The lifecycle stage provides meaningful context to the AI: an account at "Renewal" stage warrants renewal-specific risk framing; an account at "Expansion" stage warrants expansion signal framing. This is low-effort context that meaningfully improves enrichment quality for accounts with a stage set.

The stage value is a freeform string stored in metadata — the backend reads it without validating against the current preset's vocabulary. An account with lifecycle "Renewal" that was set under the CS preset will still inject "Account lifecycle stage: Renewal" even if the user switches to a Product preset where "Renewal" is not a defined stage. The AI handles this gracefully.
