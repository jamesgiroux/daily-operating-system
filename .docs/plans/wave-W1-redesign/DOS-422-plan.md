# DOS-422 — SignalDot primitive (W1)

**Status:** scout impl — driving end-to-end as the W1 contract-validation probe.

**Acceptance:**
- [ ] `SignalDot.tsx` + `SignalDot.module.css` ship at `src/components/dashboard/`
- [ ] Component consumes `MovingSignalViewModel` from `src/types/briefing.ts` (no internal types)
- [ ] All 8 `kind` values render with the right `--color-signal-*` token
- [ ] `urgency: "overdue"` flips text to terracotta
- [ ] `correctionState: "corrected" | "contested"` renders the outline ring
- [ ] `threadAction` is a `<button>` that stops event propagation
- [ ] `pnpm tsc --noEmit` clean
- [ ] Vitest unit test covers all 8 kinds + 3 lifecycle states + threadAction event-stop

## Contract-fit issue caught (rev 1)

The design-system spec at `.docs/design/primitives/SignalDot.md:8` lists variants in **camelCase** (`gongCall`, `zendeskTicket`, `slackThread`, `linearIssue`).

The wire contract at `src/types/briefing.ts:332-340` uses **kebab-case** (`"gong-call"`, `"zendesk-ticket"`, `"slack-thread"`, `"linear-issue"`).

**Resolution:** the component's prop type is `SignalDotKind` (kebab-case wire literals) — the contract type is the source of truth. The CSS Module class names are camelCase per project convention (`SignalDot_gongCall`); the component maps `kind` → class internally via a small lookup. The spec's `Variants:` field is informational discoverability only; the actual prop API is `SignalDotKind`.

This reconciliation gets folded into `.docs/design/primitives/SignalDot.md` as a "Wire vs class-name" note before W1 closes.

## Files this lands

```
src/components/dashboard/
  SignalDot.tsx                ← new, ~80 LOC
  SignalDot.module.css         ← new, ~120 LOC
  SignalDot.test.tsx           ← new, ~80 LOC
.docs/design/primitives/
  SignalDot.md                 ← edit: add wire-vs-class note + Status: integrated
```

## Imports

From `src/types/briefing.ts`:
- `MovingSignalViewModel` (the prop shape with TrustMixin + LifecycleMixin flattened in)
- `SignalDotKind` (8 kebab-case literals)

No other imports from contract or app. Component is self-contained per primitive convention.

## Render shape (mirrors spec anatomy)

```tsx
<span
  className={`${styles.SignalDot} ${kindClass} ${urgencyClass} ${lifecycleClass}`}
  data-kind={signal.kind}
  data-ds-name="SignalDot"
  data-ds-spec="primitives/SignalDot.md"
>
  <span className={styles.SignalDot_dot} />
  <span className={styles.SignalDot_when}>{signal.when}</span>
  <span className={styles.SignalDot_what}>
    {signal.whatSegments.map((seg, i) =>
      seg.emphasized
        ? <em key={i}>{seg.text}</em>
        : <Fragment key={i}>{seg.text}</Fragment>
    )}
  </span>
  {signal.threadAction && (
    <button
      className={styles.SignalDot_threadAction}
      onClick={(e) => { e.stopPropagation(); window.location.href = signal.threadAction!.href; }}
      data-ds-name="SignalDot.threadAction"
    >
      {signal.threadAction.label}
    </button>
  )}
</span>
```

CSS Module class layer:
- Base `.SignalDot` — grid `12px 70px 1fr auto`
- Per-kind `.SignalDot_meeting`, `.SignalDot_action`, `.SignalDot_email`, `.SignalDot_lifecycle`, `.SignalDot_gongCall`, `.SignalDot_zendeskTicket`, `.SignalDot_slackThread`, `.SignalDot_linearIssue` — each `.SignalDot_dot { background: var(--color-signal-*) }`
- `.SignalDot_overdue` — what text terracotta
- `.SignalDot_corrected` — solid sage outline ring on the dot
- `.SignalDot_contested` — dashed terracotta outline ring on the dot

## What this scout verifies

1. **Contract is renderable.** `MovingSignalViewModel` carries everything the rendering needs — no extra props, no derived data needed at consumer level.
2. **Wire kind ↔ CSS class mapping is sound.** Kebab-case input → camelCase class lookup is one stable utility, not a per-component pattern that grows.
3. **TrustMixin / LifecycleMixin via flatten works at consumer.** The TS interface inheritance reaches the React prop cleanly.
4. **The 8 token aliases (`--color-signal-*`) actually resolve.** Reference HTML for the surface (`briefing-redesign.html`) lists these tokens; the W0 token-sync caught the missing aliases on the mirror, but this is the first runtime use.

## Out of scope for this ticket

- `MovingRow` integration — that's DOS-423 (W3)
- The thread-action navigation behavior — `window.location.href` is a placeholder; `MovingRow` wraps the row in a router link and threadAction navigates within it. The event-stop is the only contract here.
- Storybook / visual-regression scaffolding — defer to a global tooling ticket.

## L1 self-validation gates

- `pnpm tsc --noEmit` clean
- `pnpm test src/components/dashboard/SignalDot.test.tsx` green (8+ tests)
- Manual visual check via reference HTML once the audit-reference manifest gets the new TSX path (W1 lifts `_pending_implementation` flag)

## L2 reviewers

- code-reviewer subagent — diff review on the 3 new files + spec update
- codex review or adversarial-review — focus on wire kind ↔ class lookup correctness

L0 is a one-pass plan (scout pace). If reviewers flag structural issues, those become a class-level finding to apply across the rest of W1.
