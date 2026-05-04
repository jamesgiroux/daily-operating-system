# I440 — Meeting Prep Preset Persona

**Status:** Open
**Priority:** P1
**Version:** 0.14.1
**Area:** Backend / Intelligence

## Summary

The meeting prep prompt in `workflow/deliver.rs` (approximately line 3374) hardcodes the string "Customer Success Manager" as the AI's persona when generating meeting intelligence. Every user — Sales, Product, Leadership, Marketing, Partnerships — gets prep framed for a CSM: renewal language, health scores, QBR preparation. This issue removes the hardcoded string and replaces it with `config.active_preset.name` (and the preset's `briefing_emphasis`), so that meeting prep speaks to the user in their actual role's vocabulary.

## Acceptance Criteria

1. The "Customer Success Manager" hardcoded string in `workflow/deliver.rs` is removed. In its place: `config.active_preset.name` (or equivalent) is read and used as the role description in the meeting prep persona. Verify: `grep -rn "Customer Success Manager" src-tauri/src/` — returns 0 results.
2. With the Sales preset active, open a meeting detail page for a customer meeting. The prep's framing language reflects a sales context — deal stage, close probability, competitive context — not renewal, health score, or QBR language.
3. With the Leadership preset active, meeting prep for a strategic account meeting uses leadership vocabulary — strategic priorities, board-level context, executive alignment — not CSM operational framing.
4. The preset's `briefing_emphasis` field is already injected in some prompts. Confirm it is also injected into the meeting prep prompt after this change. Verify: `grep -n "briefing_emphasis\|briefingEmphasis" src-tauri/src/workflow/deliver.rs` — appears in the meeting prep prompt section.
5. `cargo test` passes. No regressions on existing meeting prep for CS preset users.

## Dependencies

- No hard blockers. Self-contained backend change.
- CS preset users should not experience any change in prep quality; their preset name is "Customer Success" and the vocabulary fields align with the prior hardcoded framing.

## Notes / Rationale

This is the single highest-impact fix for non-CS users in this version. Every Sales, Product, Leadership, Marketing, Partnerships, Agency, Consulting, and The Desk user currently receives meeting prep framed for a Customer Success Manager. The fix is small in code surface — replace one hardcoded string with a config read — but the effect is significant: the prep's analytical frame, the questions it suggests, and the risk signals it emphasizes will all shift to match the user's actual role.

The hardcoded string is at approximately `workflow/deliver.rs:3374`. The surrounding prompt context constructs the AI persona instruction ("You are a [role]..."). After this change, the construction reads from `config.active_preset.name` and injects `config.active_preset.briefing_emphasis` as the focus context for the prep. The preset's `vocabulary.riskVocabulary` and `vocabulary.urgencySignals` (already read by the enrichment pipeline) should also be available to the meeting prep prompt builder — verify they are injected or add them if missing.
