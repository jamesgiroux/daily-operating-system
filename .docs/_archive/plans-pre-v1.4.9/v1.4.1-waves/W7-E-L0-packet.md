# DOS-260 W7-E L0 Packet V1

## 1. Header

- **Date:** 2026-05-15.
- **Project:** v1.4.1 - Abilities Runtime Completion.
- **Wave:** Wave 7 - Release gate hardening + close.
- **Agent:** W7-E.
- **Linear issue:** DOS-260 - "Opt-in anonymous aggregate telemetry — collector + client emission + user flow" (verbatim in §2 + §5).
- **Packet status:** V1, ready for L0 review.
- **Boundary for this authoring pass:** documentation-only. Only file created: `.docs/plans/v1.4.1-waves/W7-E-L0-packet.md`.
- **W7-E assignment:** telemetry collector + client emission + user toggle. Closes v1.4.1 background lane started by DOS-284. Source: `.docs/plans/v1.4.1-waves.md:692-696`.
- **W7 merge gate:** v1.4.1 release-gate close. Source: `.docs/plans/v1.4.1-waves.md:698-712`.
- **Reviewer contract:** `qa-expert` + `security-auditor` on L0 panel. **W7-E is the only W7 agent with security-auditor required** because the change adds a new trust boundary (opt-in egress of metric data) and operates within the ADR-0116 tenant control plane boundary's permitted-class taxonomy.
- **Runtime contract:** ADR-0116 §1 (amended 2026-04-20) permits opt-in aggregate telemetry. ADR-0120 §10 specifies shape + flow. Default OFF. Counts/durations/percentiles/booleans only. No free text, no entity references, no hashes of content.

## 2. Load-Bearing User Outcome

DOS-260 frames the problem:

> "Codex adversarial review on the aggregate v1.4.0 plan surfaced that the harness bet (Bet 1 in strategy doc) cannot be empirically validated from per-user local data alone. Population-level signals — correction rates, runtime evaluator composite distributions, Glean availability, ghost-resurrection incident counts across users — are needed to validate that harness work is moving the quality needle."

The load-bearing outcome: **a user who opts in contributes a small set of permitted metrics that let the team validate substrate effectiveness at population level, without any PII or content leakage.**

Required from DOS-260 scope limits:

> "Opt-in only; default is OFF. Counts, durations, percentiles, booleans only. No free-text fields. No hashes of content. Anonymized install ID (random UUID at first boot). Not tied to user identity. No signed-in account required on the collector side. No remote-driven behavior change in the client based on collector responses."

Required from DOS-260 acceptance:

> "`observability::aggregate_metric::AggregateMetric` struct matches ADR-0120 §10 shape exactly. Const `AGGREGATE_METRIC_CATALOG` lists every permitted metric name; adding new entries requires PR review. Local hourly aggregation. Anon install ID at first launch. Opt-in surface: one-time splash on first launch. Settings → Privacy panel: toggle ON/OFF; first time ON shows a sample of last 24h. Active indicator: small footer icon when telemetry is ON. Disabling stops emission immediately + flushes buffer. Local buffer cap of 24 hours. HTTPS POST to DailyOS-operated collector endpoint with TLS. CI test: every emission site references a catalog name; forbidden fields cause compile error."

## 3. Pre-Work

- **Read W7 source of truth.** `.docs/plans/v1.4.1-waves.md:692-696` assigns W7-E to telemetry collector + client emission + user toggle. Closes v1.4.1 background lane started by DOS-284.
- **Read ADR-0116 §1 (amended 2026-04-20).** Opt-in aggregate telemetry is now a permitted metadata class within the tenant control plane boundary.
- **Read ADR-0120 §10.** Specifies the `AggregateMetric` struct shape + flow.
- **Read ADR-0108 for sensitivity discipline.** Aggregate telemetry is a new channel — must be added to the W6-E `RenderPolicyChannel::all()` matrix.
- **Locate observability module.** Currently exists at `src-tauri/src/observability/` (search for the directory structure). The new `aggregate_metric` submodule fits there.
- **Existing telemetry channel.** Channel 6 of the W6-E sensitivity matrix is "telemetry". W7-E telemetry must register itself with that matrix so the compile-time exhaustiveness check (RenderPolicyChannel + `#[non_exhaustive]`) fires if any new channel is added later without classification.

