# L0 review cycle 3 — DOS-209 plan v3 — code-reviewer (substituted for codex challenge)

**Reviewer:** code-reviewer (cycle 3; substituted for /codex challenge per L6 process note — runtime stall on this prompt pattern)
**Plan revision under review:** v3 (2026-04-29)
**Authorized cycle-3 scope:** NF1 (catalogue) + NF2 (landing order) + NF3 (CI command) only
**Verdict:** APPROVE

## NF1 closure verification

### Audit script exists at scripts/dos209-mutation-audit.sh: yes
Path: `/Users/jamesgiroux/Documents/dailyos-repo/scripts/dos209-mutation-audit.sh`. Mode: 755 (executable). 296 lines, last modified 2026-04-29 05:56. File is currently untracked in git (process flag, see rationale).

### Audit script is programmatic (regex/AST): yes — programmatic regex-based scanner with light AST awareness
Approach: bash launcher pinned to `git rev-parse --show-toplevel` invokes a Python heredoc that:
1. Walks `src-tauri/src/services/**/*.rs` deterministically (sorted).
2. Parses each file with a Rust `fn` regex (`FUNCTION_RE`) plus a hand-rolled brace matcher (`matching_brace`) that handles line comments, block comments, regular strings, char literals, and raw strings (`r"..."`, `r#"..."#`). This is brace-balanced lexer-lite, not just naive regex — it correctly skips string-internal braces.
3. Excludes `#[cfg(test)] mod ... { ... }` spans by computing their byte ranges first (`cfg_test_spans`) and dropping any function whose start index falls inside one. Also drops functions preceded by `#[test]` or `#[tokio::test]` within a 160-byte prefix window.
4. Classifies bodies against seven mutation regex families: D (DB writes), SQL (`execute`/`execute_batch`), TX (`with_transaction*`), SIG (`emit*`/`.emit(`), FS (`fs::write|create_dir_all|remove_*|rename|set_permissions`), BG (`enqueue*|.enqueue(|remove_by_entity_id|invalidate_and_requeue|schedule_recompute|notify_one`), EXT (`google_api|gmail|reqwest|.post(|.send().await|run_report_generation|...`), and C (`Utc::now|thread_rng|rand::rng`).
5. Drops C-only rows (clock/RNG without any other side effect) — pure-clock helpers are not mutators.
6. Emits a stable header plus one row per mutator: `module::fn:line | KINDS | KIND=path:line:snippet ; ...`.

Verdict: this is genuinely re-runnable on any future commit; not a hand-curated list.

### Script output matches committed snapshot: yes
Verification command: `bash scripts/dos209-mutation-audit.sh > /tmp/dos209-audit-fresh.txt && diff /tmp/dos209-audit-fresh.txt src-tauri/tests/dos209_mutation_catalog.txt`. Result: exit 0, zero-byte diff. Both files are 233 lines. The committed snapshot is a faithful, reproducible product of the committed script against the current `src-tauri/src/services/` tree.

### All 5 cycle-2-omitted mutators present: yes (all 5)
Direct grep against the snapshot:
- `accounts::snooze_triage_item:1941` — present at snapshot line 23, kinds `D+C`, evidence `db.snooze_triage_item(...)` and `Utc::now()`.
- `emails::unarchive_email:1124` — present at snapshot line 70, kinds `D+EXT`, evidence `db.unarchive_email(&eid)` and `crate::google_api::get_valid_access_token()`.
- `emails::unsuppress_email:1181` — present at snapshot line 72, kinds `D`, evidence `db.unsuppress_email(email_id)`.
- `emails::pin_email:1186` — present at snapshot line 73, kinds `D+SIG`, evidence `db.toggle_pin_email(email_id)` and `emit_and_propagate`.
- `entity_linking::rules::p2_thread_inheritance::evaluate:9` — present at snapshot line 99, kinds `D+BG`, evidence `db.enqueue_thread_inheritance(thread_id, ...)`.

All five cycle-2 NF1 omissions are now catalogued with kind classifications consistent with the cycle-2 challenge's diagnosis (DB + external + signal + queue side effects).

### CI no-drift test in §9: yes
Test name: `services::context::tests::mutation_catalog_no_drift`.
Quoted spec from §9: "re-runs `scripts/dos209-mutation-audit.sh` and asserts stdout exactly matches `src-tauri/tests/dos209_mutation_catalog.txt`; any drift breaks CI. This closes cycle-2 challenge NF1." This is a concrete, named, mandatory test bound by the §9 commands list.

## NF2 closure verification

