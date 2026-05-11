# L2 (Diff) Architecture Review — DOS-546 W1-D

**Commit:** `57a57e1f` — ability description CI gate + W1-D/W1-C/W1-E.1 workflow integration
**Reviewer:** architect-reviewer
**Date:** 2026-05-11
**Verdict:** APPROVE (with two follow-up findings, both path-α → maintenance)

## 1. Implementation choice — Python in bash wrapper, not bash + grep

The commit message frames this as a bash + grep gate, but the implementation is a `python3 - "$@" <<'PY' ... PY` heredoc embedded in a bash entrypoint. This is the **correct** architectural call and is materially stronger than naive bash + grep would have been:

- Multi-line `#[ability(...)]` attributes are parsed by depth-tracking `(`/`)` with string-state awareness (`find_matching_paren`) — grep-based pattern matching over `.rs` would have produced false negatives on multi-line attrs, which is the dominant shape in `abilities-runtime` per ADR-0102 §7.
- Rust and JSON string literals are de-escaped via `ast.literal_eval` and `json.loads` before scanning, so terms split across escape sequences (`enrichment`) match the canonical form.
- Whole-word boundary regex (`(?<![A-Za-z0-9_])...(?![A-Za-z0-9_])`) is properly applied per-term with `re.IGNORECASE`. Bash + grep would have struggled with the multi-word terms (`AI enrichment`, `intelligence pipeline`, `trust band scoring`) because `\b` semantics differ across grep flavors.

The bash wrapper carries no real grep logic — it is a thin shim that resolves `ROOT_DIR` and the two blocklist paths and hands off to Python. **The commit message description undersells what landed**; "bash + grep" would have been architecturally inadequate for this contract. Recommend updating the W1-D issue/proof bundle to describe the implementation accurately. No code change needed.

## 2. Single-source-of-truth — `scripts/ability_description_vocab_blocklist.txt`

The seed list (8 terms: `enrichment`, `AI enrichment`, `intelligence pipeline`, `pipeline run`, `prompt fingerprint`, `claim writer`, `trust band scoring`) cleanly matches the **CLAUDE.md §Figma Visual Rules clause** (`enrichment`, `AI enrichment`, `intelligence pipeline`) and the most leak-prone runtime nouns from W1-C / W2-A / W2-E surface area. So the file is anchored to two real authorities.

**However, it is materially narrower than ADR-0083's canonical translation table.** ADR-0083 names a much larger set of system terms that "must not appear in user-facing text without translation" (§Two-Layer Vocabulary, lines 54–104): `entity`, `entity resolution`, `intelligence` (three context-dependent translations), `signal`, `signal bus`, `prep`, `prep file`, `proposed`, `archived`, `bayesian fusion`, `thompson sampling`, `correction learning`, `run briefing`, `refresh intelligence`. The W1-D blocklist contains **none** of those.

This is defensible — adding bare `entity` or `signal` would false-positive constantly in legitimate ability descriptions (`"Return entity context for a subject reference"` is the clean fixture and arguably itself violates strict ADR-0083). The pragmatic seed list trades coverage for false-positive rate.

**The architectural gap is the absence of a drift-detection mechanism.** There is no test or doc that establishes: "when ADR-0083's translation table or CLAUDE.md's pipeline-vocabulary list grows, this blocklist must be reviewed." A future amendment to ADR-0083 will not surface a CI signal that the blocklist is out of sync. **Recommend** filing a path-α maintenance issue: add a `scripts/check_ability_description_vocab_blocklist_freshness.sh` (or amend the existing script) that scans ADR-0083's `| System term | Product term |` table rows and asserts each "never user-facing" entry is either present in the blocklist or explicitly exempted in a sibling allowlist file. Single source of truth then becomes ADR-0083, with the txt file as a materialized projection.

## 3. Fixture test design — `pii-fixture-marker` + the meta-gate question

The fictional marker is correct discipline per CLAUDE.md §Critical Rules ("no customer-specific data in source code"). The fixture-test script also self-provisions a per-CI-run PII blocklist when `.claude/pii-blocklist.txt` is absent (`CREATED_PII_BLOCKLIST=1`), which is necessary because `.claude/` is gitignored and therefore absent in GitHub Actions.

**There is no meta-gate preventing a future contributor from inlining a real customer term (e.g., `blackstone`) into the fixture.** The defenses today:

