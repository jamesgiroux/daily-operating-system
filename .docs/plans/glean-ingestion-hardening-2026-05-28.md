# Glean Ingestion Hardening — Phase 1 Spec (2026-05-28)

Status: Spec — pending L0 plan review
Scope tier: Standard, low-risk. Single PR, ~530 LOC across 4 files.
Branch: continues uncommitted work on `codex/glean-output-contract-fix`
Threat topology: local-to-local, single-user, encrypted local DB
Related prior work: `.docs/plans/glean-finalization-shared-producer-l0-2026-05-23.md` (different problem — queue/manual producer asymmetry — but adjacent code path)

## Problem

Glean dimension responses fail at the JSON ingestion step more often than not. The user-visible symptom is partial enrichment — strategic, engagement, or commercial dimensions go stale because their refresh attempts failed silently or surfaced misleading errors.

Investigation (summarized at the top of this session) identified three concrete defects in the current ingestion pipeline:

1. **Shape drift.** Glean emits JSON that drifts from the app's `IntelligenceJson` schema in two specific ways:
   - Scalar fields wrapped as objects: e.g. `"trend": {"direction": "stable", "rationale": "..."}` where the schema expects `"trend": "stable"`. Seen repeatedly on `engagement_signals` and `commercial_financial` dimensions.
   - Multiline strings inside string fields produce `Invalid JSON: control character ... while parsing a string`.
   - Snake_case or typo'd field names that don't match the camelCase contract.
2. **Diagnostic masking.** `try_parse_json_response` at `src-tauri/src/intelligence/prompts.rs:3373` returns `None` on real serde validation failures. The caller falls through to the legacy pipe-delimited parser, which surfaces `No INTELLIGENCE block or JSON found in response`. The real serde error is logged at `warn` level but never reaches the user-facing per-dimension error string.
3. **Lost forensic state.** `glean_provider.rs:310-321` writes only the first parallel dimension's response to a fixed path `dailyos-glean-response.txt`. When dimensions 2-6 fail, the response that caused the failure is unrecoverable after the run.

The investigation report's recommendation is a small, targeted patch. An in-flight uncommitted patch on `codex/glean-output-contract-fix` already implements the shape-drift tolerance layer (495 lines: key normalizer, `stringish_value` coercion, defaulted required fields with empty-text filtering, custom `deserialize_recommended_actions`). It does **not** address defects 2 or 3.

This spec lands the in-flight patch plus the two missing diagnostic fixes as one standalone PR, then defers architectural decisions to data collected from real Glean traffic after the patch ships.

## Out of scope (explicit)

- **PTY-Claude repair shim.** Considered. Deferred until we have data on what the lenient parser leaves uncaught.
- **Producer/consumer rearchitecture.** Considered (Glean as evidence emitter, deterministic step shapes to `IntelligenceJson`). Deferred for the same reason.
- **Tightening the prompt contract further** than the +4 lines already in the in-flight patch. Same reason.
- **Reducing `prompts.rs` file size** (currently 5209 lines). Real signal but different problem.
- **Queue-vs-manual producer asymmetry** addressed in `.docs/plans/glean-finalization-shared-producer-l0-2026-05-23.md`. Adjacent code; different defect.

## Architecture

Pipeline unchanged at the high level:

```
Glean response → extract_json → validate → normalize → deserialize → IntelligenceJson
                                                                  ↘ on failure: real error logged + surfaced
no JSON found ────────────────────────────────────────────────────→ pipe-delimited fallback (legitimate path)
```

The change is **distinguishing two failure modes** that currently collapse into one:
- "No JSON object in the response" — fall through to the pipe parser is the legitimate path.
- "JSON object extracted but failed validation/deserialization" — fall through is misleading. We want the real serde error to surface directly.

## Components

### 1. Lenient parser layer — already in `codex/glean-output-contract-fix` (~495 LOC)

Lands as-is. Touches `src-tauri/src/intelligence/{prompts.rs, dimension_prompts.rs, glean_prompts.rs, io.rs}`. Specifically:

- `normalize_ai_response_value` recursively rewrites snake_case/typo keys to canonical camelCase (e.g. `renewaloutlook` → `agreementOutlook`, `meetingcadenceassessment` → `meetingCadence`).
- `stringish_value` extracts string-bearing inner fields from objects when the schema expects scalars (e.g. `{"direction": "stable", ...}` → `"stable"`).
- `#[serde(default)]` added to previously-required `text` / `name` / `statement` / `description` fields, paired with `.filter(|x| !x.text.trim().is_empty())` so a missing field drops the item instead of the whole array.
- `deserialize_recommended_actions` custom deserializer accepts strings or objects, normalizes to `RecommendedAction`.
- Three one-line "use exact camelCase" reminders in the prompt builders.
- Inline test `test_parse_json_response_accepts_glean_case_and_shape_drift` exercising a Glean-shaped drift blob.

### 2. Error propagation — new (~25 LOC)

In `src-tauri/src/intelligence/prompts.rs`:

