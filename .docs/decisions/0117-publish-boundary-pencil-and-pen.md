# ADR-0117: The Publish Boundary — Pencil and Pen

**Status:** Proposed
**Date:** 2026-04-19
**Target:** v1.4.0 (Pencil/Pen protocol on the existing `publish_to_p2`, outbox integration, confirmation contract)
**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0103](0103-maintenance-ability-safety-constraints.md)
**Related:** [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0111](0111-surface-independent-ability-invocation.md)

## Context

[ADR-0102](0102-abilities-as-runtime-contract.md) §3 introduces four ability categories: Read, Transform, Publish, and Maintenance. Three of them write to state that DailyOS controls — in-memory, in the local DB, or in an outbox under [ADR-0103](0103-maintenance-ability-safety-constraints.md). A failed Maintenance ability can be retried, rolled back, or compensated. A failed Read or Transform ability simply does not update anything.

**Publish is different.** A Publish ability writes to a destination DailyOS does not own — a P2 blog post, an external email, a Slack channel, a CRM, an exported file. Once that write succeeds, the side effect is in a system we cannot reliably reverse. Some destinations allow edit or delete (P2 post, Slack message within retention); some do not (email sent, webhook fired, third-party record created). Even when retraction is possible, the effect may have already been observed — the briefing has been read by three stakeholders, the email is in recipients' inboxes, the webhook triggered a downstream automation.

This asymmetry — reversible inside DailyOS, irreversible once the side effect lands — is architecturally distinct. It requires a different contract from the user and a different implementation discipline from the authoring ability. We have one Publish ability today (`publish_to_p2`), referenced in [ADR-0102](0102-abilities-as-runtime-contract.md) §3 and [ADR-0112](0112-migration-strategy-parallel-run-and-cutover.md) §3. Before new Publish abilities land — and before `publish_to_p2` itself gets wrapped in the v1.4.0 abilities runtime — the class needs a shared protocol.

The metaphor this ADR formalizes: **Pencil and Pen.** Every publish is a two-phase operation. Pencil is the draft — reviewable, edit-in-place, infinitely reversible, lives in DailyOS. Pen is the send — irreversible side effect, user-confirmed, materializes outside DailyOS. The transition from Pencil to Pen is the only asymmetric step, and it requires an explicit user or policy gate.

This ADR specifies the protocol, the idempotency guarantees, the retraction behavior where applicable, the outbox integration, and the surface requirements for all Publish abilities present and future.

## Decision

### 1. The Pencil phase

Every Publish ability produces a **Draft** before it produces a side effect. The Draft is a fully-materialized representation of what will be sent:

```rust
pub struct PublishDraft {
    pub id: DraftId,
    pub ability_name: &'static str,
    pub ability_version: AbilityVersion,
    pub destination: PublishDestination,
    pub payload: PublishPayload,           // Full rendered content, not a template
    pub rendered_preview: Option<String>,  // Human-readable summary
    pub recipients: Vec<RecipientRef>,
    pub provenance: Provenance,
    pub expires_at: DateTime<Utc>,         // Drafts are TTL'd; default 24h
    pub created_at: DateTime<Utc>,
    pub created_by: Actor,
    pub state: DraftState,                 // Open | Withdrawn | Committed | Expired | Failed
}
```

Characteristics:

- A Draft is a row in `publish_drafts`. It is DailyOS-internal, encrypted like any other table, subject to [ADR-0116](0116-tenant-control-plane-boundary.md) — content never leaves the device during the Pencil phase.
- A Draft is inspectable. The user or a policy check can see exactly what will be sent, to whom, with what provenance.
- A Draft is editable. Most fields (`payload`, `recipients`, `rendered_preview`) can be amended through a companion Maintenance ability before commit. Every edit bumps a `draft_version` and is logged.
- A Draft is withdrawable. The user (or a policy) can transition state to `Withdrawn`; the Draft row remains for audit but cannot be committed.
- A Draft has a TTL. Default 24 hours. An unacted-on Draft auto-transitions to `Expired` and is never committed.

