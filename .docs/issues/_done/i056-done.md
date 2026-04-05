# I56 — Educational Redesign — Demo Data Mode and Guided Tour

**Status:** Open
**Priority:** P0
**Version:** 0.16.0
**Area:** Frontend / Onboarding

## Summary

A new user opening DailyOS for the first time needs to see what's possible before they've connected anything. Demo data mode shows a pre-populated workspace — real-looking accounts with intelligence, meetings with prep, signals active. The guided tour shows first-time users what each surface does through contextual callouts. Together these make the first minute of DailyOS self-explanatory.

## Acceptance Criteria

1. A "Try with demo data" option appears prominently on the empty dashboard state (before Google is connected). Activating it loads a pre-populated workspace with: 3 customer accounts with real-looking intelligence, 4 upcoming meetings with prep content, 5 active actions, 2 recent email signals. Demo data is clearly marked as demo — a persistent "Demo mode" badge in the folio bar.

2. The guided tour activates on first real launch (not demo mode). It consists of 4–6 contextual callouts, one per surface, that explain what each section does in plain language. Callouts are dismissible individually or all-at-once. They do not block interaction — the user can skip ahead at any time.

3. Demo mode is fully exitable. "Connect real data" clears demo data and starts the normal first-run flow. `SELECT count(*) FROM accounts WHERE is_demo = 1` returns 0 after exiting demo mode.

4. The guided tour state persists per session, not per launch. After completing the tour (or dismissing all callouts), it does not reappear on subsequent launches. `SELECT has_completed_tour FROM app_state` returns 1.

## Dependencies

None. Independent of I57 and I437 — can be built in parallel with either.

## Notes

Demo data should be realistic enough to be compelling — not obviously fake names, but clearly labeled as demo. The guided tour callouts must use existing UI patterns from the design system (tooltips/popovers that already exist). Do not invent new overlay patterns.
