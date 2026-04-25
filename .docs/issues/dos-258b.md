# DOS-258b — Email-signature inference signal

**Parent:** DOS-258 (entity linking rewrite)
**Escalated from:** `dos-258-evidence-hierarchy-fix` branch, tier 4.
**Status:** Not started.

## Why this exists

The DOS-258 engine today matches emails to accounts via:
- `account_domains` lookup (sender/to/cc domain → account)
- Thread-parent inheritance (P2)
- Title keyword/slug match (P5)

Missing signal: email-signature content. When an external sender writes "Best, Alex — Test Software" in their signature block, that company name is strong evidence for the Jane account. Today the engine ignores the body entirely; it only reads the envelope (`from`, `to`, `cc`, `subject`). A new hire at Jane sending their first email from `@gmail.com` has no domain link, no thread history, and no keyword match — but their signature says "Test Software" and a human reader would correctly tag the email as Jane.

This was discovered during the v1.2.1 post-ship review when the engine mis-linked a meeting with a `@example.test` attendee to "WordPress VIP" based on title alone. The meeting fix landed via `P4a` stakeholder-inference (`dos-258-evidence-hierarchy-fix`). Emails from unknown senders remain blind.

## Scope

### Signal contract

New signal source value: `"signature_hint"`.
New signal type: `signal_events.signal_type = "entity_signature_hint"` (follow the existing signal bus conventions — see `src-tauri/src/signals/bus.rs`).
Payload: `{ candidate_account_id, matched_text, confidence }`.
Confidence: fixed 0.50 by default. Heuristic-based, below domain-match tier (0.80–0.95) but equal to title-slug (0.50).

### Producer

Hook: email ingestion path (`src-tauri/src/services/emails.rs` or `src-tauri/src/prepare/email_enrich.rs` — whichever runs after body is loaded, before entity linking fires). Runs only when the envelope provides no stronger signal (no domain match, no thread parent).

Logic:
1. Extract the last ~500 characters of the plaintext body as "signature zone." Real signature parsing (libsigcheck/forest) is a separate effort; a substring sweep of the tail is a working heuristic.
2. Load known account names + slugs + aliases from `accounts` + `entity_keywords` at query time (cache per-ingest-batch).
3. Search the signature zone for whole-word matches (≥ 4 chars, case-insensitive, same stoplist P5 uses).
4. If exactly one account matches, emit `entity_signature_hint` against that email. If multiple match, emit nothing (ambiguous).

### Consumer

New rule in the P4 family (suggested slot: `P4e` — runs after the domain rules, before P5). File: `src-tauri/src/services/entity_linking/rules/p4e_signature_hint.rs`.

Inputs: reads `signal_events` filtered to `signal_type = "entity_signature_hint"` for the current email owner.
Confidence: 0.72 (above title-slug 0.50, below domain rules 0.80+). Treat as "domain-adjacent evidence" so it plays nicely with P9 multi-match dispatching.
Active only for `owner_type IN ('email', 'email_thread')`. Meeting owners don't have bodies to parse.

Participates in `collect_p4_candidates` so multi-account signature hints feed the P9 picker instead of electing a wrong primary.

### Tests

1. `p4e_signature_hint_single_account_elects_primary`
2. `p4e_signature_hint_multi_account_feeds_p9`
3. `p4e_signature_hint_meeting_owner_is_skip`
4. `signature_producer_ignores_envelopes_with_domain_match` (don't duplicate work)
5. `signature_producer_handles_malformed_bodies_gracefully`
6. `p4e_confidence_below_domain_above_title`

## Non-goals (explicitly)

- Full signature-block parsing (the libsigcheck approach). Deferred until we see whether the heuristic is enough.
- Phone number / social handle extraction.
- Signature-based person disambiguation (linking to a specific person_id, not just an account).
- Real-time re-ingestion of historical emails. Forward-looking only; an optional backfill command can come later.

## Acceptance

- New senders from unknown domains whose signature clearly names a known account get correctly linked 80%+ of the time on a sampled batch of real inbox data.
- No regression in the "sender domain is known" path — that still wins.
- Ambiguous signatures (multi-account match) surface the P9 picker rather than picking wrong.

## Where to start

1. Read `src-tauri/src/services/entity_linking/rules/p4d_sender_domain.rs` as a parallel rule template.
2. Read `src-tauri/src/signals/bus.rs` for signal emission conventions.
3. Read `src-tauri/src/prepare/email_enrich.rs` to find the right ingestion hook.
4. New rule `p4e_signature_hint.rs` is structurally a twin of P4d — collect account_ids from stored signals instead of querying domain lookup.

## Related

- DOS-258 main: `.docs/plans/DOS-258-entity-linking-rewrite.md`
- Evidence hierarchy rework: branch `dos-258-evidence-hierarchy-fix`, commits `535f0b90…44b15307`.
