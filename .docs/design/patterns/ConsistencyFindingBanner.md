# ConsistencyFindingBanner

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `ConsistencyFindingBanner`
**`data-ds-spec`:** `patterns/ConsistencyFindingBanner.md`
**Variants:** `state="corrected" | "flagged"`; `severity="low" | "medium" | "high"`
**Design system version introduced:** 0.2.0

## Job

Surface a v1.4.0 trust compiler consistency finding directly beside the affected claim. The banner explains whether the claim was corrected or flagged and gives the user the next actions without requiring a full receipt first.

## When to use it

- Inline with a `ClaimRow` when consistency metadata is `corrected` or `flagged`.
- When the trust compiler found contradiction, correction, or verification findings.
- In receipt expansion when a consistency finding needs higher visibility than normal metadata.
- When users need to confirm, inspect, correct, or dismiss the finding.

## When NOT to use it

- For ordinary trust banding with no consistency finding — use `TrustBand`.
- For complete receipt inspection — use `ReceiptCallout`.
- For broad source coverage or missing-data explanation — use `AboutThisIntelligencePanel` or `DossierSourceCoveragePanel`.

## Composition

Composes:

```
[VerificationStatusFlag] [finding text]
[Actions: Inspect / Correct / Dismiss]
```

`VerificationStatusFlag` leads with corrected or flagged state. Finding text is short and specific. Actions render only when the caller can handle them; otherwise the banner remains informational.

## States / variants

- **corrected** — previous value was automatically repaired; accent is caution, not error.
- **flagged** — unresolved finding requires user attention; accent is verification-needed.
- **low** — muted treatment; finding does not affect primary interpretation.
- **medium** — standard inline warning treatment.
- **high** — stronger border and action emphasis; used for unresolved contradictions.
- **action-pending** — disables actions while a correction, dismissal, or inspection flow starts.
- **dismissed** — hidden from normal view but still visible inside `ReceiptCallout`.

## Tokens consumed

- `--color-text-primary` — finding summary.
- `--color-text-secondary` — supporting evidence text.
- `--color-text-tertiary` — finding code or compiler metadata.
- `--color-rule-light` — banner border.
- `--color-spice-terracotta` — flagged accent.
- `--color-spice-terracotta-8` — high-severity border.
- `--color-trust-use-with-caution` — corrected accent.
- `--space-xs`, `--space-sm`, `--space-md` — flag, copy, and action spacing.
- `--font-mono` — finding code and action labels.

## API sketch

```tsx
<ConsistencyFindingBanner
  finding={{
    state: "flagged",
    severity: "high",
    code: "contradicts_recent_meeting",
    text: "Renewal risk conflicts with the latest meeting note.",
    evidenceText: "Customer said legal review is still open.",
  }}
  onInspect={() => openReceipt()}
  onCorrect={(finding) => startCorrection(finding)}
  onDismiss={(finding) => dismissFinding(finding)}
/>
```

## Source

- **Spec:** v1.4.0 trust compiler consistency finding contract
- **Related pattern:** `ClaimRow`
- **Related pattern:** `ReceiptCallout`
- **Mockup origin:** `.docs/_archive/mockups/claude-design-project/_audits/04-trust-ui-inventory.md`
- **Note:** Wave 2 follow-on for v1.4.4

## Surfaces that consume it

DailyBriefing chapter callouts, AccountDetail/ProjectDetail/PersonDetail intelligence panels, MeetingDetail findings sections.

## Naming notes

`ConsistencyFindingBanner` is the canonical inline finding pattern. Keep `Finding` in the name because the source of truth is compiler metadata, not a generic alert or validation error.

## History

- 2026-05-03 — Proposed pattern for Wave 2 (v1.4.4 trust UI substrate).
