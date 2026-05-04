# I383 — AccountsPage Three-Group Layout — Your Book / Your Team / Your Partners

**Status:** Open (0.13.3)
**Priority:** P1
**Version:** 0.13.3
**Area:** Frontend / UX

## Summary

The AccountsPage currently shows all accounts in a single undifferentiated list. With three account types (customer, internal, partner), the page should organize accounts into three named groups: **Your Book** (customers — what you sell to and are responsible for), **Your Team** (internal accounts — cross-functional relationships), and **Your Partners** (partner accounts — agencies, SIs, channel partners). Empty groups don't render. Existing parent/child hierarchy is preserved within each group.

## Acceptance Criteria

From the v0.13.3 brief, verified in the running app:

1. The AccountsPage renders three named sections: **Your Book** (customer accounts), **Your Team** (internal accounts), **Your Partners** (partner accounts). The section names match exactly.
2. Each account appears in exactly one section, determined by its `account_type`. An account does not appear in more than one section.
3. Sections with zero accounts do not render. If the user has no partner accounts, "Your Partners" is not shown.
4. The existing parent/child hierarchy is preserved within each section. A parent customer account still shows its child accounts nested beneath it within "Your Book."
5. Search applies across all three sections simultaneously. Searching for an account name returns the result regardless of which section it's in.
6. The page uses the existing design system — no new layout patterns. Section headers use the existing editorial section rule/header treatment.

## Dependencies

- Blocked by I382 (partner entity type must exist before grouping by type is meaningful).
- See ADR-0087 decision 2.

## Notes / Rationale

"Your Book / Your Team / Your Partners" mirrors how people actually categorize their work relationships — not how database schemas organize account records. The grouping makes the AccountsPage navigable at a glance: a user scanning for a customer goes to "Your Book"; looking for the agency they work with goes to "Your Partners." The current flat list requires the user to remember all their account names and scan for them.