## 4. Architecture

### 4.1 Files Owned

- **Rust (backend):**
  - `src-tauri/src/observability/aggregate_metric/mod.rs` (new): `AggregateMetric` struct + `AGGREGATE_METRIC_CATALOG` const + emission API.
  - `src-tauri/src/observability/aggregate_metric/buffer.rs` (new): local hourly aggregation + 24h cap.
  - `src-tauri/src/observability/aggregate_metric/emitter.rs` (new): HTTPS POST + retry/backoff + offline-buffer behavior.
  - `src-tauri/src/observability/aggregate_metric/install_id.rs` (new): anon install ID generation + storage at `$APP_SUPPORT/dailyos/anon_id`.
  - `src-tauri/src/observability/aggregate_metric/lint.rs` (new): CI compile-time check that all emission sites reference catalog names AND forbidden fields cause compile errors.
- **Frontend:**
  - `src/components/onboarding/TelemetryOptInSplash.tsx` (new): first-launch splash explaining what's sent / what isn't.
  - `src/components/settings/PrivacyPanel.tsx` (extension): telemetry toggle, sample preview, active indicator.
  - `src/components/shared/TelemetryActiveIndicator.tsx` (new): small footer icon visible when telemetry is ON.
- **Configuration:**
  - Collector endpoint URL in config (documented; the collector itself is sibling infrastructure, not part of this PR).

### 4.2 AggregateMetric Struct (verbatim per ADR-0120 §10 lines 182-192)

```rust
pub struct AggregateMetric {
    pub anon_install_id: AnonInstallId,        // Random UUID at first boot; not tied to user identity
    pub metric_name: &'static str,              // Enumerated; not free-text
    pub metric_value: MetricValue,              // Count | Duration | Percentile | Boolean
    pub ability_name: Option<&'static str>,     // For per-ability metrics
    pub ability_version: Option<&'static str>,
    pub signal_type: Option<SignalType>,        // For signal-class metrics
    pub outcome: Option<Outcome>,
    pub bucket_start: DateTime<Utc>,            // Hourly bucket
    pub build_version: &'static str,
}
```

**Field shape rules (closed-enum mechanism, not advisory lint):**

- `AnonInstallId` is a **newtype** wrapping `uuid::Uuid` with no `From<String>` impl; the only constructor is `AnonInstallId::generate_on_opt_in()` (see §4.5). This is the load-bearing "opaque to user" boundary.
- `MetricValue` is a **closed Rust enum** with exactly four variants: `Count(u64)`, `Duration(std::time::Duration)`, `Percentile { quantile: f32, value_ms: u64 }`, `Boolean(bool)`. No `String`, no `Vec<u8>`, no payload variant. The struct shape **makes PII addition a Rust compile error**, not a runtime audit — adding a `String` field requires editing this struct + the ADR amendment, which is the design intent.
- `SignalType` is the existing `crate::signals::SignalType` enum (closed; new variants require an ADR amendment per ADR-0080).
- `Outcome` is a closed enum: `Success | Failure | Skipped | Timeout`. New variants require a code-review-gated PR amendment.
- `ability_name`, `ability_version`, `build_version` are `&'static str` — sourced from compile-time `env!()` or the abilities registry's static identifier set. The cardinality is bounded by the abilities runtime's known set; no runtime user-provided strings ever flow into these fields.

**Forbidden fields (compile-time check):** any attempt to add `entity_id`, `claim_text`, `content_hash`, `actor`, `field_path`, `prompt_template_id`, `invocation_id`, or `file_path` to `AggregateMetric` is rejected by code review per ADR-0120 §10's strict rules ("No entity references", "No invocation_id"). The struct's field list is the gate.

