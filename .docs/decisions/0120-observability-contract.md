# ADR-0120: Observability Contract

**Status:** Proposed
**Date:** 2026-04-20
**Target:** v1.4.0 (must land before any ability substrate ships)
**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md)
**Related:** [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md), [ADR-0110](0110-evaluation-harness-for-abilities.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md)
**Consumed by:** [DOS-250](https://linear.app/a8c/issue/DOS-250) (Debug Trace Surface)
**Resolves:** Persona-review findings S1 (day-1 observability plan), S9 (structured logging + correlation), A7 (observability as a cross-cutting concern)

## Context

Twenty v1.4.0 substrate ADRs land claims, provenance, trust scoring, signal propagation, and the runtime evaluator — all of which produce telemetry in some form. Each ADR defines its own telemetry locally: [ADR-0105](0105-provenance-as-first-class-output.md) carries the `Provenance` envelope; [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) produces `PromptFingerprint`; [ADR-0110](0110-evaluation-harness-for-abilities.md) generates eval run data; [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md) writes `evaluation_traces`.

What does not exist is a **cross-cutting contract** that every ability, every service function, and every background worker emits uniformly. The aggregate effect: when something goes wrong in production, an engineer (or Claude Code session) debugging the issue has to stitch together provenance envelopes, ad-hoc `println!`-style logs, `signal_events` rows, and maybe a `tracing` span here and there — every ADR made its own choice and no contract ties them together.

The persona review flagged this as the single highest-urgency cross-cutting concern (findings S1, S9, A7). Without an observability contract:

- Day-1 debugging of a bad trust score or unexpected tombstone requires log archaeology.
- The [DOS-250](https://linear.app/a8c/issue/DOS-250) debug trace surface has no uniform data to consume across abilities.
- Correlation across an invocation — `invocation_id` exists in provenance but doesn't thread through log lines, signal events, or evaluation traces — is manual.
- Retrofitting observability after substrate lands is an order of magnitude more expensive than baking it in now.

This ADR defines the contract. It is intentionally small: three required fields per invocation, one correlation primitive (`tracing` spans), one log format (NDJSON on stderr). Implementation is a few hours of work at AI velocity. Its payoff compounds — every future ADR inherits the contract by default.

## Decision

### 1. Every invocation carries an invocation record

Every ability invocation, every mutation service function, every background worker pass, and every LLM provider call emits a structured **invocation record**. Records are NDJSON lines to stderr under a single consistent schema.

Minimum required fields:

```rust
pub struct InvocationRecord {
    // Identity
    pub invocation_id: InvocationId,          // ULID, one per top-level ability invocation
    pub span_id: SpanId,                       // Nested span within the invocation
    pub parent_span_id: Option<SpanId>,        // Composition parent
    pub kind: InvocationKind,                  // Ability | Service | Worker | ProviderCall | SignalEmit

    // Identity of the thing being invoked
    pub name: String,                          // e.g., "prepare_meeting", "services::claims::commit_claim"
    pub version: Option<String>,               // Ability version, service fn version, provider model, etc.

    // Outcome
    pub outcome: Outcome,                      // Success | Error | Warn | Skipped
    pub error_kind: Option<String>,            // Typed error category if Error

    // Timing
    pub started_at: DateTime<Utc>,
    pub duration_ms: u64,

    // Context (optional but encouraged)
    pub entity_id: Option<EntityId>,
    pub actor: Option<String>,                 // Actor string per ADR-0113

    // Mode
    pub mode: ExecutionMode,                   // Live | Simulate | Evaluate per ADR-0104
}
```

The record is **invocation-grained**, not log-grained. Individual log messages within an invocation are child records inheriting `invocation_id` + `span_id`. This produces a tree per invocation that the debug trace surface ([DOS-250](https://linear.app/a8c/issue/DOS-250)) can render directly.

### 2. `tracing` crate with span propagation

Implementation uses the Rust `tracing` crate (already idiomatic for Tauri + Tokio). Every ability entry point opens a root span carrying `invocation_id`; nested operations enter child spans that inherit the id automatically.

```rust
use tracing::{info_span, instrument};

#[instrument(skip(ctx), fields(invocation_id = %Invocation::new(), ability = "prepare_meeting"))]
pub async fn prepare_meeting(
    ctx: &AbilityContext,
    input: PrepareMeetingInput,
) -> AbilityResult<MeetingBrief> {
    // All logs within this function and any nested spans carry invocation_id
    tracing::info!(entity_id = %input.account_id, "invocation start");
    // ... implementation
}
```

`tracing`'s structured fields become NDJSON fields in the emitted record. A minimal subscriber formatter (shipped as a library helper) produces the `InvocationRecord` shape above.

**Every** `#[ability]` macro-expanded entry point, every service mutation function, every background worker tick, and every `IntelligenceProvider::complete()` call opens a span. No exceptions.

### 3. Correlation across signals, claims, and outputs

The `invocation_id` is threaded through everything the invocation touches:

- Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md)) already carries `invocation_id`. This ADR reaffirms it as the stable correlation key.
- Signal events ([ADR-0115](0115-signal-granularity-audit.md)) gain an optional `caused_by_invocation_id` column. Every signal emitted as a consequence of an invocation records which invocation caused it. Debug queries like "show me everything that happened because of invocation X" become trivial SQL.
- Claim writes (`propose_claim`, `commit_claim`) record `caused_by_invocation_id` in the same way on `intelligence_claims` (or in a companion audit table if storage pressure is a concern).
- Ability outputs in `evaluation_traces` ([ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md)) already carry `primary_invocation_id`.

Any future storage that could be correlated to an invocation **should** carry `caused_by_invocation_id`. The convention is: if an engineer would reasonably ask "what invocation caused this row," the row has the field.

### 4. Log format: NDJSON to stderr

One record per line. JSON object. To stderr (stdout is reserved for actual app data where applicable). Standard tools (`jq`, `grep`, `less`) work immediately. No external log aggregator required for v1.4.0.

Example:

```json
{"invocation_id":"01HZ...","span_id":"s-001","kind":"Ability","name":"prepare_meeting","version":"1.3","outcome":"Success","started_at":"2026-04-20T12:00:00Z","duration_ms":847,"entity_id":"acct-acme","mode":"Live"}
{"invocation_id":"01HZ...","span_id":"s-002","parent_span_id":"s-001","kind":"Service","name":"services::claims::commit_claim","outcome":"Success","started_at":"2026-04-20T12:00:00.412Z","duration_ms":12,"entity_id":"acct-acme","mode":"Live"}
{"invocation_id":"01HZ...","span_id":"s-003","parent_span_id":"s-001","kind":"ProviderCall","name":"glean.chat","version":"claude-opus-4.7","outcome":"Success","started_at":"2026-04-20T12:00:00.500Z","duration_ms":320,"mode":"Live"}
```

Log lines outside the invocation record schema (e.g., ad-hoc `tracing::info!`) are also emitted as NDJSON but marked `kind: "Message"`. They are not first-class invocation records; they exist for human-readable context alongside structured records.

### 5. Log levels and sampling

Levels follow `tracing` conventions:

- `ERROR` — something failed and the caller sees a degraded outcome.
- `WARN` — something unexpected but the invocation completed normally.
- `INFO` — invocation boundaries (start, end, major phases).
- `DEBUG` — internal detail useful for diagnosis.
- `TRACE` — exhaustive, usually off.

Default filter: `RUST_LOG=info,dailyos=debug`. Invocation records themselves are emitted at `INFO` level minimum — they always flow regardless of the `DEBUG` setting — so production observability is complete without verbose output.

No sampling in v1.4.0. Every invocation record is captured. Volume is bounded by user activity (this is a single-user native app). If volume becomes a concern in future multi-user scenarios, head-based sampling at the span root is the right place; not planned for v1.4.0.

### 6. Redaction and privacy

The observability contract honors [ADR-0108](0108-provenance-rendering-and-privacy.md) masking rules. Specifically:

- **No raw user content in log records.** `entity_id` is permitted (it is metadata, not content). Claim text, email body, transcript fragment, etc. are never emitted to logs.
- **No prompt text or response text in log records.** Prompt template ID + version is permitted; prompt content is not. Completion length (tokens, chars) is permitted; completion content is not.
- **Actor strings follow [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) canonical forms.** `user` and `agent:name:version` are permitted; user email addresses are not.
- **Error kinds are typed enums, not error messages.** `error_kind: "ProviderTimeout"` is permitted; `error_kind: "Timed out waiting for response to 'summarize Alice's...'"` is not.

The distinction: logs carry **the shape of what happened**, not **the content of what happened**. Content lives in the encrypted database with the provenance envelopes and `evaluation_traces`. Debug flows pull content at display time through [ADR-0108](0108-provenance-rendering-and-privacy.md) renderer, not by reading raw logs.

### 7. Mode awareness

Under [`ExecutionMode::Evaluate`](0104-execution-mode-and-mode-aware-services.md):

- Invocation records are captured to an in-memory ring buffer rather than emitted to stderr. Fixtures can assert against the captured record set.
- Log output is suppressed from test stderr (a common source of noise in CI output).

Under `ExecutionMode::Simulate`:

- Records emit normally but are flagged `mode: "Simulate"`. The debug trace surface renders simulation runs distinctly from production runs.

Under `ExecutionMode::Live`:

- Records emit to stderr per §4.

### 8. Log rotation and retention

Log files are managed by the platform (macOS forwards stderr to the app container's log; user sees via Console.app). DailyOS does not manage log rotation directly.

For users who need longer retention, a `scripts/export_logs.sh` utility reads the last N days of logs from Console and emits them as NDJSON. Not shipped in v1.4.0; file under DevEx.

### 9. Log records vs debug trace surface

This ADR defines **how observability data is produced**. [DOS-250](https://linear.app/a8c/issue/DOS-250) defines **how it is rendered** to the developer in-app.

The debug trace surface consumes:

- Invocation records from the NDJSON stream (recent, ephemeral, time-bounded).
- `evaluation_traces` rows (durable, bounded retention).
- `intelligence_claims` + `signal_events` filtered by `caused_by_invocation_id`.
- Provenance envelopes for selected invocations.

Both are necessary. Log records are the ephemeral stream; the DB tables are the durable record. Together they answer "what happened" at any granularity.

### 10. Telemetry — local always, aggregate only opt-in

This ADR distinguishes two tiers of telemetry:

**Local-only (always on, shipped in v1.4.0).** Invocation records per §1–§4 are emitted to stderr as NDJSON on the user's device. SQL queries over `evaluation_traces`, `signal_events`, `intelligence_claims` answer per-user observability needs. The strategy doc's metrics dashboard is satisfied by these queries for single-user diagnosis.

**Opt-in aggregate telemetry (new category, 2026-04-20 per outside voice finding #5).** Population-level metrics required to validate the harness bet ([ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) Bet 1) — correction rates across users, evaluator composite distributions, Glean availability, ghost-resurrection incident counts — cannot be assembled from per-user SQLite alone. Per [ADR-0116](0116-tenant-control-plane-boundary.md)'s amended §3 metadata taxonomy (2026-04-20), an opt-in aggregate telemetry class is permitted:

**Shape of the aggregate emission:**

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

**Strict rules for aggregate emissions:**

- **Counts, durations, percentiles, booleans only.** No free-text fields. No hashes of content.
- **No entity references.** `entity_id`, `actor`, `claim_text`, `field_path`, `prompt_template_id` — all forbidden in aggregate.
- **No invocation_id.** Local-only correlation; aggregation doesn't need it.
- **Hourly bucketing.** Rapid-fire metrics aggregate locally before emission; sampling rate is one roll-up per hour per `metric_name`.
- **Anonymized install ID.** Generated at first boot as a random UUID, stored locally, never reset automatically. Used to count distinct installs reporting. Cannot be tied to user identity because DailyOS never asks the user to log in to a server with this ID.
- **Enumerated metric names.** A `const` list of permitted metric names in `observability::aggregate_metric_catalog`. New metrics require the same code review pressure as a new ADR amendment — can't slip in.

**Opt-in flow:**

1. User's first launch shows a one-time splash: "Help improve DailyOS by sending anonymous usage statistics? (Counts and timings only; no account data, no claim content, no identity.)"
2. Default choice is OFF.
3. User can toggle via Settings → Privacy at any time; disabling stops all emission immediately.
4. When ON, a persistent small indicator in the app footer shows telemetry is active. Click → takes the user to the same settings page.
5. The first time telemetry is enabled, show a sample of what would be sent in the last 24 hours, so the user can verify content. Transparency beats reassurance.

**Destination:**

An HTTPS POST to a DailyOS-operated collection endpoint. The endpoint's contract: accept JSON, respond 200, never echo. Responses do not drive behavior in the client. Network failure → local buffer (capped at 24 hours; oldest dropped). TLS required; no exceptions.

**Not in v1.4.0:**

The opt-in surface + aggregate collector ships in v1.4.1 or v1.5.0 (depending on what prompts it — earliest is when the runtime evaluator rolls out per [ADR-0119](0119-runtime-evaluator-pass-for-transform-abilities.md) and the harness-bet-validation metrics become relevant). v1.4.0 ships local-only telemetry + the ADR amendment defining the shape of what opt-in will look like. A separate Linear issue tracks the opt-in implementation.

**Rationale:**

Strategy doc's harness bet (Bet 1) cannot be empirically validated from local-only data. Population-level signals are required. [ADR-0116](0116-tenant-control-plane-boundary.md)'s firm metadata-only boundary stands — opt-in anonymous aggregate telemetry fits the existing "metadata about user actions" permitted class when bounded precisely as above.

If users overwhelmingly opt out, the harness bet becomes unverifiable and Bet 1 rewrites as a craft/taste commitment rather than an empirical one. That's acceptable — the architecture preserves user agency over whether to contribute data.

## Consequences

### Positive

- **Day-1 observability problem solved.** Every ability invocation has structured records with correlation IDs. Debug investigation in v1.4.0 works from day one.
- **Debug trace surface ([DOS-250](https://linear.app/a8c/issue/DOS-250)) has a uniform data source.** Every ability looks the same when rendered in the trace panel. No per-ability debug implementation.
- **Correlation across substrate is structural.** "Show me everything caused by invocation X" is a single SQL query across claims + signals + eval traces.
- **Retrofitting avoided.** Adding this after substrate lands is expensive; adding it alongside is cheap.
- **Privacy preserved.** Logs carry shape, not content. Content stays encrypted.
- **No external aggregation dependency.** NDJSON on stderr works with `jq`, `grep`, and a developer who needs to debug. No Datadog, no Grafana, no OpenTelemetry required to ship v1.4.0.
- **Existing `tracing` ecosystem.** Rust crate is idiomatic; no custom framework.

### Negative / risks

- **`#[instrument]` is boilerplate.** Every ability, every service mutation, every worker. Mitigated by the `#[ability]` macro ([ADR-0102](0102-abilities-as-runtime-contract.md) §7) expanding to include it; service functions can adopt `#[instrument(skip(ctx))]` by convention.
- **NDJSON volume at scale.** Single user, single device: fine. Multi-user future: head-based sampling at span root will be necessary. Not a v1.4.0 concern; acknowledged.
- **Schema version of `InvocationRecord`.** Adding a field is backward-compatible; removing one isn't. Schema evolves; add `record_schema_version: u32` and parse forward-compatibly. Start at `1`.
- **Correlation field on signal_events is a schema change.** Small (one nullable column); migration needed. Low risk.
- **Developer muscle required.** Contributors must remember to annotate new service/ability entry points. Linting + macro coverage mitigate; discipline covers the rest.

### Neutral

- No user-facing change.
- No performance impact expected at v1.4.0 user volume (single user, native app).
- `tracing` crate is already a dependency in most Tauri apps; verify it's configured correctly.

## Implementation notes

- The `Invocation` type, span helpers, and NDJSON subscriber live in a new module: `src-tauri/src/observability/` (new top-level module).
- The `#[ability]` macro from [ADR-0102](0102-abilities-as-runtime-contract.md) §7 expands to open the root span with `invocation_id` + `ability_name` + `ability_version`.
- Service mutation functions take `&ServiceContext`; the observability subscriber reads `mode` from the context.
- The NDJSON subscriber is a thin wrapper over `tracing_subscriber::fmt::layer().json()` with a custom event formatter that emits the `InvocationRecord` shape when `kind` is set.
- Tests assert against recorded events via `tracing_test::traced_test` or a custom `InMemoryLayer`.

## Scope for v1.4.0

Ships:
- `observability::` module with `Invocation`, `SpanId`, `InvocationRecord`, `Outcome`, `InvocationKind`.
- `#[ability]` macro wiring to open root span.
- Service mutation function annotation convention (instrument via macro or by hand).
- `IntelligenceProvider` trait's `complete()` wired to span (blocker DOS-259).
- NDJSON subscriber + stderr emission.
- Redaction rules enforced by typed fields (no free-text fields that could leak content).
- `caused_by_invocation_id` column added to `signal_events` (schema migration).

Out of scope:
- Metrics aggregation (derived from records at query time if needed).
- Log rotation / retention utilities.
- Sampling (no need at single-user volume).
- External log aggregator integration.

## Amendments and evolution

- Adding new `InvocationKind` variants: backward-compatible, no amendment required.
- Adding optional fields: backward-compatible, no amendment required.
- Removing or renaming fields: amendment required + `record_schema_version` bump.
- Changing redaction rules: amendment required; changes how renderers operate.
- Any new storage table that could correlate to an invocation: **must** include `caused_by_invocation_id` as a nullable FK-style reference. This convention is enforced by code review.
