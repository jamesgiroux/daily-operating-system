# I423 — Gravatar writeback + propagation

**Status:** Open
**Priority:** P1
**Version:** 0.13.9
**Area:** Backend / Gravatar

## Summary

Gravatar stores 125 avatar images in `~/.dailyos/avatars/` and the `gravatar_cache` table, but never writes back to the `people.photo_url` column. Additionally, the `profile_discovered` signal uses plain `emit_signal` with no propagation to linked accounts. This issue: write `photo_url` back to the people table when Gravatar returns a result and no higher-priority source has claimed the field, and upgrade signal emission to `emit_signal_and_propagate` so that profile discoveries ripple out to accounts.

## Acceptance Criteria

1. When Gravatar returns a result (`has_gravatar = true`) for a person who has `people.photo_url = NULL` (or whose `photo_url` was not set by a higher-priority source like Clay), `people.photo_url` is updated with the Gravatar avatar URL after the cache write. Verify: identify a person in the DB with `photo_url IS NULL` who has an email with a known Gravatar. Run `start_gravatar_bulk_fetch`. After processing: `SELECT photo_url FROM people WHERE id = '<id>'` returns a non-null value.

2. Gravatar respects the source priority system. It does NOT overwrite a `photo_url` that was set by Clay (source priority 3 > Gravatar priority 2). Verify: a person whose `photo_url` was set by Clay retains their Clay-sourced photo after a Gravatar sync.

3. `profile_discovered` signals now use `emit_signal_and_propagate` (not plain `emit_signal`). Verify: after Gravatar processes a person linked to an account, `signal_events` contains a derived signal for that account from propagation.

4. `SELECT count(*) FROM signal_events WHERE source = 'gravatar'` increases after running a Gravatar sync (previously 23 rows — should grow as new profiles are discovered and existing ones propagate).

## Dependencies

None. Gravatar is independent, though the source priority system must respect Clay (I422) when both connectors are enabled.

## Notes / Rationale

**Key files:**
- Source priority system in `src-tauri/src/processor/enrich.rs` or equivalent — user=4, clay=3, gravatar=2, ai=1

**Rationale:**
Gravatar is a low-latency, free source of profile images. By writing back to `people.photo_url` and respecting the source priority system, Gravatar becomes useful for entity enrichment without overwriting higher-confidence sources. The `profile_discovered` signal is the point where a new email/person is found in an account's contact list — by propagating this signal, accounts get the metadata benefits (avatar, new contact) without needing to re-run enrichment.

**Data quality:**
The writeback must respect source priority to ensure that AI or Clay (higher confidence) sources never get overwritten by Gravatar. This prevents data regressions when multiple enrichment sources are active.