The Pencil phase is fully inside DailyOS. No external system knows the Draft exists.

### 2. The Pen phase — confirmation contract

Transition from `Open` to `Committed` requires a **ConfirmationToken**:

```rust
pub struct ConfirmationToken {
    pub draft_id: DraftId,
    pub draft_version: u32,              // Prevents confirming a stale draft that was edited after review
    pub confirmed_by: Actor,             // Must be an acceptable actor for this ability
    pub confirmed_at: DateTime<Utc>,
    pub policy_source: PolicySource,     // UserAction | PrePolicy | AgentWithAuthority
}

pub enum PolicySource {
    UserAction { session_id: SessionId },
    PrePolicy { policy_id: PolicyId, evaluated_at: DateTime<Utc> },
    AgentWithAuthority { agent: AgentRef, authority_grant_id: AuthorityGrantId },
}
```

Acceptable `PolicySource` values per Publish ability are declared in the ability's registry entry:

- `UserAction` — a human explicitly clicks "Send" in a DailyOS surface. The default for every Publish ability.
- `PrePolicy` — a pre-authorized policy (e.g., "auto-post my Friday status to P2 if it passes the eval rubric") that was reviewed and granted by a user, and whose conditions match at publish time. Policies are themselves stored, versioned, and revocable.
- `AgentWithAuthority` — an agent has been explicitly granted publish authority by a user, with scope and expiry. Used for future automation; out of scope for v1.4.0 implementation but the ConfirmationToken shape must accommodate it.

A Publish ability invocation that reaches the Pen phase without a valid `ConfirmationToken` fails with `PublishError::ConfirmationRequired`. The call graph is structural: `commit_publish` is the only function that writes externally, and it requires the token.

### 3. Idempotency and the outbox

Publish commits go through the [ADR-0103](0103-maintenance-ability-safety-constraints.md) outbox pattern. The outbox is the boundary between "we intend to publish" and "we have published."

```rust
pub struct PublishOutboxEntry {
    pub id: OutboxEntryId,
    pub draft_id: DraftId,
    pub confirmation_token_id: ConfirmationTokenId,
    pub destination: PublishDestination,
    pub payload_hash: Hash,              // Idempotency key for the destination
    pub attempt_count: u32,
    pub status: OutboxStatus,            // Pending | Delivered | FailedRetryable | FailedPermanent
    pub delivered_at: Option<DateTime<Utc>>,
    pub destination_ref: Option<String>, // e.g., P2 post ID, Slack message ts, email message-id
    pub last_error: Option<String>,
}
```

Semantics:

- `commit_publish` writes the outbox entry and returns. The actual delivery happens via a worker pool, identical in shape to the invalidation job worker ([ADR-0115](0115-signal-granularity-audit.md) §5).
- Every destination client supplies an **idempotency key** computed from `(ability_name, payload_hash, destination_ref_or_recipient)`. Retrying a failed delivery with the same key must be safe — the destination sees either one publish or none, never two.
- Destinations that do not natively support idempotency (plain SMTP, some webhooks) require a local dedup table keyed on the idempotency key, consulted before delivery.
- On permanent failure (4xx from destination, non-retryable policy error) the outbox entry transitions to `FailedPermanent`; the corresponding Draft transitions to `Failed`. User is notified.

The outbox persists across restarts. A crash between `commit_publish` and delivery leaves a `Pending` entry that the worker picks up on startup. At-least-once delivery is guaranteed; exactly-once is enforced via idempotency keys.

### 4. Retraction where possible, clarity where not

Destinations vary in what they allow after publish:

| Destination | Edit | Delete | Retract semantics |
|---|---|---|---|
| P2 post | Yes | Yes | `retract_publish` → calls P2 API to delete post; outbox entry transitions to `Retracted`. |
| Slack message (within retention) | Yes | Yes (within retention window) | `retract_publish` → calls chat.delete; best-effort. |
| Email | No (already sent) | No | `retract_publish` not available; returns `RetractionNotSupported`. |
| Webhook / third-party API | Depends on destination | Depends | Per-destination contract; most default to no. |