### 4.3 AGGREGATE_METRIC_CATALOG

A `pub const &[&str]` listing every permitted metric name. Adding an entry requires a PR review (CI lint enforces that emission sites reference only catalog names). Initial catalog (subject to revision): correction_rate, ability_invocation_count, glean_availability_pct, ghost_resurrection_incidents, eval_replay_match_pct, etc. — exact list lives in the catalog itself, not duplicated here.

### 4.4 Hourly Aggregation

`buffer.rs` collects rapid-fire metric calls into hourly buckets keyed by `(metric_name, bucket_start)`. On the hour boundary, the bucket flushes to the emitter. Per-metric-name caps prevent runaway-volume emitters from saturating.

### 4.5 Anon Install ID (generated on first OPT-IN, not first launch)

`install_id.rs` defines `AnonInstallId::generate_on_opt_in()`. The UUIDv4 is generated and persisted to `$APP_SUPPORT/dailyos/anon_id` **only when the user first toggles telemetry ON**, not at first app launch. This closes the soft-leak vector where an anonymous ID would exist before consent and could be correlated against the eventual opt-in moment.

**File handling:**

- File written with `0600` permissions (read/write owner only); atomic-write via tempfile + rename.
- Backup/iCloud-sync exclusion attribute set on macOS via `setxattr` `com.apple.metadata:com_apple_backup_excludeItem` — the anon_id must not replicate across the user's devices.
- Clearing the file regenerates a new UUID on the next opt-in (if user had previously opted in, then cleared, then re-opted-in — the old ID's data on server cannot be correlated to the new ID).
- If telemetry is OFF, `$APP_SUPPORT/dailyos/anon_id` does not exist. Asserted by the default-OFF integration test.

**Anonymity contract:** the anon_id is never sent to any server other than the collector endpoint; never bound to a logged-in account; never written to log files or NDJSON stderr output.

### 4.6 Opt-In Surface

`TelemetryOptInSplash.tsx`: one-time first-launch splash. Default OFF. Explains:

- What's sent (list of catalog names + their AggregateValue shapes).
- What isn't sent (no PII, no entity refs, no claim content, no hashes of content).
- Why the team wants the data (substrate effectiveness validation).
- Links to ADR-0120 §10 + ADR-0116 §1 amendment for technical detail.
- Default state: OFF. User explicitly opts in.

### 4.7 Settings → Privacy Panel

`PrivacyPanel.tsx`:

- Toggle ON/OFF for telemetry.
- First-time-ON: show sample of last 24 hours of collected metrics before enabling. User reviews + confirms. (Sample is from the **local pre-opt-in buffer** — see §4.4 — which captures metrics in memory only; nothing is emitted before the user confirms.)
- **Toggling OFF: stop emission immediately AND DROP the pending buffer.** No flush, no final emission, no retained state. The pending buffer's contents are discarded. This is the security-correct disable semantics: the user said stop, so we stop — we don't emit the in-flight buffer.
- Active indicator: when ON, a small footer icon is visible everywhere in the app; click → Settings → Privacy panel.

### 4.8 Active Indicator

`TelemetryActiveIndicator.tsx`: small icon in the app footer when telemetry is ON. Invisible when OFF. Click navigates to Settings → Privacy.

### 4.9 Emission Path

`emitter.rs`:

- **HTTPS-only via typed URL.** The emitter API accepts a `HttpsUrl` newtype, not `&str`. `HttpsUrl::parse(s)` returns `Result<HttpsUrl, Error>` and rejects `http://` schemes at construction. A plaintext URL becomes a compile error at the call site (`HttpsUrl::parse("http://...")` returns `Err`), or a startup-time configuration error when the URL is loaded from config. There is no path where the emitter accepts a non-`HttpsUrl`.
- **Collector endpoint URL: compile-time const for production builds.** The production URL is defined as `const PRODUCTION_COLLECTOR_URL: HttpsUrl = ...;`. Runtime config override is permitted only when the `debug-telemetry-override` Cargo feature is enabled (off in production builds). This prevents the attacker-with-filesystem-write redirect vector.
- **Bounded retry with jittered backoff.** Max 5 retry attempts over 24h, exponential backoff with jitter capped at 1h between attempts. Avoids thundering-herd on reconnect after long offline windows.
- **Local buffer cap of 24 hours; oldest dropped on overflow.**
- **Disabling telemetry: see §4.7 — DROP pending buffer, do not flush.**
- **No remote-driven behavior change.** The client reads only the response status code (200 = success, anything else = retry-eligible). Response bodies are not parsed for any purpose. Response headers do not drive client behavior. Asserted by a unit test on the emitter API surface (signature accepts only status code, not body).

### 4.10 CI Lint — Two Distinct Gates

The "compile-time check" terminology in earlier drafts conflated two gates with different mechanisms. They are now split:

**Gate 1 — Catalog reference (build-script + clippy lint, NOT pure type check):**

The emission API accepts only a `CatalogName` newtype, not `&'static str`. `CatalogName` has no public constructor — the only way to obtain one is via the `aggregate_metric_name!(...)` macro that checks the string literal against `AGGREGATE_METRIC_CATALOG` at macro-expansion time. Compile fails if the string isn't in the const list.

```rust
// Allowed (CatalogName from macro that checks AGGREGATE_METRIC_CATALOG)
emit(aggregate_metric_name!("correction_rate"), ...);

// Compile error — name not in catalog:
emit(aggregate_metric_name!("invented_name"), ...);

// Compile error — bypassing the macro is not allowed:
emit(CatalogName::from("anything"), ...);  // no such constructor
```

A supplementary `cargo` build-script (`src-tauri/build.rs` or a workspace-level clippy lint) walks the emission sites for defense-in-depth, but the macro is the load-bearing gate.

**Gate 2 — Forbidden-field structural impossibility (pure compile-time, no lint required):**

`AggregateMetric`'s field set is closed at the struct definition. Adding `entity_id: String`, `claim_text: String`, or `content_hash: Vec<u8>` requires editing the struct in source, which requires a code-review-gated PR amendment per ADR-0120 §10's strict rules. There is no runtime path where these fields can appear. This is structural impossibility, not a lint.

Together, Gate 1 enforces "no name leakage" (no free-text metric_name slipping in) and Gate 2 enforces "no payload leakage" (no PII fields slipping in).

### 4.11 Sensitivity Matrix Registration

Register telemetry as channel 6 of the W6-E `RenderPolicyChannel::all()` enum. Because the enum is `#[non_exhaustive]` and compile-time exhaustive, any future addition of a new channel without classification fails compile. This packet does not add new variants; it consumes the existing `RenderPolicyChannel::Telemetry`.

**Specific bundle-17 sweep test:** `src-tauri/tests/bundle17_source_lifecycle_actor_provenance_substrate_test.rs::revoked_restricted_rejection_is_green_for_each_channel` already iterates `RenderPolicyChannel::all()` and asserts revoked/restricted source content does not leak. W7-E extends the bundle-17 fixture's expected_output.json to include a Telemetry channel entry asserting that telemetry emissions for a revoked-source-derived metric are suppressed (or, more precisely: telemetry shapes never carry source-derived content, so the test asserts the empty/absent shape per the channel matrix).

### 4.12 Intelligence Loop Check

- **Claim model:** N/A. AggregateMetric is observability, not a claim.
- **Provenance and trust:** N/A.
- **Signals and invalidation:** no new signal types. Emission is fire-and-forget to a server endpoint, not into the signal bus.
- **Runtime and surfaces:** opt-in UI surface respects ADR-0108 sensitivity language conventions.
- **Feedback loop:** N/A.

DOS-260 explicitly disclaims Intelligence Loop fit: this is observability infrastructure that lives outside the claim/trust/signal pipeline.

## 5. Acceptance Criteria

DOS-260 Acceptance, quoted verbatim:

> "`AggregateMetric` struct matches ADR-0120 §10 shape byte-for-byte. CI test: every emission site references a metric_name in `AGGREGATE_METRIC_CATALOG`. CI test: forbidden fields cause compile error. Opt-in default is OFF; verified in integration test. Active indicator visible when opt-in enabled; invisible when disabled. Local buffer capped at 24h; oldest dropped on overflow; verified. No PII, no entity references, no content in any emission path (code audit + linter). `clippy -D warnings` + `cargo test` + `pnpm tsc --noEmit` green."

Testable decomposition:

1. **`AggregateMetric` struct shape matches ADR-0120 §10 verbatim.** All 9 fields with canonical names + types: `anon_install_id: AnonInstallId`, `metric_name: &'static str`, `metric_value: MetricValue`, `ability_name: Option<&'static str>`, `ability_version: Option<&'static str>`, `signal_type: Option<SignalType>`, `outcome: Option<Outcome>`, `bucket_start: DateTime<Utc>`, `build_version: &'static str`. CI test asserts struct field set by name + type via `static_assertions::assert_type_eq_all!`.
2. **CATALOG enforcement via `CatalogName` newtype + `aggregate_metric_name!` macro.** Emission API accepts only `CatalogName`. The macro fails compilation when given a string not in `AGGREGATE_METRIC_CATALOG`. Bypassing the macro is structurally impossible (no public `CatalogName` constructor).
3. **Forbidden fields structurally impossible.** `MetricValue` is a closed 4-variant enum (`Count`, `Duration`, `Percentile`, `Boolean`). `AggregateMetric` cannot carry `entity_id`, `claim_text`, `content_hash`, `actor`, `field_path`, `prompt_template_id`, `invocation_id`, or `file_path`. Adding such a field requires editing the struct + ADR amendment.
4. **Default OFF — first-launch state.** Integration test: fresh `$APP_SUPPORT/`, no `anon_id` file present, no emission attempted, no anon_id written, splash shown but not opted in.
5. **Anon ID generated on first opt-in only.** Test: launch fresh install → no `anon_id` file. Toggle ON → file created with `0600` perms + backup-exclusion attribute. Toggle OFF → file retained but emission stops.
6. **Active indicator visible/invisible.** Frontend test renders both states + asserts icon presence.
7. **Buffer cap 24h with overflow drop.** Unit test fills buffer past cap + asserts oldest entries dropped.
8. **No PII or content in any emission shape.** Code audit (manual review of all emission sites) + Gate 1 + Gate 2 (per §4.10). `MetricValue` variants carry no free-text fields.
9. **Opt-in splash.** First-launch integration test: splash appears, default OFF, user can dismiss without opting in. Asserts no `anon_id` file created on dismissal.
10. **Sample preview from pre-opt-in buffer.** First-time-ON in settings: panel renders last 24h sample from the local in-memory buffer (no emission has occurred yet) before enabling.
11. **Disable + DROP buffer (NOT flush).** Toggle OFF stops emission immediately AND discards the pending buffer's contents. No final emission. Asserted by integration test that fills buffer, toggles OFF, asserts no emission attempt over the next 60 seconds AND that the buffer length is 0.
12. **HTTPS-only via typed URL.** Emitter API signature accepts `HttpsUrl`, not `&str`. `HttpsUrl::parse("http://...")` returns `Err`. Compile error at call sites that pass plaintext.
13. **Collector URL compile-time const for production.** `PRODUCTION_COLLECTOR_URL: HttpsUrl` is a const; runtime override only available under `debug-telemetry-override` Cargo feature (off in production builds).
14. **No response-body parsing.** Emitter API only observes response status code. Asserted by reading the emitter's signature (status-only return type).
15. **Sensitivity sweep coverage.** `bundle17_source_lifecycle_actor_provenance_substrate_test.rs::revoked_restricted_rejection_is_green_for_each_channel` extended to cover `RenderPolicyChannel::Telemetry`. Bundle-17 expected_output.json gains a Telemetry entry asserting no source-derived content in telemetry shapes.
16. **Clippy + cargo test + tsc green.**

## 6. Linear Dependency Edges

- **Canonical issue content:** DOS-260 supplied verbatim in §2 + §5.
- **Upstream:**
  - ADR-0116 §1 amendment (landed 2026-04-20) — permits opt-in aggregate telemetry as a metadata class.
  - ADR-0120 §10 elaboration (landed 2026-04-20) — struct shape + flow.
  - ADR-0119 runtime evaluator — optional. When the evaluator produces `evaluation_traces`, those traces become candidates for aggregation. W7-E does not depend on ADR-0119 to land.
- **Adjacent:** W6-E (bundle 17 sensitivity sweep) provides `RenderPolicyChannel::Telemetry` that this packet registers against. Already merged via PR #290.
- **Out:** the collector endpoint infrastructure is sibling work, filed separately. W7-E is client-side instrumentation + opt-in flow only. Client emits to a configured endpoint; if the collector doesn't exist yet, emissions buffer locally up to 24h and drop.
- **Closes:** v1.4.1 background lane started by DOS-284. Source: `.docs/plans/v1.4.1-waves.md:696`.

## 7. L0 Reviewer Panel

- **Required reviewers:** `qa-expert` + `security-auditor`.
- **Panel reason:** W7-E adds a new trust boundary (opt-in egress) and operates within the ADR-0116 permitted-class taxonomy. security-auditor must verify:
  - No PII or content leakage paths in any emission shape.
  - HTTPS-only enforcement.
  - Default OFF is verified end-to-end.
  - The anon install ID is genuinely anonymous (no correlation with user identity).
  - The CI lint actually catches catalog drift + forbidden-field additions.
  - Sensitivity matrix registration is correct.
- **qa-expert focus:**
  - Opt-in UX flow: splash, sample preview, toggle, indicator visibility.
  - Buffer overflow + flush-on-disable behavior.
  - Integration tests cover the default-OFF, opt-in, and opt-out paths.

## 8. L0 Acceptance Gate

L0 passes only if both reviewers accept:

1. **ADR alignment:** struct shape matches ADR-0120 §10 byte-for-byte.
2. **Permitted-class adherence:** counts/durations/percentiles/booleans only.
3. **CATALOG enforcement:** CI lint blocks unknown metric_name strings.
4. **Forbidden-field compile error:** struct shape makes PII addition a compile error.
5. **Default OFF verified:** integration test.
6. **Opt-in UX:** splash + sample preview + active indicator + settings toggle.
7. **HTTPS-only:** plaintext path impossible.
8. **Anon install ID:** UUIDv4, opaque, clearable, no correlation.
9. **Sensitivity matrix:** registers `RenderPolicyChannel::Telemetry`; passes the W6-E sweep test.
10. **Reviewer panel:** qa-expert + security-auditor both APPROVE.
11. **No PII in tests:** synthetic identifiers only.

## 9. Out-Of-Scope

- Collector endpoint infrastructure (sibling work, separate PR).
- Certificate pinning (evaluate after first shipping per DOS-260 §edge-cases).
- Runtime evaluator (ADR-0119) — telemetry consumes its outputs when present; ADR-0119 itself isn't W7-E scope.
- Adding metric names beyond the initial catalog (each addition is its own PR review).
- Remote-driven client behavior change based on collector responses (explicitly prohibited).
- Signed-in account requirement (the collector accepts anon install ID, not user identity).
- Hashes of content (explicitly prohibited).

## 10. Changelog

- **V1 - 2026-05-15:** Initial W7-E L0 packet. Mapped ADR-0116 §1 + ADR-0120 §10 to acceptance criteria; locked CATALOG + forbidden-field compile-time gate; registered against W6-E RenderPolicyChannel matrix; named qa-expert + security-auditor as required L0 reviewers.
