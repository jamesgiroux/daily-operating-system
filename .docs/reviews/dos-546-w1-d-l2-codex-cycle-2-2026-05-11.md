# DOS-546 W1-D.1 — L2 codex cycle-2 verdict

**Date:** 2026-05-11
**Commit:** `8cbdfc1c` on `dos-546-wp-studio-spike`
**Cycle-1 review:** `.docs/reviews/dos-546-w1-d-l2-codex-2026-05-11.md` (REVISE, 2 P1)
**Reviewer:** codex (exec, non-interactive)

## Verdict: APPROVE

Both cycle-1 P1 findings closed. Live verification by codex (not just static read).

### F1 (P1, cycle-1): MCP/inventory artifact ungated — CLOSED

`scripts/check_ability_descriptions.sh` (line 202 of embedded Python) now reads
`payload.get("abilities", [])`. Confirmed against `8cbdfc1c:tools/dailyos-abilities.json`:

```
keys = ['abilities', 'schema_version']
abilities = list (len 4)
```

Live fixture test: a temp JSON using `{"abilities": [{"description": "uses intelligence pipeline"}]}`
exits **1** with `matched term "intelligence pipeline"`. The MCP artifact is now gated.

### F2 (P1, cycle-1): `.claude/pii-blocklist.txt` gitignored → CI vocab-only — CLOSED

`scripts/ability_description_pii_denylist.txt` exists in `8cbdfc1c`. Non-comment terms:

```
pii-fixture-marker
acme-corp-test-fixture
```

Both are purely fictional markers. No real PII committed. Header doc-comments authoring rules.

Three-blocklist load order verified at `scripts/check_ability_descriptions.sh` line 89:
- (a) `.claude/pii-blocklist.txt` — optional (per-developer)
- (b) `scripts/ability_description_pii_denylist.txt` — REQUIRED (committed PII safety net)
- (c) `scripts/ability_description_vocab_blocklist.txt` — REQUIRED (committed internal vocab)

CI-shape simulation: with local `.claude/pii-blocklist.txt` absent (verified via
`test -f` → exit 1), a fixture containing `pii-fixture-marker` still exits **1**.
Committed denylist provides the CI safety net cycle-1 demanded.

### Path-α P2 (cycle-1, deferred): Rust raw strings + inventory.toml evade scanner

Acceptable for W1-D scope. Not a literal AC violation, not a PR-introduced regression.
Files as Linear maintenance ticket per project memory rule (path-α findings →
`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`).

### L1 cross-checks (codex live exec)

- `./scripts/check_ability_descriptions.sh` exit 0 on current tree.
- `./scripts/check_ability_descriptions_test.sh` exit 0.
- Working tree clean for the two changed files (no uncommitted drift).
- `git show --name-only 8cbdfc1c` confirms only the two intended files changed.

## Bottom line

W1-D.1 closes both cycle-1 P1 findings with live verification. APPROVE.
