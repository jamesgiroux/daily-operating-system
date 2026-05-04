# I414 — User-Context-Weighted Signal Scoring

**Status:** Open
**Priority:** P1
**Version:** 0.14.0
**Area:** Backend / Signals

## Summary

Signals are currently scored by confidence and temporal decay. This issue adds a third dimension: relevance to the user's stated priorities. When `current_priorities` contains "expand Crestview Media account," signals from Crestview Media-domain entities receive a relevance multiplier. When `value_proposition` mentions security compliance, security-related signals from accounts rank higher in the briefing attention section. This extends the email relevance scoring mechanism (I395) applied at the entity signal level, making the user's declared context a first-class input to signal prioritization.

## Acceptance Criteria

1. A `compute_user_relevance_weight(signal: &SignalEvent, user_entity: &UserEntity, db: &SqlitePool) -> f64` function exists in `signals/user_relevance.rs` (new file). The function returns a multiplier in the range [0.5, 2.0]: 1.0 = neutral (no user context match), > 1.0 = aligned with user priorities, < 1.0 reserved for future penalty cases. The function is a pure function (deterministic, no side effects).

2. The function computes relevance via two methods: (a) **keyword overlap** — extract topic keywords from `current_priorities` + `value_proposition` + `product_context`; compute overlap with signal `message` text and associated entity `domain`; high overlap (>3 keyword matches or >0.70 Jaccard similarity) → multiplier 1.5–2.0. (b) **embedding similarity** — embed the signal text and user context fields; compute cosine similarity; similarity > 0.75 → multiplier 1.3–1.8. Combine the two scores (average or max, documented in code). Default multiplier 1.0 when no match or when user entity fields are NULL.

3. The signal relevance scoring logic in `signals/scoring.rs` (from I395) is extended to call `compute_user_relevance_weight` and multiply the final signal score by the weight. The signal bus and scoring pipeline are unchanged; only the final score output is modified. Verify: log the weight calculation for each signal during a scoring cycle at DEBUG level, showing the keywords/similarity score and resulting multiplier.

4. The daily briefing attention section (from `services/dashboard.rs`) ranks signals using the weighted scores. Top signals now reflect both confidence AND alignment with user priorities. Verify with real data: set `current_priorities` to explicitly name a specific account (e.g., "expand Acme to 3 regions"). Trigger signal scoring for multiple entities including Acme and non-Acme entities. The Acme entity's signals should rank higher in the attention section than equivalent-confidence signals from other accounts, even if Acme's raw signal confidence is lower. Compare the daily briefing before and after setting priorities — the named account's signals appear first or near-first.

5. When `current_priorities` is NULL and all other user entity fields are NULL, the weight function returns 1.0 for all signals (neutral multiplier). No behavior change from pre-v0.14.0. Verify: clear all user entity fields, run a full signal scoring cycle, compare the briefing signal ordering to the pre-v0.14.0 baseline — the order is identical.

6. The user relevance weight is logged at DEBUG level for each scored signal, including: signal ID, entity, keyword matches (if any), embedding similarity (if computed), and final multiplier. This enables debugging and auditing of the user-context-weighted ranking.

## Dependencies

- Blocked by I411 (user entity must exist).
- Builds on I395 (email relevance scoring pattern; signal scoring infrastructure must exist).

Unblocks observable user-centric signal prioritization.

## Notes / Rationale

From ADR-0089 Decision 4: user context adds a third dimension to signal scoring. This issue implements the mechanism that makes priorities observable in the app — when a user declares that an account is a focus, DailyOS's daily briefing reflects that priority immediately via signal weighting. The dual approach (keyword + embedding) provides both lexical precision (if the user types "expand Acme," exact matches are high-weight) and semantic flexibility (if the signal talks about Acme's growth without using the word "expand," the embedding similarity still captures relevance). The [0.5, 2.0] range preserves the relative ordering of signals while making priority-aligned signals more prominent without completely suppressing lower-priority signals. The neutral default (1.0) and NULL handling ensure backward compatibility.