1. `.claude/hooks/pre-commit-gate.sh` runs locally against the developer's own `.claude/pii-blocklist.txt`. If their local list contains the term, the commit is blocked. **But the contributor's local list is per-developer and not authoritative for the team.**
2. In CI, `.claude/pii-blocklist.txt` is absent, so `load_terms(pii_blocklist, required=False)` returns `[]`. The lint gate would **not** catch a real PII term hardcoded in a fixture because there is no shared CI-side PII list at all.
3. The fixture test self-creates a one-term PII blocklist containing only `pii-fixture-marker`. So even if a contributor inlined `blackstone` into the fixture, the fixture test would still pass (the script's blocklist only contains the marker term).

**This is a structural gap.** The CI workflow enforces vocab + PII as the commit message claims, but the **PII half is effectively a local-developer aid only**, not a CI gate. For W1-D the gap is small (no real descriptions exist yet), but it will widen as W2/W3 add real ability description copy.

**Recommend** (path-α maintenance): provision a small, **non-gitignored**, public-safe PII denylist (e.g. `scripts/ability_description_public_pii_denylist.txt`) seeded with terms that are public-safe to commit because they're the integration target's brand or a fictional-marker class. Real customer/account names continue to live only in the gitignored local list. The lint script then loads from both. This makes the CI gate match the commit message's promise without leaking PII into the repo.

A lighter meta-gate also works: a separate `check_fixtures_against_local_pii.sh` that runs **only in the local pre-commit hook** (not CI) and grep-scans `scripts/*test*.sh` for any term in `.claude/pii-blocklist.txt`. This catches the "contributor adds blackstone to a fixture" case at commit time without needing a public denylist.

## 4. CI workflow step ordering — PII/vocab → inventory → authorship

The three new steps land after `Enforce durable source comments` and before `Enforce must_use on DB mutation methods`. The relative order among the three is:

1. Ability description PII + vocab (W1-D)
2. Ability surface inventory matches live registry (W1-C)
3. ADR-0130 substrate-owned composition authorship (W1-E.1)

**No architectural sequencing concerns.** None of the three gates have dependencies on each other; all three are independent lint passes on different surfaces (macro attrs / JSON description fields for W1-D; ability registry shape for W1-C; composition authorship metadata for W1-E.1). They could run in any order or in parallel. Putting the PII/vocab gate first is mildly preferable — it fails fast on the highest-disclosure-risk class — and the chosen order follows that principle. Fine as-is.

One small observation: none of the three steps gate each other via `if: success()` or `continue-on-error`. A failure in step 1 still allows steps 2 and 3 to run because `cargo`-style steps in `test.yml` run sequentially with default fail-fast on the first non-zero exit. That's correct for lint gates — you want the full list of violations across all three classes in one CI run, not a cascade of "fix one, push, discover the next." Confirmed by reading the surrounding workflow.

## 5. Cross-ADR consistency — ADR-0083 + ADR-0102 §7.4 / §7.6

**ADR-0102 §7.6 (amendment, 2026-05-10) explicitly establishes the contract the gate enforces:**

> "Per-ability metadata fields (name, description, schema, annotations, category) are model-facing AND browser-facing API surface for SurfaceClient consumers. Description copy is reviewed with the same care as user-facing UI (per ADR-0083 discipline)."

The W1-D gate is a direct mechanization of that sentence — it's the substrate enforcement of "reviewed with the same care as user-facing UI." ADR-0102 §7.4 actor-filtered introspection establishes that the same description string is the surface that an `Agent` or `SurfaceClient` actor sees, so a single CI gate over the description field is architecturally the right shape (one source, one gate, all introspection paths).

**The gate also covers `tools/dailyos-abilities.json` description strings**, which is correct — that file is the MCP-side projection of ability metadata (per ADR-0102 §"MCP tool registration"), and ADR-0083 discipline applies equally there.

**Coverage gap I want to flag for completeness** (not blocking): ADR-0102 §7 lists additional metadata fields that are also model/browser-facing — `name`, `category`, `annotations`. The W1-D gate today scans only `description`. A vocab leak through `name = "enrich_account_intelligence"` or `category = "enrichment"` would not be caught. The seed terms in the blocklist (`enrichment`, `intelligence pipeline`, etc.) are exactly the shape that name/category fields tend to use. This is path-α — file as a maintenance follow-up to extend the regex set to `name`, `category`, `annotations` (the same Python `RUST_DESCRIPTION_RE` pattern, parameterized).

## Verdict

**APPROVE.** The gate is architecturally sound, the Python-in-bash implementation choice is correct, ADR alignment is clean, and the AC ("lint gates only, no implementation churn") is satisfied. The three findings below are path-α (not literal AC violations, not ADR-named contract violations, not PR-introduced regressions) and per CLAUDE.md §Behavioral Rules should be filed in the Codebase Maintenance & Production Quality Linear project (`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`), not gating this commit:

1. **Vocab blocklist drift detection.** No mechanism keeps the txt file in sync with ADR-0083's translation table or future amendments. Add a freshness check that projects from the ADR.
2. **CI-side PII gate is effectively absent.** `.claude/` is gitignored, so the CI run has no PII blocklist; the commit message claim of "PII + vocabulary lint" is only half-true in CI. Add a public-safe denylist file (or a local-only fixture-PII pre-commit hook) so the PII half of the gate runs in CI as advertised.
3. **Coverage scoped to `description` only.** ADR-0102 §7 lists `name`, `category`, `annotations` as model-facing alongside `description`. Extend the regex set when those fields become populated in W2/W3.

None of the three block merge. Wave 1 substrate is in good shape.
