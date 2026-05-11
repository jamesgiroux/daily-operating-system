# L2 (Diff) review — DOS-546 W1-D — code-reviewer — 2026-05-11

**Commit:** `57a57e1f` — ability-description CI gate + W1-C/W1-D/W1-E.1 workflow integration.
**Verdict:** APPROVE.

## AC-bounded assessment

1. **Strict mode + multi-line attr scanning** — PASS. Both shell scripts run `set -euo pipefail`. The Rust scanner anchors on `#\s*\[\s*ability\s*\(` then walks balanced parens with string-aware escape tracking (`find_matching_paren`), so multi-line `#[ability(...)]` invocations and any `description = "..."` inside are captured. `re.DOTALL` on `RUST_DESCRIPTION_RE` permits embedded newlines in literals. JSON path delegates to `json.loads` with line-numbered fallback for findings.
2. **Fixture coverage** — PASS. BAD fixture trips on both an internal-vocab term ("intelligence pipeline") and the `pii-fixture-marker`, asserting exit 1 and grepping both terms in stderr. CLEAN fixture asserts exit 0. Ran locally: `EXIT=0`. The lint also exits 0 against the live tree (no `description` attrs today, as expected per AC).
3. **PII discipline** — PASS. `grep -ri "blackstone\|palantir\|automattic\|tumblr"` across the three new W1-D files returns zero hits. The fixture's `pii-fixture-marker` is fictional and gated behind an ephemeral `.claude/pii-blocklist.txt` the test writes itself (with cleanup via trap) when absent.
4. **Vocab blocklist coverage** — PASS. Seven terms (enrichment, AI enrichment, intelligence pipeline, pipeline run, prompt fingerprint, claim writer, trust band scoring) align with ADR-0083 and CLAUDE.md's "raw pipeline vocabulary" guidance. Whole-word + case-insensitive matching via `\s+`-tolerant compile prevents trivial bypass while avoiding substring false positives.
5. **CI integration order** — PASS. Three steps wired contiguously after "Enforce durable source comments." Order is W1-D → W1-C → W1-E.1; each is independent (no shared state), and fail-fast on the first violation is acceptable. CLAUDE.md is also updated to name ability descriptions as a scanned surface (AC §609).

## Path-α observations (file to maintenance project, do not block)

- `line_for_description` in the JSON scanner falls back to line 1 when description content is duplicated across tools; harmless today but could mislabel future findings. Minor.
- Override path parser splits on `:` only — Windows paths would break, but CI is Linux-only.

No acceptance-criterion violations or PR-introduced regressions. Ship.
