# W2 L0 codex challenge — cycle 1

Verdict: BLOCK

Findings:

- Trigger idempotency needs explicit failure/retry semantics. A unique `dedupe_key` plus pre-downstream insert can permanently coalesce an incomplete run after partial failure.
- Surface-specific suppression must reuse `claim_surface_dismissals`; reading only `claim_feedback` can re-render claims dismissed on a specific surface.
- Privacy plan must account for free-string fields in `WhyThisNow.text`, `TriggerRef.source`, and `signal_events.value`.
- Subject/action cooldown needs an auditable `action_signature` and tests, not an undefined semantic similarity rule.
- Mutation-path surfacing must reference persisted salience, not preview-only scores without a durable evaluation id.
- Migration slots should be contiguous from current `dev` v270 rather than skipping v271/v272/v274.

Cycle 2 packet changes: add retry statuses, surface dismissal reads, source/text sanitization rules, `action_signature`, durable salience recompute, and v271/v272 migration slots.