- Change `try_parse_json_response` return type from `Option<IntelligenceJson>` to `Result<Option<IntelligenceJson>, String>`.
  - `Ok(Some(intel))` — success.
  - `Ok(None)` — no JSON object found in the response at all (no `{` candidate, no fenced block). Caller falls through to the pipe parser as today.
  - `Err(msg)` — JSON object extracted but `validate_intelligence_response` or `serde_json::from_value` rejected it. Caller returns the error directly; pipe parser is **not** invoked.
- In `parse_intelligence_response` at line 3216, update the call site to match the new three-state return.
- Error message format: `<stage>: <serde_error>` where `<stage>` is one of `validation` or `deserialize`. The existing per-dimension channel wrapper at `glean_provider.rs:346` already prepends `parse failed:`, so the user-visible string is e.g. `parse failed: deserialize: missing field 'text' at line 42 column 3`.

### 3. Per-dimension debug capture — new (~10 LOC)

In `src-tauri/src/intelligence/glean_provider.rs:310-321`:

- Drop the `if is_first` guard.
- Filename pattern: `dailyos-glean-{dim_name}-{utc_millis}.txt` in `std::env::temp_dir()`.
- Always-on. Local /tmp, user's own data, threat topology is local-to-local.
- No retention policy in this PR — `/tmp` is OS-managed.

## Data flow change

Only one branch is new. When `extract_json_from_response` returns `Some(json_str)` but downstream validation or deserialization fails after normalization, the dimension's error string flows back through the existing `mpsc::channel` in `glean_provider.rs` as:

```
parse failed: deserialize: missing field `text` at line 42 column 3
```

instead of:

```
parse failed: No INTELLIGENCE block or JSON found in response
```

The on-disk debug capture at `/tmp/dailyos-glean-{dim_name}-{ts}.txt` then lets us inspect the exact response that triggered the failure.

## Error handling

No new error types. No new error surfaces. The existing per-dimension `Err(String)` channel in `glean_provider.rs:340-347` is the surface; the change is that the strings flowing through it are now diagnostic instead of misleading.

## Testing

- Keep the existing `test_parse_json_response_accepts_glean_case_and_shape_drift` test from the in-flight patch.
- Add one failure-path test: `test_parse_json_response_propagates_schema_error`. Feeds a JSON blob with a structural error the normalizer cannot resolve (e.g. `"risks": "not an array"`). Assert the returned error string contains the serde wording (e.g. "expected sequence", "expected array") and does **not** contain "No INTELLIGENCE block".
- Add one extraction-failure test: `test_parse_json_response_returns_none_when_no_json_present`. Feeds a response with no `{` candidate; asserts `Ok(None)` so the caller's fall-through to the pipe parser still triggers.
- No automated test for per-dim debug capture. Manual verification: run `pnpm dev`, trigger an account refresh that exercises Glean enrichment, confirm `/tmp/dailyos-glean-*` contains one file per dimension.

## Acceptance criteria

1. `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` passes.
2. Existing `test_parse_json_response_accepts_glean_case_and_shape_drift` still passes (no regression on lenient-parser coverage).
3. New `test_parse_json_response_propagates_schema_error` proves the misleading-error path is fixed.
4. Manual verification: a Glean refresh writes one debug file per dimension to `/tmp/dailyos-glean-*`.
5. PR title contains no PII per `.claude/pii-blocklist.txt`. Commit message includes `L2-status: passed`.

## Phase 2 trigger — explicit, not a deferral

After this PR ships, run real Glean refreshes for ~1 week. Categorize per-dimension failures from `/tmp/dailyos-glean-*` and the real serde error strings. Three possible outcomes drive the next decision:

- **<5% failure rate, varied shapes.** Done. No Phase 2.
- **5-20% failure rate, clustered shapes.** Tighten prompt contract or add targeted normalizers in a follow-up PR. Still no Phase 2.
- **>20% failure rate, or persistent unrecoverable shapes.** That's the evidence that justifies revisiting the PTY-Claude repair shim OR the producer/consumer rearchitecture. File as a new L0 plan packet with the failure data attached.

## Intelligence Loop check

This PR does not add new tables, schema columns, claim fields, or user-visible intelligence surfaces. The 5-question check does not apply at the substrate level. The PR hardens the parser path that already feeds existing claim writes; provenance, trust scoring, signal emission, runtime consumption, and feedback are unchanged.

## Risks

- **Normalizer false positives.** The recursive key normalizer could rewrite a legitimately distinct field name to a canonical one and silently coerce wrong data. Mitigation: the in-flight patch's normalizer only rewrites a fixed allowlist (`renewaloutlook`, `meetingcadenceassessment`, `commitments`, etc.). Generic `_` / `-` / space splitting is gated on a separate match arm and only camelCases — it doesn't rename.
- **`stringish_value` data loss.** When Glean returns `{"direction": "stable", "rationale": "..."}` and we extract `"stable"`, the rationale is lost. Mitigation: this is acceptable for v1.4.x — rationale is not part of the consuming schema for those fields. If Phase 2 widens the schema to hold the rationale, the deserializer changes accordingly.
- **Always-on debug capture writes.** Six dim files per refresh per entity. `/tmp` is OS-managed but this is non-zero disk usage. Acceptable for the duration of the data-collection window; revisit if Phase 2 lands.
