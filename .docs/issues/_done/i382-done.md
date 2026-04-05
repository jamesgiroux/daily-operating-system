# I382 — Partner Entity Type — `partner` Account Type, Badge, Prompt Shape

**Status:** Open (0.13.3)
**Priority:** P1
**Version:** 0.13.3
**Area:** Backend / Entity

## Summary

DailyOS currently has two account types: `customer` (accounts you sell to) and `internal` (teams within your organization). Agencies, SIs, consulting firms, and channel partners don't fit either category — they're entities you coordinate with but don't sell to. This issue adds `partner` as a third first-class account type, with a distinct visual badge, and an AI enrichment prompt shape appropriate for partner relationships (alignment health, joint deliverables, communication cadence, escalation risk) rather than the customer health/spend/renewal shape.

## Acceptance Criteria

From the v0.13.3 brief, verified with real data in the running app:

1. `partner` is a valid `account_type` in the DB enum and Rust type system. Creating an account with type `partner` via the UI or directly in the DB succeeds without error.
2. Partner accounts render with a distinct visual badge on account cards, account detail pages, and anywhere account type is displayed (meeting attendee context, entity chips). The badge is visually differentiated from both `customer` and `internal`.
3. The AI enrichment prompt for partner entities uses a partner-appropriate shape — alignment health, joint deliverables, communication cadence, escalation risk — not the customer health/spend/renewal shape and not the internal coordination shape. Verify by checking the prompt template or reading the resulting `intelligence.json` for a partner entity: no `renewal_risk` or `spend` fields should appear; `alignment` and `deliverables` fields should.
4. Meeting classification recognizes partner attendees. If a meeting attendee's email domain matches a partner account's domain, the meeting's entity context reflects the partner relationship. Verify with a real meeting that includes a known partner attendee.
5. A DB migration sets all existing accounts without an explicit `account_type` to `customer` as the default. Verify: `SELECT count(*) FROM accounts WHERE account_type IS NULL` returns 0 after migration.
6. Existing customer and internal accounts are unaffected. `cargo test` passes.

## Dependencies

- Required by I383 (AccountsPage three-group layout needs the type enum to exist).
- Informs I384/I393 (partner accounts should not render portfolio sections regardless of whether they have children — partner ≠ portfolio holder).
- See ADR-0087 (Entity Hierarchy Intelligence) — decision 1.

## Notes / Rationale

Partner intelligence has a fundamentally different shape from customer intelligence. A partner isn't tracked by renewal risk and spend — they're tracked by deliverable alignment, escalation health, and communication cadence. Lumping partners with customers produces intelligence that uses the wrong vocabulary and surfaces the wrong questions before a meeting.