Every Publish ability declares its retraction capability:

```rust
pub enum RetractionSupport {
    Full,                                 // Edit + delete supported
    DeleteOnly,                           // Delete but not in-place edit
    TimeBounded { window: Duration },     // Delete possible within a window
    None,                                 // Once sent, done
}
```

Surfaces rendering a completed Publish show the user its retraction state. "This email cannot be recalled" is the honest default for destinations that don't support retraction; the UI does not offer a button that silently fails.

### 5. Destination abstraction

A `PublishDestination` is a typed enum with per-variant fields and per-variant client traits:

```rust
pub enum PublishDestination {
    P2 { site_id: P2SiteId, category: Option<P2Category> },
    Email { recipients: Vec<EmailAddress>, subject: String, reply_to: Option<EmailAddress> },
    Slack { workspace: SlackWorkspaceId, channel: SlackChannelId, thread_ts: Option<String> },
    Export { target: ExportTarget },      // Local file, presigned URL upload
    Webhook { url: Url, auth: WebhookAuth },
}

pub trait DestinationClient {
    fn deliver(&self, payload: &PublishPayload, idem_key: &str) -> Result<DestinationRef>;
    fn retract(&self, destination_ref: &DestinationRef) -> Result<RetractionOutcome>;
    fn retraction_support(&self) -> RetractionSupport;
}
```

`DestinationClient` implementations are mode-aware ([ADR-0104](0104-execution-mode-and-mode-aware-services.md)): under `Evaluate`, they route to a replay layer that records expected calls and returns canned responses. No real external calls in test mode, ever.

### 6. Surface requirements

Any DailyOS surface that triggers a Publish ability must present the user with, at minimum:

- The **rendered preview** — what will actually be sent, not a template.
- The **destination** — where it is going. Named recipients for email, channel name for Slack, site + category for P2.
- The **retraction support** — plain-language statement of what can be undone and what cannot.
- The **provenance summary** — the top-level sources feeding the content, consistent with [ADR-0108](0108-provenance-rendering-and-privacy.md) rendering rules.
- An explicit **Send** action, distinct from any other affordance. Never a default-primary button that a user might accidentally activate.

For pre-policy publishes, the user reviewed and granted the policy at some prior moment — the surface at execution time may be a confirmation log, not a pre-send dialog, but the policy-grant surface itself must have met all the bullets above.

### 7. Pencil/Pen under `ExecutionMode::Evaluate`

Under `Evaluate` ([ADR-0104](0104-execution-mode-and-mode-aware-services.md)):

- Draft creation is identical to `Live` — rows are written to an in-memory or fixture-loaded DB.
- `ConfirmationToken` fabrication is permitted in tests; the token's `policy_source` field identifies the test fixture.
- `commit_publish` writes to the outbox as in `Live`, but the `DestinationClient` is the replay layer.
- No real external delivery occurs. The outbox entry's `status` transitions to `Delivered` with the replay's recorded response.

This means evaluation harness fixtures ([ADR-0110](0110-evaluation-harness-for-abilities.md)) can end-to-end exercise a publish path without sending anything externally. Regression tests for Publish abilities are as cheap as any other ability.

### 8. Audit and provenance

Every Publish leaves a durable audit trail:

- The `publish_drafts` row records what was drafted, by whom, when.
- The `publish_outbox` row records the commit decision, the confirmation token, the delivery attempts, the destination ref, and the final status.
- The Provenance envelope ([ADR-0105](0105-provenance-as-first-class-output.md)) on the `AbilityOutput<PublishedRecord>` returned by the Publish ability includes a `publish_context` field with the outbox entry ID and the destination ref, enabling anyone reading the output later to trace exactly what went out and where.

Audit rows live as long as the related ability's retention policy dictates; typical default is 365 days for Publish records.

