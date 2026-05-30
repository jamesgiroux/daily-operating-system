# Codex Challenge — packet-W1-substrate-gaps cycle 1

**VERDICT: APPROVE WITH CHANGES** — DOS-701 reuse claim verifies on disk; IL checks have substance (not copy-paste); AC-W1.2 "wiring IS the work" gate is declared but mechanically unenforced — fixable with one CI script before W1 close.

Scope: adversarial pass on 3 vectors (DOS-701 reuse claim, IL-check fluff hunt on §5.2/§5.8/§5.10, AC-W1.1/W1.2 enforcement). Architecture/correctness/CSO already covered in sibling cycle-1 packets.

---

## Findings

### F1 — AC-W1.2 has no enforcement mechanism (HIGH)

**Section:** §7, AC-W1.2 + AC-W1.4 (lines 924, 926).

**Attack vector:** Wave acceptance gate.

**Claim under attack:** "No W1 producer ships without at least one downstream consumer skeleton in W2 (or W3/W4)…Each W1 PR must point to the W2/W3/W4 PR that will consume it."

**Finding:** The rule is declared as a manual reviewer assertion. AC-W1.4's CI fence (`cargo clippy && cargo test && pnpm tsc && pnpm test`) does not test producer-consumer pairing. DOS-461 (§5.3) gates W2 surfaces from bypassing the envelope; it does NOT gate W1 producers from landing without a consumer skeleton. The class-pattern hazard is concrete: a W1 PR can pass all of AC-W1.3/W1.4/W1.5 with an empty `wp/dailyos/blocks/account-detail/render-functions.php` and a hand-wave in the PR body that "W2 will consume." This is exactly the failure mode CLAUDE.md "wiring IS the work" + memory `feedback_wire_existing_substrate_not_future_producer` exists to prevent.

**Recommended fix:** Add **AC-W1.9** — `scripts/check_w1_consumer_skeleton.sh` (CI lint, modeled on `check_claim_writer_allowlist.sh` per §5.8 precedent): for each W1-shipped producer (`abilities-runtime/src/abilities/get_entity_intelligence/`, `get_daily_briefing/`, `services/meeting_prep_status/`, `services/entity_intelligence/touchpoints.rs`, filled placeholders in `services/claim_receipt/{boundary,contradiction,feedback,privacy,render_rules}.rs`), assert at least one block render PHP entry point under `wp/dailyos/blocks/**/render-functions.php` invokes it via the abilities runtime handle. Fails CI if any producer lands without a consumer. The script IS the work for the wiring obligation.

### F2 — DOS-701 reuse claim verifies; one minor accuracy correction (LOW)

**Section:** §6 substrate table line 892 ("`services::claim_receipt::{contracts, render, auth}` (319+290+175 LOC)").

**Attack vector:** Reuse claim integrity.

**Finding:** Disk audit confirms: `auth.rs` (12092B), `contracts.rs` (5517B), `render.rs` (11282B) all non-trivial. `boundary.rs`, `contradiction.rs`, `feedback.rs`, `privacy.rs`, `render_rules.rs` are 1-byte placeholders (correct per packet). However, line 892's LOC counts (319+290+175) don't match byte-size order (auth largest at 12KB, render next at 11KB, contracts smallest at 5.5KB). Either the LOC numbers are stale or the file/LOC mapping is mis-ordered. Doesn't change the reuse claim — DOS-701 substrate IS load-bearing — but the packet's specific LOC figures should match disk reality before L2.

**Recommended fix:** Run `wc -l src-tauri/src/services/claim_receipt/{contracts,render,auth}.rs` and update line 892 with current LOC counts in matching order; re-verify before L0 close.

### F3 — §5.8 IL check is thin but defensible (LOW)

**Section:** §5.8 IL check (lines 714–719).

**Attack vector:** IL-check boilerplate.

**Finding:** §5.2 (DOS-460) and §5.10 (DOS-507) IL checks answer each Q with ticket-specific substance (ClaimType::OpenLoop, WrongSubject routing, DOS-278 candidate-set invalidation, DOS-8 routing). §5.8 (DOS-340) IL check is structurally thin: Q1 "no claims written", Q3 "no new signals", Q5 "N/A". This is defensible for a pure render-side boundary filter, but functionally close to a fail-shape under AC-W1.3 ("substrate that fails any question is incomplete"). The packet should explicitly justify that "N/A for feedback loop" is acceptable for boundary-only substrate, OR re-frame Q5 to point at "violations of this boundary feed source reliability negatively via DOS-8 if a surface accidentally exposes audit-only fields" — which IS a real feedback path.

**Recommended fix:** Re-author §5.8 Q5 to: "Boundary violations detected by the §5.8 CI lint or by user reports of leaked audit-only fields feed back as DOS-8 `WrongSubject`/`SourceUnreliable` signals on the receipt's source — the boundary is itself a trust contract." Keeps the IL-check honest and the CSO panel happy.

### F4 — AC-W1.1 dependency map double-counts DOS-461 outside §5.3 scope (LOW)

**Section:** AC-W1.1, line 917 ("W2 Project Detail block → same set + DOS-461 harness green").

**Attack vector:** Wave acceptance gate scoping.

**Finding:** AC-W1.1 lists DOS-461 only as a Project-block prerequisite. But DOS-461's no-bypass harness gates Account / Project / Person equally (AC-461.1, AC-461.7). The packet's §5.3 (line 326) confirms harness gates all W2 entity surfaces; AC-W1.1 line 916 (Account) and line 918 (Person) omit DOS-461. Minor consistency bug — could cause a W2 reviewer to land Account or Person blocks while skipping the harness gate.

**Recommended fix:** Add `+ DOS-461 harness green` to lines 916 and 918 in AC-W1.1 to match line 917.

---

## Summary

DOS-701 reuse claim is real (vector 1). IL-checks on the 3 spot-checked sub-tickets are substantive, not boilerplate, with one thin section (§5.8) that should be re-authored (vector 2). The "wiring IS the work" acceptance gate (vector 3) is the only material gap — F1 is HIGH because the wave can mechanically pass all current AC criteria while shipping bare W1 producers, recreating exactly the failure mode the rule was written to prevent. Address F1 by adding AC-W1.9 + a CI script; F2–F4 are LOW polish items for L0 close.
