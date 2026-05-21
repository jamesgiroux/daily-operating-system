# L0 Cycle-1 verdict — v1.4.4 wave-plan packet (codex-consult, narrow-scope verify)

**VERDICT: REVISE.** Three citations in the packet are off by a small but load-bearing margin: the sha range is exclusive of two tickets the packet enumerates, one cited section of ADR-0129 is ambiguous, and the K-in keyword pass is missing one ADR. ADR cite accuracy itself (ADR-0130 §2/§4/§5) is clean.

Scope: panel covered the rest; this pass verifies only (1) chrome-lane sha range, (2) K-in completeness against the named keyword set, (3) ADR cite accuracy.

---

## Findings

### F1 — HIGH — chrome lane sha range is **exclusive of DOS-729**, and DOS-721 sits **outside the range entirely**

§1 line 20: "Pulled-forward work already shipped on this branch (sha range `771a3d5d..26062496`)" enumerating DOS-721, DOS-729, DOS-730, DOS-731, DOS-732, DOS-724, DOS-722.

Evidence:
- `git log 771a3d5d..26062496` (two-dot, exclusive lower bound) does NOT include 771a3d5d itself, which IS the DOS-729 chrome lane Tier 1 commit (`feat(theme): chrome lane Tier 1 — tokens + alias layer (DOS-729)`).
- DOS-721 lives at sha `52d25db5` (2026-05-19, `feat(canonical): refactor FolioBar refresh button inline style to class`), which is BEFORE 771a3d5d. DOS-721 is not in `52d25db5..26062496` either as written — `git log 771a3d5d..26062496` does not surface it at all.
- DOS-722, DOS-724, DOS-730, DOS-731, DOS-732, plus the chrome-lift commit `0342bf7a` and theme.json wiring `26062496` (DOS-336) all DO land in the range as written.

Recommended fix: change the cited range in §1 line 20 and §12 line 363 to `52d25db5^..26062496` (inclusive of DOS-721) OR list DOS-721 + DOS-729 as the bounding shas and use `52d25db5..26062496` with explicit note that DOS-729 (`771a3d5d`) is included. Cleanest: `52d25db5..26062496 inclusive` and drop the misleading two-dot form.

### F2 — LOW — `26062496` is DOS-336, not chrome lane

§12 line 363 cites "`26062496` (DOS-721, DOS-729, ..., plus chrome lane commit `0342bf7a` and theme.json wiring `26062496`)" — the line reads as if `26062496` is chrome-lane work. It is actually `fix(theme): wire layout + spacing.padding into theme.json generator (DOS-336)`, which is theme.json infrastructure (related to magazine theme from v1.4.3 W6), not chrome lane. Reword §12 line 363 to call this out: "theme.json generator wiring `26062496` (DOS-336, magazine-theme follow-up, landed in same window)."

### F3 — MEDIUM — K-in keyword pass missing ADR-0132 Pill primitive dual existence

Per the verify prompt, K-in re-grep on `composition / composable / block-theme / gutenberg / inner-blocks / parity-proof / tauri-deprecation / flag-flip` should surface every ADR the packet ought to reference.

Evidence: `grep -rli` on `.docs/decisions/` for the keyword set returns `0132-pill-primitive-dual-existence.md` (matched on `composition`/`gutenberg`-adjacent content). The packet references DOS-722 chrome lane Pill duality at lines 27 + 246 but does NOT cite ADR-0132 in §3 K-in or §12 References. ADR-0132 is the durable contract for the chrome `.Pill_*` vs block `.dailyos-pill*` split that §10 invariant rows (lines 314–315) implicitly depend on.

Recommended fix: add ADR-0132 to §3 ADR enumeration (line 57) and §12 References. K-in keyword set (`composition`, `gutenberg`) also surfaces ADR-0099 / ADR-0108 / ADR-0125 / ADR-0131 — those ARE already cited or covered. ADR-0132 is the lone gap.

No `docs/solutions/` gap. The 6 referenced entries (lines 48–53) cover the workflow-issues directory comprehensively for the keyword set; no entries omitted.

### F4 — INFO — ADR cite accuracy clean (no finding)

Packet cites:
- ADR-0129 §1 "DailyOS is a runtime; surfaces are clients" — verified line 46–52 of `0129-composable-surfaces-wordpress-studio-as-primary-surface.md`. Citation accurate.
- ADR-0130 §2 `Composition` model — verified lines 45–107 (struct + ProvenanceRef + Salience). Citation accurate.
- ADR-0130 §4 renderer-not-author boundary — verified lines 148–162 ("Surface bindings — renderers, not authors"). Citation accurate.
- ADR-0130 §5 — packet line 13 says "§5 authorship boundary." Verified lines 164–175 ("Authorship boundary — abilities produce compositions"). Citation accurate.
- ADR-0130 §3 BlockType taxonomy — verified lines 109–146. Citation accurate.

No fix needed.

### F5 — INFO — §3 secondary claim "no documented substrate reinvented" holds

Verdict line 61 ("Verdict: K-in complete. No documented substrate reinvented") survives the re-grep. Every named v1.4.4 producer (envelope DOS-459, touchpoints DOS-460, get_daily_briefing, meeting prep DTO DOS-335, Receipt DTO DOS-339, claim review queue) is either named in a prior wave's substrate or scheduled as a W1 gap. No reinvention.

---

## Net

Three fixes (F1 + F2 + F3) all in §1 line 20 / §3 line 57 / §12 line 363 / §12 References. Mechanical edits, ~15 min. Re-issue cycle 1 verdict as APPROVE after the sha-range correction lands; the ADR cite set is otherwise clean.