### 9. Authorization and `may_publish`

Only abilities registered with `may_publish = true` in the ability registry may call `commit_publish`. Maintenance abilities, which can mutate internal state freely under [ADR-0103](0103-maintenance-ability-safety-constraints.md), may not publish unless their registry entry explicitly grants publish authority. Granting `may_publish` requires an ADR amendment — it is not a routine configuration change.

This prevents a Maintenance ability that refreshes an entity's state from accidentally acquiring publishing capability through composition.

### 10. Scope for v1.4.0

In scope:

- Pencil/Pen protocol in code: `publish_drafts` table, `publish_outbox` table, `ConfirmationToken` type, `commit_publish` service function.
- Outbox worker integrated with the invalidation worker pool from [ADR-0115](0115-signal-granularity-audit.md) §5.
- `publish_to_p2` wrapped in the abilities runtime with Pencil/Pen semantics, replacing any direct-publish legacy path.
- Surface requirements documented for the one existing consumer.
- `DestinationClient` trait with `P2Client` implementation; others stubbed.

Out of scope:

- New Publish abilities beyond `publish_to_p2`.
- Pre-policy or agent-authority publish paths (`PolicySource::PrePolicy`, `AgentWithAuthority` shapes exist but the runtime only accepts `UserAction` in v1.4.0).
- Additional destination clients (Slack, email, webhook) — future work per destination-specific ADRs.
- Batch publish / multi-destination fan-out — future work.

## Consequences

### Positive

- **Irreversible side effects are structurally isolated.** Every publish passes through Pencil → Pen; no ability can write externally without a ConfirmationToken.
- **Retries are safe.** Outbox + idempotency keys mean a crash or transient network failure does not produce duplicate publishes.
- **Audit is complete.** Every publish is inspectable from Draft through Delivery through (optional) Retraction. No opaque "it went out, trust us" moments.
- **Retraction is honest.** The product tells users what can and cannot be undone per destination. No silent-failure retract buttons.
- **Future publish destinations compose.** New destinations become new `DestinationClient` implementations; the Pencil/Pen protocol stays unchanged.
- **Agent-authority shape exists but is not yet enabled.** Future automation work can plug into the same model without re-architecting.

### Negative / risks

- **Two new tables and a worker pool add operational surface.** Mitigated by reusing the [ADR-0115](0115-signal-granularity-audit.md) §5 worker infrastructure; minimal net new code for the queue substrate.
- **Pencil/Pen adds a click to existing publish flows.** Accepted — the click is the product. Surfaces may combine "Preview + Send" into a single panel to keep the user experience tight, but the decision to send must be a distinct gesture.
- **Idempotency keys require per-destination correctness.** Each `DestinationClient` must correctly implement idempotency; a bug here produces duplicate publishes. Mitigated by a per-destination dedup table as the belt-and-suspenders layer.
- **Pre-policy and agent-authority are shapes without users.** Adding `PrePolicy` and `AgentWithAuthority` to the ConfirmationToken enum in v1.4.0 without implementation surface might feel premature. Kept deliberately — the shape of the confirmation model is where forward thinking earns its keep. Implementation waits.
- **Retraction TTLs vary across destinations and change over time.** Slack's delete window has shifted historically. `RetractionSupport::TimeBounded` must be keyed off current vendor capability, not a cached value. Each `DestinationClient` owns its own freshness check.

### Neutral

- `publish_to_p2` behavior becomes protocol-compliant but the user-observable flow is unchanged — the existing P2 posting flow is already a two-step preview/send interaction.
- No user-visible change for consumers of other abilities. Publish is a clearly isolated category.
- The outbox infrastructure is identical in shape to [ADR-0115](0115-signal-granularity-audit.md) — workers, retry, dead-letter — and reuses that code.

---

## Strategic elevation — 2026-04-20

Founder decision (D2, recorded in [ADR-0116](0116-tenant-control-plane-boundary.md) "Founder commitment" section) makes the publish framework strategically load-bearing for DailyOS's enterprise commercial story.