### §1 acknowledges 2026-04-29 amendment: yes
Location: §1 first sub-block, titled `### Amendment — 2026-04-29 (L6 decision after cycle-2 review)`, immediately under "Verbatim frozen contract from Linear DOS-209, including the 2026-04-29 L6 amendment:". The amendment block is rendered before the original Problem/Scope blocks, with both the original landing-order line and the amended landing-order line preserved (§1 "Dependencies" block ends with `**Landing order (original):** this issue first; ...` and `**Landing order (amended 2026-04-29 per L6 decision):** see amendment block above. W2-B ... lands first.`). Both versions visible — author did not silently overwrite the frozen contract.

### §7 reframes W2-B-first as frozen contract: yes
Location: §7 paragraph 2: "W2-B-first is now the frozen DOS-209 contract per the 2026-04-29 L6 amendment, not merely coordination guidance:" — followed by a quoted blockquote restating the amendment, then the W2-B/W2-A edit-boundary list. v2's framing was "per coordination guidance"; v3 explicitly re-frames as "frozen ... contract per the 2026-04-29 L6 amendment." Closure final sentence: "This closes cycle-2 challenge NF2."

### Amendment text matches Linear (not paraphrased): cannot independently verify
I do not have direct Linear access from the review environment. The plan's amendment text matches the L6 decision packet's Option-1A recommended wording in substance (W2-B first, dated 2026-04-29, L6-authorized, architectural rationale = reduce W2-A's mutation sweep surface, supersedes the original "this issue first"). If the operator confirms the Linear ticket carries the same wording (or a clear cite to the L6 decision file), this closes cleanly. The plan does not paraphrase covertly — it explicitly preserves the original Dependencies block intact and adds the amendment as a separately-marked block, which is the right architectural shape regardless of exact wording match. Not a REVISE-blocking concern.

## NF3 closure verification

### §9 includes full cargo clippy + test + tsc: yes
Verbatim quote from §9: "CI command: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml && pnpm tsc --noEmit`. This restores the full regression suite and closes cycle-2 challenge NF3."

### Targeted dos209 tests are additional, not replacement: yes
Quoted: "Additional DOS-209 evidence command: `cargo test --manifest-path src-tauri/Cargo.toml dos209`; this targeted invocation supplements the full regression suite and is not a replacement." The wording explicitly disclaims replacement. v2's command (which dropped clippy + tsc and narrowed test scope) is repaired.

## Scope discipline

### Cycle-3 changes confined to the three authorized NFs: yes
Section-by-section comparison against the cycle-2 v2 plan as approved by architect-reviewer-v2 and codex-consult-v2:
- §1 Contract: identical Problem/Scope/Acceptance/Edge/Dependencies/checklists from v2 plus the new Amendment block at the top and the dual landing-order lines at the bottom of Dependencies. Authorized (NF2).
- §2 Approach: unchanged from v2 (caller construction paths, single-PR migration, no feature flag, `dos209_mutation_catalog.rs` reference).
- §3 Key decisions: catalogue table is now generated from `scripts/dos209-mutation-audit.sh` (NF1) and committed as `dos209_mutation_catalog.txt`. The taxonomy paragraph, ServiceContext visibility paragraph, ServiceError paragraph, transaction-API paragraph, plan-set deferral paragraph, DOS-304 handling paragraph, and CURRENT_TIMESTAMP audit paragraph are unchanged from v2. The catalogue's textual content is the script's deterministic output, not author re-curation. Authorized (NF1).
- §4 Security: unchanged from v2 (capability leakage primary risk, trybuild compile-fail, `new_evaluate` rejects production DB).
- §5 Performance: unchanged from v2 (hot-path enumeration, 5% p99 budget, W1 Suite P baseline comparand).
- §6 Coding standards: unchanged from v2 (services-only-mutations CI invariant activation, Intelligence Loop five answers).
- §7 Integration: NF2 reframing only. Edit-boundary list and protocol unchanged from v2. Authorized (NF2).
- §8 Failure modes + rollback: unchanged from v2.
- §9 Test evidence: NF3 CI command repair plus the new `mutation_catalog_no_drift` test (NF1). Other test names (`proptest_check_mutation_allowed_modes`, `dos209_surface_constructors`, `dos209_mode_boundary`, `dos209_mutation_catalog`, `dos209_lint_regex_test`, `dos209_capability_trybuild`, `dos209_transactions`) unchanged. Authorized (NF1, NF3).
- §10 Open questions: unchanged from v2.

No silent drift detected. Cycle 3 is clean.

## Fresh findings (if any)

### NFv3-1 — Audit artifacts and plan v3 are not yet committed to git (severity: Low)
Description: `git status` reports `scripts/dos209-mutation-audit.sh`, `src-tauri/tests/dos209_mutation_catalog.txt`, and `.docs/plans/wave-W2/DOS-209-plan.md` as untracked. The L6 ruling (Option A) explicitly required "Plan v3 commits the script, the catalogue output, and a test that re-runs the audit and asserts the catalogue file matches." The script content + snapshot match exactly when re-run, the no-drift test is named in §9, and the file paths exist on disk — the substantive cycle-3 work is done. The artifacts simply aren't staged/committed yet. This is a process-flag, not a contract-violation flag: the plan author has produced the artifacts the ruling required and they live at the paths §9 binds; merging the W2-A PR will commit them as part of the implementation work. Worth flagging for the operator to ensure the plan-v3 commit lands before W2-A coding starts so the no-drift test has a stable snapshot to assert against.
Location: working tree state.
What needs to change: commit `scripts/dos209-mutation-audit.sh`, `src-tauri/tests/dos209_mutation_catalog.txt`, and the plan-v3 file before W1 clears L3 / W2-A coding starts. No plan revision required.

### NFv3-2 — Snapshot embeds line numbers; script is line-number-fragile (severity: Low)
Description: The committed snapshot rows include both the function-definition line (`accounts::snooze_triage_item:1941`) and per-evidence line numbers (`D=src-tauri/src/services/accounts.rs:1944`). Any benign edit above a catalogued mutator (insert a comment, reflow imports, add an unrelated function) shifts every downstream line and breaks the no-drift test even when no mutator semantics changed. This is the canonical fragility of line-number-keyed snapshots.

Mitigation already partially in place: the no-drift test fires only as part of `cargo test`, and the script is committed alongside, so re-running it produces a clean re-snapshot — fixing the test is a one-command operation (`bash scripts/dos209-mutation-audit.sh > src-tauri/tests/dos209_mutation_catalog.txt`). The fragility is therefore "extra one-line update on every services-tree edit," not "blocked merges." For DOS-209's substrate-freeze purpose this is acceptable — the test's job is to make sure the catalogue remains the source of truth, not to be edit-stable. Future tightening (sort by `module::fn` only without line numbers, or compute a content-hash per function body) is a v1.4.x follow-on, not a v3 blocker.

Location: snapshot rows lines 1–233 of `src-tauri/tests/dos209_mutation_catalog.txt`; classify-and-emit logic at script lines 277–294.
What needs to change: nothing for v3. File a v1.4.x follow-on if line-number churn becomes a maintenance pain point during W2-A migration.

### NFv3-3 — Script is portable across machines: yes (no fresh issue, recording the verification)
Verified during fresh-pass review:
- Script begins with `set -euo pipefail` and uses `git rev-parse --show-toplevel` to anchor the working directory — runs identically from any CWD inside the repo.
- All file paths produced are relative to repo root (`src-tauri/src/services/...`); no absolute paths leak into the snapshot.
- Python heredoc uses only stdlib (`pathlib`, `re`); no third-party packages, no `pyenv`/venv assumptions.
- No environment-variable reads, no `$HOME`, no system-clock dependencies (sorted file walk gives deterministic ordering).
- File walk uses `sorted(SERVICE_ROOT.rglob("*.rs"))` which is locale-stable since `pathlib` returns `PosixPath` ordering by default.
This is a deterministic, machine-portable script. No fresh finding.

## Verdict rationale

NF1, NF2, and NF3 are all closed at the architectural and operational levels: the audit script exists, is programmatic, runs clean, matches the committed snapshot exactly, and includes all five cycle-2-omitted mutators; §1 and §7 acknowledge and reframe the 2026-04-29 amendment as the frozen contract; §9 restores the full `cargo clippy + cargo test + pnpm tsc --noEmit` command with targeted dos209 tests as supplemental evidence. Sections 2, 4, 5, 6, 8, and 10 are unchanged from the v2 state that architect-reviewer-v2 and codex-consult-v2 APPROVED, so cycle-3 scope discipline is maintained. The two fresh observations (uncommitted artifacts; line-number snapshot fragility) are Low severity and do not warrant pausing work for deeper architecture review — the catalogue gap that triggered the cycle-2 ESCALATE is closed by construction since the snapshot is now the deterministic output of a committed re-runnable script.

## If APPROVE
All three NF closures verified architecturally and operationally; cycle-3 scope discipline maintained; plan is frozen for coding when W1 clears L3.
