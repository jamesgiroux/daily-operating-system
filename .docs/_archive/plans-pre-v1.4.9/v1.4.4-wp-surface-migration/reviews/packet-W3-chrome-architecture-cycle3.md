# L0 Architecture Review — Packet W3 Chrome Lane (Pulled Forward) — Cycle 3

**Reviewer:** `compound-engineering:ce-architecture-strategist`
**Mode:** Pattern compliance + design integrity (architect-reviewer per L0 matrix)
**Date:** 2026-05-19
**Packet under review:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` V1.2
**Cycle-2 verdict reviewed:** `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md` (APPROVE)

## Verdict: **APPROVE**

V1.2 strengthens the architectural posture (canonical pre-lift upgrade, source-of-truth invariant, sharpened Pill scope, AC #31 sequencing). Nothing introduced regresses the cycle-2 APPROVE; in fact, the source-of-truth invariant at §10 makes the substrate boundary tighter than V1.1 had it.

---

## 1. Focus-area assessment

### F1 — §5.4.1 canonical pre-lift upgrade vs Tauri-UI-freeze memory

**Sound. Freeze does not apply.**

- Memory `feedback_tauri_ui_freeze` reads (verified at `/Users/jamesgiroux/.claude/projects/-Users-jamesgiroux-Documents-dailyos-repo/memory/feedback_tauri_ui_freeze.md:10`): *"No new Tauri React UI work in any version from 2026-05-15 forward. Existing Tauri surfaces remain in stasis."*
- §5.4.1 edits two files: `.docs/design/reference/_shared/styles/FolioBar.module.css` and `.docs/design/reference/_shared/chrome.js`. Both are **mockup substrate / canonical design-system source**, not React, not TSX, not under `src/`, and not consumed by the Tauri runtime app's render path. They power `.docs/design/reference/surfaces/*.html` reference pages and are the canonical source for the WP chrome lift.
- The React production component `src/components/ui/folio-refresh-button.tsx` is **NOT touched** by §5.4.1. Tauri React UI is in stasis as required.
- The packet's narrative at lines 295 + 321 ("React production component at `src/components/ui/folio-refresh-button.tsx` already uses class-shaped React state, so canonical alignment improves Tauri parity") is slightly misleading — the file actually uses inline `style={{...}}` + `onMouseEnter`/`onMouseLeave` handlers (verified `src/components/ui/folio-refresh-button.tsx:38-62`). However, this is a narrative inaccuracy, not an architectural defect: the canonical upgrade lands cleanly regardless of React-component state, and the freeze policy bars touching that file (which the packet correctly does not propose). Optional V1.3 sharpening: rewrite the rationale to drop the misleading "already structured to support class-based refactor" claim and ground it solely in canonical mockup substrate improvement.

### F2 — §10 "Canonical source-of-truth for chrome assets" invariant

**Sound long-term; does not overconstrain.**

- The invariant (V1.2 §10, packet line 418) forbids WP-only module-CSS overlays and routes chrome improvements through canonical first. This is the correct architectural shape because:
  1. The single source-of-truth is the chrome lane's load-bearing claim (memory `feedback_check_substrate_before_authoring_primitives`). Allowing WP-only CSS divergence creates a second chrome substrate that drifts from Tauri.
  2. The escape valve is named: `patch-chrome-js.py` handles JS-only WP-specific patches (iframe reload guards, etc.). Patches 1-9a are JS-only WP-specifics that have no Tauri analog — they should NOT round-trip canonical. The invariant correctly carves out chrome.js patches and forbids only module-CSS overlays.
  3. §5.4.1 establishes the canonical-first precedent for class/visual changes that both surfaces benefit from. Future legitimate WP-only visual needs (if any emerge) would land via L0 amendment + new packet — the same gate as runtime-injection scope expansion (V1.1 invariant). Symmetric treatment.
- Risk vs reward: the cost of routing a future hypothetical WP-only visual through canonical is at most one extra PR (the canonical edit). The cost of NOT having this invariant is unbounded chrome drift. Trade is correct.

### F3 — §10 Pill scope sharpening

**Coherent.**

- §1 (packet line 46) + §10 (line 412) consistently name Pill as a child primitive consumed INSIDE the 4 shell modules (NavIsland active-state, FolioBar status indicators, AtmosphereLayer watermark labels). The chrome-side `Pill.module.css` is lifted because the 4 shell modules import its selectors; Pill itself is NOT runtime-injected as a top-level chrome element.
- The dual-namespace coexistence (`.Pill_*` chrome / `.dailyos-pill*` Gutenberg block at `wp/dailyos/blocks/pill/`) is governed by AC #28 allowlist gate (`check-chrome-block-collision.sh`). The allowlist entry is a single explicit grandfather, with a Pill reconciliation Linear ticket filed against the maintenance project.
- Per `wp/dailyos/blocks/pill/` (verified exists, plugin-owned), the namespaces ARE collision-free in selectors today — the gate ensures they STAY collision-free as either side evolves.
- Memory `project_wp_block_custom_vs_core_strategy` ("many small blocks not few large ones") is preserved: Pill remains a Gutenberg block for body-content authoring; the chrome-side `Pill.module.css` is internal infrastructure to the 4-module shell, not a competing surface.

### F4 — AC #31 sequencing

**Sound.**

- AC #31 (packet line 383, §5.4.1 line 327-330) requires the canonical pre-lift upgrade to land as a separate PR BEFORE chrome lane Tier 1 sync. Sequencing rationale:
  1. If §5.4.1 landed concurrent with the chrome lane, the chrome lane's verbatim sync (`sync-chrome.sh`) would lift pre-upgrade canonical and the `.FolioBar_folioRefreshButton` class rule would be missing. AC #4 byte-equivalence would fail.
  2. Splitting as a separate PR keeps L2 review scope tight (2 files, ~35 LOC) and confines blast radius — if the canonical edit needs revision, the chrome lane is unblocked from working off the pre-edit canonical until upgrade lands.
  3. The pre-W3 ticket has its own L2 review (codex review + design-reviewer per §5.4.1 line 330), which is correct — design-reviewer because the focus-visible rule is a net new visual behavior on mockup substrate pages.
- One small omission worth surfacing (non-blocking): the pre-W3 PR should add a note pointing forward at `src/components/ui/folio-refresh-button.tsx` — when the Tauri freeze lifts and that component is migrated/decommissioned per memory `feedback_tauri_ui_freeze`, the React component should re-converge with the now-canonical class. This is L5 drift surveillance territory; recording here so it doesn't get lost.

### F5 — Cycle-2 verdict regression check

**No regression. V1.2 strictly strengthens.**

- Cycle-2 cited disjoint W4-coupling matrix, class-contract-not-DOM-order invariant, Patch 9 design. All preserved verbatim.
- V1.2's removal of Patch 9b (refresh-button class replacement) from `patch-chrome-js.py` REDUCES the WP-overlay maintenance surface — cycle-2 noted Patch 9b's overlay-vs-canonical trade was "honest" and "bounded"; V1.2 promotes the cleaner half (canonical upgrade) and keeps only the WP-specific half (DOM idempotency, Patch 9a) as overlay. This is the architecturally cleaner choice and aligns with the cycle-2 §2 observation that 9b *could* go upstream but was held overlay to minimize Tauri regression surface — V1.2 reassesses regression risk and finds it low (the canonical mockup substrate has no production-runtime dependency).
- AC #28 (Pill allowlist) sharpened to gate failure mode + reconciliation ticket — strictly stronger than V1.1's promotion stub.
- AC #30 (alias discipline) regex fix is a correctness improvement, not a scope change — fixes a real B3 issue (digit-suffixed tokens like `--color-spice-turmeric-10` missed by V1.1's regex).
- §10's new "Canonical source-of-truth" invariant tightens the substrate boundary the cycle-2 verdict approved.

---

## 2. K-in re-audit (cycle-3, V1.2)

`docs/solutions/` greped for: `chrome`, `canonical`, `overlay`, `tauri`, `freeze`, `wp-only`, `module-css`. No prior documented solutions or anti-patterns conflict with §5.4.1 canonical-first decision or §10 source-of-truth invariant.

`.docs/decisions/` — no new ADRs introduced since cycle-2; the 5 cited (0073, 0076, 0077, 0129, 0130) remain consumed not overridden. Verified.

K-in clean. No BLOCKED-with-cited-path.

---

## 3. File paths referenced

- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:22-25` — V1.2 canonical pre-lift edits
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:46` — Pill scope clarification
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:254-333` — §5.4.1 full canonical pre-lift upgrade
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:383` — AC #31 sequencing
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:412` — §10 Pill scope (V1.2 sharpened)
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md:418` — §10 canonical source-of-truth invariant (V1.2 NEW)
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/design/reference/_shared/chrome.js:192-204` — refresh-button site for canonical class replacement
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/design/reference/_shared/styles/FolioBar.module.css` — canonical destination for new `.FolioBar_folioRefreshButton` rule
- `/Users/jamesgiroux/Documents/dailyos-repo/src/components/ui/folio-refresh-button.tsx:38-62` — React production component (inline-style + onMouseEnter/Leave) — NOT touched by §5.4.1; freeze respected
- `/Users/jamesgiroux/Users/jamesgiroux/.claude/projects/-Users-jamesgiroux-Documents-dailyos-repo/memory/feedback_tauri_ui_freeze.md:10` — freeze memory text
- `/Users/jamesgiroux/Documents/dailyos-repo/wp/dailyos/blocks/pill/` — plugin-universe Pill block (AC #28 allowlist anchor)
- `/Users/jamesgiroux/Documents/dailyos-repo/.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W3-chrome-architecture-cycle2.md` — cycle-2 verdict (APPROVE)

---

## 4. Optional V1.3 sharpening (non-blocking)

These are reviewer-suggested improvements that do NOT block APPROVE. File as L1 nice-to-haves OR fold into V1.3 only if other reviewers also surface conditions:

1. **§5.4.1 rationale wording (F1):** Drop the "React production component already uses class-shaped React state" framing (inaccurate — verified inline-style). Replace with: "React production component (`src/components/ui/folio-refresh-button.tsx`) uses inline styles and `onMouseEnter`/`onMouseLeave` callbacks; canonical alignment lands first on mockup substrate, and a future Tauri-freeze-lift release would re-converge the React component with the now-canonical class-based pattern."
2. **§5.4.1 forward-pointer (F4):** Add a one-liner to §8 deferrals listing "React `folio-refresh-button.tsx` re-convergence with canonical class" — Tauri-freeze-lift release, L5 drift surveillance.

Neither is required for APPROVE.

---

## 5. Architecture summary

V1.2 cleanly resolves cycle-2's 3 NEW BLOCKERS via the architecturally-correct path (upstream to canonical, not WP-overlay), introduces a new §10 invariant that tightens the substrate boundary, sharpens Pill scope, and sequences the canonical pre-lift as a separate PR before chrome lane sync. The Tauri-UI freeze does not apply because §5.4.1 touches mockup substrate (`.docs/design/reference/_shared/`), not the React component. Cycle-2 APPROVE holds; V1.2 strictly strengthens.

**Verdict: APPROVE.** Cycle-3 closes from this reviewer.