The reasoning: enterprise buyers will ask for "leadership visibility into team activity." The architectural answer is not to soften [ADR-0116](0116-tenant-control-plane-boundary.md)'s metadata-only control-plane boundary. The architectural answer is that **publish is the channel** — a user-initiated push (or user-scheduled push) to an enterprise storage destination the customer controls. DailyOS does not reach into the customer's storage; the user publishes to it. Enterprise gets visibility into what their users choose to publish, in the format (human- or machine-readable) the destination expects, on the cadence the user sets. The control plane sees zero of it.

This elevates the publish framework from "a P2 posting capability" to "a commercial interface between DailyOS and any external destination the user configures — including enterprise storage for reporting purposes."

**Scope correction — 2026-04-20 (outside voice finding #1):** Publish is **reporting/export**, not **team intelligence**. The earlier framing of this section implied publish was the complete answer to "enterprise wants leadership views." It isn't. Publish solves: "user creates a snapshot of their work, pushes it to a customer-controlled destination on a cadence they set." That is reporting. It does NOT solve: "a team sees the same live operational state about an account" — that is team intelligence and is a separate architectural question tracked in [ADR-0121](0121-team-intelligence-architecture.md) (Open). The publish framework remains the right channel for the reporting case; team intelligence is unsolved and explicitly acknowledged as such in [ADR-0116](0116-tenant-control-plane-boundary.md) "Update — 2026-04-20" section.

**Implications for this ADR:**

- The `PublishDestination` enum (§5) needs extensibility. S3, SharePoint, Confluence, and generic Webhook variants are plausible v1.4.2–v1.5.0 additions alongside P2. None of them should require an [ADR-0117](0117-publish-boundary-pencil-and-pen.md) amendment — the protocol is stable; destinations are extensions.
- Both human-readable (Markdown/PDF briefing export) and machine-readable (structured JSON with provenance) payload formats must be first-class. The `PublishPayload` shape (§1) supports both; formalize as required in v1.4.1.
- Scheduled publishes (user-set cron) are a legitimate Pencil/Pen variant: the schedule is the user's confirmation, with subsequent emissions requiring the Pencil/Pen contract but using a pre-configured `PolicySource::PrePolicy` token (§2). R1.6 said v1.4.0 hard-rejects `PrePolicy`; that stays true for v1.4.0–v1.4.1. Schedule-based publishes land with enterprise destinations in v1.5.0+ with appropriate authority-grant design.
- Retraction is honest per destination. Enterprise destinations typically do not support retraction (once pushed to S3, it's there). `RetractionSupport::None` applies; the user is told clearly before push.

**Non-goal:** This does not mean DailyOS builds every enterprise integration. It means the *protocol* supports user-driven push to any destination the user configures, and new destinations are a matter of writing a `DestinationClient` implementation — hours of work at AI velocity.

---

## Revision R1 — 2026-04-19 — Reality Check

Adversarial review + reference pass found the ADR's foundational claim — that `publish_to_p2` already exists — is false. The ADR is therefore not wrapping an existing capability; it is inventing the entire subsystem. Retarget accordingly.

### R1.1 Target version moved to v1.4.2

Ground truth: there is no `publish_to_p2` in the repo. There is no P2 API client, no draft/outbox tables, no destination client abstraction, no worker pool. The existing `export_data_zip` is the only external-write path and it is a one-shot local zip export.

**Revised target:** this ADR ships in **v1.4.2**, alongside the first real Publish ability. v1.4.0 ships only the **forward-looking interface definitions** (types, trait shapes, and boundary principles), and explicitly ships **no publish runtime**. This matches [ADR-0112](0112-migration-strategy-parallel-run-and-cutover.md)'s sequencing (Publish migrates last) and removes the inconsistency codex flagged.

The original §10 "Scope for v1.4.0" is retracted and replaced by:

- **v1.4.0:** types and trait shapes only. `PublishDraft`, `ConfirmationToken`, `PublishOutboxEntry`, `DestinationClient`, `RetractionSupport` defined in Rust. No tables. No workers. No delivery. Compile-only.
- **v1.4.1:** outbox tables, worker pool (shares infrastructure with [ADR-0115](0115-signal-granularity-audit.md) invalidation workers per R1.3), idempotency enforcement.
- **v1.4.2:** first real Publish ability (`publish_to_p2` or whatever replaces it), P2 API client, end-to-end Pencil/Pen flow.

### R1.2 Completion contract fix — `commit_publish` returns queue receipt, not `PublishedRecord`

Codex flagged: `commit_publish` writes an outbox entry and returns synchronously, but the original ADR implies the returned `PublishedRecord` includes destination refs (post ID). Those don't exist until delivery, which is async.

**Revised signature:**

```rust
pub struct PublishAccepted {
    pub outbox_entry_id: OutboxEntryId,
    pub draft_id: DraftId,
    pub expected_destination: PublishDestination,
    pub enqueued_at: DateTime<Utc>,
}

pub fn commit_publish(
    draft_id: DraftId,
    token: ConfirmationToken,
    ctx: &ServiceContext,
) -> Result<PublishAccepted, PublishError>;
```

`commit_publish` returns a receipt, not a publish record. A separate query — `get_publish_status(outbox_entry_id) -> PublishStatus` — returns the current delivery state. Only when status is `Delivered` does a `PublishedRecord` with destination refs exist. The contract no longer implies instantaneous publication.

Surfaces render the receipt as "Queued for delivery" and poll or subscribe to status changes. Matches what users actually experience — external delivery is not instant.

### R1.3 Reuse of [ADR-0115](0115-signal-granularity-audit.md) worker pool — conditional

Original §10 assumed the worker pool from [ADR-0115](0115-signal-granularity-audit.md) already existed. After R1 of that ADR it is acknowledged as new infrastructure with Phase 0 prerequisites. This ADR cannot reuse what hasn't landed.

**Revised:** worker pool dependency defers to v1.4.1. If ADR-0115 Phase 2 (durable invalidation jobs) has shipped by v1.4.1, this ADR's publish outbox workers are a second consumer of the same infrastructure. If not, a publish-specific worker pool ships alongside the outbox tables with an explicit "share with [ADR-0115](0115-signal-granularity-audit.md) when feasible" note.

### R1.4 Idempotency key fix

Codex flagged: `(ability_name, payload_hash, destination)` as the idempotency key loses correctness under legitimate re-publish (recipients change, payload edited, intentional republish).

**Revised:**

```rust
pub struct IdempotencyKey {
    pub ability_name: &'static str,
    pub ability_version: AbilityVersion,
    pub draft_id: DraftId,              // Stable per user-acted-on draft
    pub draft_version: u32,             // Increments on edit; legitimate republish after edit is distinct
    pub payload_hash: Hash,             // Paranoid consistency check
}
```

The key is per-draft, not per-payload. An edited draft is a new key. An unchanged draft re-submitted (retry) is the same key. Legitimate republish of identical content requires a new draft with the same payload — that is a design constraint, not a bug: "I meant to send this again" must be an explicit user action.

Destination clients treat the idempotency key as opaque and store it server-side where supported. Where not supported (plain SMTP), the local dedup table is keyed on the same `IdempotencyKey` hash.

### R1.5 Outbox state enum — add `Retracted`

Codex flagged: the original §3 state machine listed `Pending | Delivered | FailedRetryable | FailedPermanent`, but §4 retraction says the entry transitions to `Retracted`. Missing state.

**Revised:**

```rust
pub enum OutboxStatus {
    Pending,
    Delivered,
    Retracted,           // Destination-side deletion succeeded
    RetractionFailed,    // Destination-side deletion attempted and failed (rare; surface to user)
    FailedRetryable,
    FailedPermanent,
}
```

### R1.6 AgentWithAuthority — hard-reject in v1.4.0, even though the shape exists

Codex flagged an inter-ADR conflict with [ADR-0111](0111-surface-independent-ability-invocation.md), which states the MCP bridge refuses confirmation fields. The `AgentWithAuthority` shape in this ADR creates a policy-bypass surface.

**Revised:** in v1.4.0 and v1.4.1, every surface that invokes `commit_publish` **rejects** `ConfirmationToken` with `policy_source: AgentWithAuthority`. The shape exists in Rust for forward-compatibility; the validation gate in `commit_publish` returns `PublishError::AgentAuthorityNotYetSupported` if present. Tests cover both that the shape compiles and that invocation is rejected. First real implementation ships with the full authority-grant surface in v1.5.0 or later with a dedicated follow-on ADR.

### R1.7 Async destination client

Codex flagged: `DestinationClient::deliver` is sync, but external HTTP is async.

**Revised:**

```rust
#[async_trait]
pub trait DestinationClient: Send + Sync {
    async fn deliver(&self, payload: &PublishPayload, idem_key: &IdempotencyKey) -> Result<DestinationRef, DeliveryError>;
    async fn retract(&self, destination_ref: &DestinationRef) -> Result<RetractionOutcome, RetractionError>;
    fn retraction_support(&self) -> RetractionSupport;  // Sync — static capability query
}
```

Workers invoke this from an async runtime. The worker pool (shared with [ADR-0115](0115-signal-granularity-audit.md) or publish-specific) is async.

### R1.8 Draft retention and tenant policy

Codex flagged: drafts store potentially sensitive content for 365 days without tying to withdrawal, expiry, or deletion.

**Revised §1:** drafts have a TTL (default 24h, configurable per ability). Expired drafts auto-transition to `Expired` and their `payload` is null-masked after 7 days (retention window for audit — the fact the draft existed is retained, the content is not). Withdrawn drafts are null-masked immediately. Completed drafts (transitioned to `Committed`) retain payload for 365 days only if the ability declares a compliance-retention requirement; otherwise payload null-masked after 30 days.

### R1.9 Confirmation token security

Codex flagged: `ConfirmationToken` as a struct is not a security boundary.

**Revised:** the token is issued by a `ConfirmationBroker` service that:

- Generates a per-invocation `token_nonce` stored in a `confirmation_tokens` table.
- Records `issued_at`, `expires_at` (default 5 minutes).
- Enforces single-use: `commit_publish` atomically marks the token consumed.
- Validates `confirmed_by` actor against the invoking session identity.

Tokens outside the broker (bare struct construction) fail validation. The broker is the narrow seam where session identity, actor permission, and one-time-use enforcement converge.

### R1.10 Ground-truth reconciliation

- `publish_to_p2` does not exist. ADR is retargeted to v1.4.2 for its implementation; v1.4.0–v1.4.1 lay foundations.
- `export_data_zip` at `src-tauri/src/export.rs` is the only existing external-write path and should be reviewed for compliance with this ADR's `PublishDestination::Export` variant as a first consumer — that verification is a v1.4.1 task.
- No outbox, client, or worker code exists. All net new. Sized accordingly.

### R1.11 Scope for v1.4.0 — revised (minimal)

Ships:
- Rust type definitions: `PublishDraft`, `PublishOutboxEntry`, `OutboxStatus`, `ConfirmationToken`, `PolicySource`, `DestinationClient`, `RetractionSupport`, `IdempotencyKey`, `PublishAccepted`, `PublishError`.
- Compile-only — no tables, no service functions, no workers.
- Documentation reserving the protocol shape for v1.4.1–v1.4.2 implementation.

Deferred to v1.4.1:
- `publish_drafts`, `publish_outbox`, `confirmation_tokens` tables.
- `ConfirmationBroker` service.
- Worker pool integration.

Deferred to v1.4.2:
- First Publish ability implementation.
- P2 client.
- End-to-end Pencil/Pen flow.
