# I439 — Personality Expanded in UI — More Copy Touchpoints

**Status:** Open
**Priority:** P1
**Version:** 0.14.1
**Area:** Frontend / UX

## Summary

Personality ("professional" / "friendly" / "playful") shapes how the app *speaks* to you — not the intelligence it produces. Intelligence is shaped by role presets. Personality is the tone of the app's voice in UI copy: empty states, completion messages, toasts, generating states, error messages. Today it reaches only 10–11 empty state strings. This issue expands personality coverage to every surface where the app speaks — creating genuine whimsy, warmth, or calm depending on the setting — without touching any AI-generated content.

## Acceptance Criteria

1. **Completion messages** — when the user completes an action, archives something, or dismisses a suggestion, a brief completion message appears. The tone is personality-driven: professional ("Done."), friendly ("Got it — that's taken care of."), playful ("Crushed it. ✓"). `getPersonalityCopy` gains a new key category: `"action-completed"`, `"action-dismissed"`, `"action-archived"`.

2. **Generating/loading states** — when the app is generating a briefing, building context for an account, or processing a transcript, the copy reflects personality. Professional: "Building context…" Friendly: "Pulling things together for you…" Playful: "Cooking something up…" New keys: `"generating-briefing"`, `"building-context"`, `"processing-transcript"`.

3. **Toast notifications** — the success toast copy for user-initiated actions is personality-driven. Professional: "Saved." Friendly: "All saved — you're good." Playful: "Locked in. 🎯" This applies to: saving a field, completing a setup step, connecting a connector. New keys: `"saved"`, `"connected"`, `"setup-complete"`.

4. **Error messages** — when something goes wrong (enrichment failed, sync error, connection lost), the tone is personality-consistent. Professional: "Sync failed. Check your connection and try again." Friendly: "Something went wrong — let's try that again." Playful: "Oops, that didn't work. Give it another go?" New keys: `"sync-error"`, `"connection-error"`, `"enrichment-failed"`.

5. **The three missing empty state keys are wired** (defined in `personality.ts` but never called): `"accounts-empty"` → AccountsPage, `"projects-empty"` → ProjectsPage, `"actions-waiting-empty"` → ActionsPage waiting-on tab.

6. **Personality does NOT touch any AI prompt.** `grep -rn "personality\|tone_instruction\|build_tone" src-tauri/src/` — returns only config storage and validation code, never prompt construction. Intelligence is framed by role presets; personality is the app's voice in UI copy only.

7. All new copy keys have values for all three personality settings. `getPersonalityCopy(key, 'professional')`, `getPersonalityCopy(key, 'friendly')`, and `getPersonalityCopy(key, 'playful')` all return non-null, non-identical strings for every new key.

## Dependencies

None. Pure frontend work. Benefits from I441 (useActivePreset cache) being done first since some personality copy appears near preset-dependent surfaces, but not a hard dependency.

## Notes / Rationale

**The design principle (from the user):** "I'm looking for whimsy in the app that makes it fun to use — professional or calm or whatever the other personalities are. The role presets determine the way intelligence prompts are done and that's different. Personality shouldn't touch AI prompts unless we have explicit use cases for it."

Personality is the app's personality, not the intelligence's personality. The intelligence is produced for business purposes — it should be shaped by what the user does (role preset), not how they want the app to feel (personality). These are orthogonal axes.

The copy touchpoints that matter most: generating states (where the user waits and the app can be delightful or calm), completion/save confirmations (where a moment of personality reinforces the action), and error messages (where tone can reduce friction). These are high-frequency, short-duration interactions where tone has outsized impact on the overall feel of the product.
