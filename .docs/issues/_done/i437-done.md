# I437 — Empty State Redesign — Every Surface Guides Action

**Status:** Open
**Priority:** P1
**Version:** 0.16.0
**Area:** Frontend / UX

## Summary

Every surface that currently renders a generic "nothing here" state is redesigned to guide the user toward the action that fills it. Empty states are the app's most frequent first impression for new users — they should demonstrate the value of what's missing, not just confirm its absence. Copy adapts to the active role preset vocabulary.

## Acceptance Criteria

1. Every surface that currently renders a generic empty state has a purpose-specific redesign. Minimum surfaces covered: Accounts page, People page, Actions page, Email page, Daily briefing, Meeting detail (no prep), Entity detail (no intelligence).

2. Every empty state contains three elements:
   - **A one-sentence explanation** of what this surface shows when it has data (not "No accounts yet" — "Your accounts are where DailyOS builds intelligence about your customer relationships")
   - **A direct action button** that begins filling it ("Add your first account", "Connect Google to see meetings", "Process your inbox")
   - **A brief benefit statement** explaining what the user gets when it's filled ("Once added, DailyOS will surface risks, opportunities, and meeting prep for this account automatically")

3. Empty state copy uses the active role preset's vocabulary. With the CS preset active: "accounts," "customers," "renewals." With the Sales preset: "deals," "pipeline," "prospects." Verify: switch presets in Settings, navigate to the Accounts page empty state — copy reflects the preset's terminology.

4. No surface renders `null`, a blank white area, or a generic "Nothing here yet" string when it has no data. Verify by:
   - Creating a fresh test DB (`~/.dailyos/dailyos-test.db`)
   - Launching the app pointing at the empty DB
   - Navigating every main surface — each shows a purposeful empty state, never blank

5. Empty states do not block interaction. If the page has a search bar or filter, it is still present and functional on the empty state. The action button in the empty state is the primary CTA, not the only thing on the page.

6. The Daily briefing empty state (when no Google is connected OR when the briefing hasn't generated yet) is distinct from the loading state. Loading: skeleton UI with shimmer. Empty/unconnected: the purposeful empty state with "Connect Google to get started."

7. Existing `DashboardEmpty` and `EntityListEmpty` components are updated or replaced. `grep -rn "DashboardEmpty\|EntityListEmpty\|Nothing here\|No data" src/ --include="*.tsx"` — every instance has been reviewed and either updated or justified as intentionally generic.

## Dependencies

None. Pure frontend work. Can be built in parallel with I56 and I57. Contributes to the v0.16.0 onboarding experience — better empty states make the first-run wizard feel less overwhelming.

## Notes / Rationale

Empty states are the app's first impression for new features, not just for new users. When a feature is configured for the first time (first Clay enrichment, first Drive import, first Linear sync), the user lands on an empty state. If that empty state says "No data" rather than "Clay hasn't enriched anyone yet — it will run automatically in the next 24 hours," the user doesn't know whether the integration is working or broken.

The role-preset vocabulary requirement matters because a Sales AE's first impression of the Accounts page shouldn't say "Build intelligence about your customer relationships" if they think in terms of "pipeline" and "deals." The terminology should feel native to how they already think about their work.
