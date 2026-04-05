# I198 — Account Merge + Transcript Reassignment

**Status:** Open (Parking Lot)
**Priority:** P2
**Version:** Unscheduled
**Area:** Entity

## Summary

When a user has two account records that represent the same company (duplicates from different sources, name variations, or acquisitions), they need a way to merge them into one canonical account. Merging is destructive — it must transfer all associated data (meetings, actions, people links, intelligence, transcripts, files) from the source account to the target account before deleting the source.

Transcript reassignment is the hardest part: transcripts stored in the filesystem under the source account's directory need to be moved to the target account's directory, and all DB references (captures, actions, meeting_entities) need to update accordingly.

## Acceptance Criteria

### Already Implemented
- [x] Merge UI on account detail page (AccountMergeDialog + "Merge Into..." button in AccountAppendix)
- [x] Backend `merge_accounts` command with signal emission
- [x] DB merge transfers: actions, meeting_entities, people links, account_team, account_events, signals, content_index, account_domains, child accounts
- [x] Source account archived after merge
- [x] MergeResult returned with counts (actions, meetings, people, events, children moved)

### Remaining Work (Transcript Reassignment)
- [ ] Update `meetings_history.transcript_path` references during merge (repoint from source to target account path)
- [ ] Filesystem operation: move transcript files from `_today/accounts/{from_id}/` to `_today/accounts/{into_id}/`
- [ ] Update any capture records that reference transcripts from the source account
- [ ] Test: merge two accounts where source has transcripts, verify transcripts accessible under target post-merge

## Dependencies

- Related to I199 (archived account recovery) — both deal with account lifecycle management.
- The people merge (I170, Sprint 11) serves as a reference implementation for the cascade merge pattern.

## Notes / Rationale

Parked because account merges are rare in practice (most users maintain distinct account records). The people merge (I170) was prioritized first because duplicate person records are far more common (created automatically from calendar events). Account merge is needed eventually but is not blocking any user-visible workflow.
