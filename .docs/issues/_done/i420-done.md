# I420 — Stakeholder–Person Reconciliation — Canonical Name Resolution and Deterministic Linking

**Status:** Open (0.13.3)
**Priority:** P1
**Version:** 0.13.3
**Area:** Backend / Intelligence + Frontend / Entity

## Summary

The AI enrichment pipeline produces `stakeholderInsights` in `intelligence.json` with names it extracts from meeting notes, email context, and signals. These names are often informal or abbreviated ("James G.", "J. Giroux", "Jim") while the same person may already exist as a Person entity ("James Giroux") linked to that account. Today, the only matching is a UI-side exact case-insensitive name comparison — which misses abbreviations, nicknames, and typos. The user has to manually reconcile these, which is maintenance they shouldn't have to do.

This issue adds two-phase stakeholder reconciliation to the enrichment pipeline: (1) prompt-side context injection so the AI uses canonical names for known contacts, and (2) post-enrichment fuzzy reconciliation that matches remaining stakeholder entries against linked Person entities using multiple signals and writes a deterministic `personId` into the stakeholder insight.

## Architecture

This aligns with ADR-0086: the account detail page is a **consumer** of `intelligence.json`. Reconciliation belongs in the enrichment pipeline (producer), not in the UI (consumer). After reconciliation, `intelligence.json` is the source of truth for stakeholder–person links, not a UI-side name match.

### Phase 1 — Prompt-side canonical name injection

In `build_intelligence_context()` (`intelligence/prompts.rs`), when building context for an account entity, include the list of linked Person entities:

```
## Known Contacts for This Account
- James Giroux (role: VP Customer Success, email: james@example.com)
- Sarah Chen (role: Account Executive, email: sarah.chen@example.com)

When referencing people in your stakeholder analysis, use their canonical name
exactly as listed above. Do not create separate entries for known contacts
using nicknames, abbreviations, or partial names.
```

This gets the AI to use canonical names for ~80% of cases.

### Phase 2 — Post-enrichment fuzzy reconciliation

After the AI response is parsed in `try_parse_json_response()` but before `intelligence.json` is written, run a reconciliation pass:

1. For each `StakeholderInsight` produced by the AI, score it against all Person entities linked to the account using multiple signals:
   - **Name similarity** (0.0–1.0): Token overlap + Levenshtein distance. "James G." → "James Giroux" scores ~0.7. "Jim Giroux" → "James Giroux" scores ~0.5.
   - **Role match** (0.0–0.3 bonus): If the stakeholder's role is semantically similar to the person's role.
   - **Meeting co-attendance** (0.0–0.2 bonus): If the person has appeared as an attendee in meetings tagged to this account.
   - **Email domain match** (0.0–0.1 bonus): If the person's email domain matches the account's domain.

2. **Confidence thresholds:**
   - **≥ 0.8 — auto-link:** Replace the AI stakeholder name with the Person entity's canonical name. Set `personId` on the stakeholder insight. This is a deterministic link — the UI no longer needs to guess.
   - **0.5–0.8 — suggested link:** Set `suggestedPersonId` on the stakeholder insight. The UI shows a subtle confirmation prompt ("Is this James Giroux?"). One click to confirm, which promotes `suggestedPersonId` → `personId`.
   - **< 0.5 — unlinked:** Leave as-is. Genuine new contact. The existing "Create contact" hover action remains.

3. **User edit protection:** If a user has manually edited a stakeholder's name (tracked via `userEdits` in `intelligence.json`), do not overwrite it with a reconciliation match. User corrections are authoritative.

## Acceptance Criteria

Each criterion verified with real data in the running app after a full `pnpm dev` restart.

1. **Prompt includes known contacts:** Trigger an enrichment on an account that has 3+ linked Person entities. Read the enrichment prompt (via PTY log or debug output). The prompt contains a "Known Contacts" section listing each linked person's canonical name, role, and email.

2. **AI uses canonical names:** After enrichment, open the account's `intelligence.json`. Stakeholder entries for linked people use the exact canonical name from the Person entity, not an abbreviation or nickname. Verify with a person whose meeting-note name differs from their entity name.

3. **`personId` on auto-linked stakeholders:** A stakeholder insight that matches a linked Person entity at ≥ 0.8 confidence has a `personId` field in `intelligence.json`. Verify: `cat intelligence.json | jq '.stakeholderInsights[] | select(.personId != null)'` returns at least one entry for an account with linked people.

4. **`suggestedPersonId` on uncertain matches:** A stakeholder insight that matches at 0.5–0.8 confidence has a `suggestedPersonId` field. The frontend renders a subtle confirmation prompt for this stakeholder. Clicking "Yes" promotes the suggestion to a confirmed `personId` and persists via `update_intelligence_field`.

5. **Unlinked stakeholders unchanged:** A stakeholder with no match (< 0.5) has neither `personId` nor `suggestedPersonId`. The existing "Create contact" hover action still appears. No regression on unlinked stakeholder behavior.

6. **Frontend uses `personId` for deterministic matching:** The `StakeholderGallery` uses `s.personId` (when present) instead of name-based matching against `linkedPeople`. The larkspur ring and link icon render based on `personId`, not name comparison. Name-based matching remains as a fallback for stakeholders without `personId` (backward compat with existing intelligence.json files).

7. **User edits protected:** Manually edit a stakeholder's name in the UI. Trigger re-enrichment. The edited name is preserved — reconciliation does not overwrite it.

8. **No false positives:** Enrichment on an account with a linked person named "John Smith" does not auto-link an unrelated stakeholder named "John S." who has a different role and appears in different meetings. The multi-signal scoring prevents low-quality matches from being promoted.

## Dependencies

- Builds on I382 (partner entity type) — partner accounts have stakeholders too.
- Consumes `linkedPeople` from account detail (already exists in `get_account_detail`).
- Extends `StakeholderInsight` struct in `intelligence/io.rs` (adds `personId`, `suggestedPersonId`).
- Frontend changes in `StakeholderGallery.tsx` (use `personId` for matching, render suggestion prompt).

## Notes / Rationale

Every manual reconciliation the user does is a signal that the pipeline failed to deliver clean data. The user managing 50 accounts with 5–10 stakeholders each should never have to click "this is the same person" — that's the system's job. Prompt-side injection handles the common case cheaply (the AI already has the canonical names in context). Post-enrichment reconciliation catches the edge cases where the AI used a different name despite the hint, or where a stakeholder appears for the first time and might match an existing person. The confidence-tiered approach avoids false positives while still reducing manual work to near-zero for established relationships.
