# ADR-0116: Tenant Control Plane Boundary — Metadata Only, Content Never

**Status:** Proposed
**Date:** 2026-04-19
**Target:** v1.4.0 historical DB-key seam (retired by v1.4.9) / v2.x (control plane activation)
**Extends:** ADR-0092 historically; v1.4.9 supersedes the DB-key storage contract
**Related:** [ADR-0099](0099-remote-first-server-canonical-architecture.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md)
**Consumed by:** [DOS-234](https://linear.app/a8c/issue/DOS-234) historical DbKeyProvider seam + LocalKeychain default

## Context

DailyOS today is a single-user native macOS app. User content is stored locally in the app's SQLite database and workspace files; v1.4.9 retired the active DailyOS-managed DB-key contract, so encryption-at-rest claims depend on the user's OS/FileVault posture rather than a DailyOS-managed DB key. There is no server-side component holding user content. This lets us tell a prospective customer "your content stays on your laptop" as a precise claim about *DailyOS's own server-side components* (which are none, in v1.4.0).

**Precise wording required — 2026-04-20 update (outside voice finding #2).** "Stays on your laptop" is not accurate without qualification when Glean is the intelligence provider (per [ADR-0100](0100-glean-first-intelligence-architecture.md)). Glean's MCP chat tool receives prompts, returns completions, and sees the user's Glean-connected source data (REDACTED, Zendesk, Gong, Slack) in the Glean infrastructure the customer has already contracted with. That is not DailyOS control-plane — it is the user's pre-existing relationship with Glean — but it's also not "on the laptop."

The precise claims that survive enterprise security review:

- **DailyOS's own server-side components** see no user content (not now; never without ADR amendment).
- **DailyOS's local app** stores user content on the user's device; local at-rest protection is the OS/FileVault/file-permission boundary, not a DailyOS-managed DB key.
- **Third-party intelligence providers** (Glean today, Anthropic/OpenAI/Ollama potentially via [ADR-0091](0091-intelligence-provider-abstraction.md)) see whatever the user's session sends them — prompts, partial context, completions. The user contracts with these providers independently. DailyOS routes to whichever provider the user has configured; DailyOS itself never stores the provider's responses beyond what lives locally.

Sales conversations should use the precise claim ("DailyOS never sees your content server-side; your content is stored locally on your device; intelligence computation runs through providers you already trust"), not the shorter claim that was imprecise about Glean's role.

Two forces are now pushing on that posture simultaneously:

1. **Enterprise storage control.** When DailyOS sells to enterprise, customers may require tenant-controlled at-rest policy and revocation semantics. The v1.4.0 DB-key lease design is no longer the active answer: v1.4.9 stores local state in plain SQLite under the OS/FileVault/file-permission boundary. Any future BYOK or cryptographic revocation model requires a fresh ADR amendment.
2. **Multi-user coordination.** Features like cross-analyst claim review ([ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md) human claim sources), shared team intelligence, and cross-device sync require some server-side component that knows "these two sessions belong to the same org." That component is a control plane.

Both forces push toward a control plane existing. The architectural question is: **what can the control plane see, and what must it never see?**

If DailyOS is not careful here, the answer drifts. A multi-tenant server gets stood up to coordinate logins. Then someone adds a "team intelligence" feature that routes claims through the server for cross-analyst visibility. Then the server is caching user content "for performance." Six months later the "your content stays on your laptop" promise is technically false. This happens to many local-first products.

This ADR establishes the boundary now, before the control plane exists, so that every future control-plane feature is measured against a hard rule. The historical v1.4.0 DB-key seam described below is retained for traceability, but v1.4.9 retires it as active storage substrate.

## Decision

### 1. The boundary — metadata only, content never

The control plane — whatever server component DailyOS eventually operates for authentication, licensing, cross-device coordination, or enterprise administration — sees **metadata only**. It never sees user content.

**Metadata** (permitted on the control plane):
- User identity: stable user ID, email, organization ID.
- Session identity: device ID, session token, expiry, revocation state.
- Capability grants: ability categories allowed for this user / org.
- License state: plan tier, seat count, entitlements.
- Audit facts: counts of ability invocations per day, aggregate cost signals. Counts, not contents.
- Storage-policy metadata: local storage posture, policy version, rotation timestamp where applicable. **Never user content or key material.**

**Content** (forbidden on the control plane, now and in all future iterations):
- Any row from any local table: `intelligence_claims`, `signal_events`, `meetings`, `accounts`, `persons`, everything.
- Any ability output or Provenance envelope.
- Any prompt, completion, or LLM response body.
- Any transcript, email body, document, Slack message.
- Any free-text user input beyond what is required for authentication.
- Any aggregate derived from user content (e.g., "this user has 47 claims about Acme").

The rule is binary. A feature that needs a content-side operation performs that operation locally on the user's device; the result stays local. The control plane may be told "user completed task X" with no payload; it may not be told "user's task X produced output Y."

The one permitted aggregate crossing the line is operational telemetry — how many abilities invoked today, approximate token counts, error rates. These are counts with no entity references. Rendering decisions, content previews, "recent activity" feeds — none of these live on the control plane.

**Permitted: user-opted-in anonymous aggregate telemetry (new category, 2026-04-20).** Per the outside voice finding that population-level metrics are needed to validate the harness bet, this ADR permits a specific additional class: telemetry the user explicitly opts into, containing only counts and category flags (never entity references, never content, never claim text), keyed on a non-reversible anonymized install ID. Examples of permitted metrics in this class:

- Per-ability invocation count, duration p50/p95, outcome distribution (success/error/warn).
- Runtime evaluator composite score distribution per ability (no critique text, no output hashes).
- Per-signal-type emission counts.
- Glean path availability and p95 latency.
- Ghost-resurrection incident count (just the count; no context).
- Anonymized install ID (a random UUID generated once at first boot, stored locally; never tied to user identity).

Opt-in is off by default. Users must explicitly enable via a settings UI that explains what's sent and what isn't. Enabling triggers a visible indicator. Disabling is always available and takes effect immediately.

**Forbidden even in opt-in mode:**

- Anything content-derived beyond counts (e.g., "claim body lengths" is forbidden even if anonymized; only "claim count" is permitted).
- Anything that could reverse-engineer entity identity (e.g., "unique account name count" is OK; "hash of account names" is not).
- Anything tied to user identity, email, or stable non-anonymous ID.

Classification rationale: this is metadata about *product usage*, not metadata about *user entities*. The distinction in §3 already separated metadata about user actions (permitted) from metadata about entities (local-only). Opt-in aggregate telemetry falls into the former — it's about user actions in aggregate, not about the entities they act on.

### 2. Why this boundary holds under future pressure

The ways local-first promises erode are predictable:

- **Feature convenience.** "Just cache the latest briefing on the server so users can preview it from the web." No — render it locally when the user opens the web surface, or accept that the web surface is login-only.
- **Debugging convenience.** "Let engineers view user sessions to help with support." No — support uses screenshares or user-exported redacted diagnostics. The server never sees content.
- **ML convenience.** "Aggregate user corrections to improve prompts." No — user corrections stay on the user's device; improvements come from prompt rebaselining against evaluation fixtures ([ADR-0110](0110-evaluation-harness-for-abilities.md)), not from content aggregation.
- **Enterprise admin convenience.** "Let the org admin see what their team is working on." No — admins get capability grants (who can do what) and audit counts (how often), not content.

Every future ADR that proposes a control-plane feature must cite this ADR and demonstrate that no content crosses. A review convention: if the server code can deserialize a user's entity, the design is wrong.

### 3. What the control plane is allowed to do

For clarity, these are explicitly permitted server-side capabilities, all metadata-only:

- Authenticate a user and issue a session token.
- Verify a session token and return user identity / org / capability grants.
- Store enterprise storage-policy metadata (policy version, rotation timestamp, revocation list where applicable — not user content or key material).
- Issue license / entitlement information.
- Revoke a user's session or capability grants without reading local content.
- Collect aggregate operational telemetry (counts, error rates, no entity references).
- Coordinate device enrollment (device ID, public key, enrollment timestamp).

Anything beyond this list requires an ADR amendment.

### 4. Local storage substrate status (v1.4.9)

The v1.4.0 `DbKeyProvider` seam was a historical plan for DailyOS-managed DB-key storage. It is not an active v1.4.9 contract. DailyOS now stores local state in plain SQLite and relies on the user's OS/FileVault/file-permission boundary for at-rest protection.

Future enterprise BYOK, tenant KMS, hardware-token, or cryptographic offboarding designs must land through a new ADR amendment. They cannot cite the v1.4.0 DB-key seam below as shipped substrate.

### 5. Schema stability

The active v1.4.9 invariant is simpler: local SQLite schema changes are driven by product data needs, not by DB-key provider swaps. Storage-policy metadata may exist in a future control-plane design, but it must not cause user content to cross the metadata-only boundary.

### 6. Revocation

In v1.4.9, revocation is session/capability revocation, not cryptographic DB-key lease revocation. A future enterprise cryptographic revocation model requires a new ADR amendment with explicit user-content, local-storage, and control-plane boundary analysis.

### 7. Execution mode interaction

Under `ExecutionMode::Evaluate` ([ADR-0104](0104-execution-mode-and-mode-aware-services.md)), the DB is either an in-memory SQLite fixture or an isolated test database. Tests do not hit the Keychain or any real control plane.

Under `ExecutionMode::Live`, local state uses the v1.4.9 plain SQLite storage boundary unless a future ADR explicitly replaces it.

### 8. Out of scope for v1.4.0

- No control plane implementation.
- No KMS integration.
- No session management changes beyond what already exists.
- No multi-user features.
- No cross-device sync.
- No administrative surfaces.

The v1.4.0 DB-key seam plan is historical. The active boundary (§1) remains: the control plane may coordinate metadata, sessions, and capabilities, but it must not read user content.

## Consequences

### Positive

- **The boundary exists in writing before it is tested in practice.** Future feature proposals have a rule to be measured against. "The server never sees content" is a single sentence that stops a large class of compromises.
- **Storage-policy changes require explicit design.** Future BYOK or cryptographic offboarding work cannot silently inherit an outdated DB-key seam.
- **Local-first promise is defensible.** We can point to this ADR when customers, reviewers, or future contributors ask how we enforce the boundary.
- **Schema stays focused on local product data.** Storage-policy metadata remains a separate concern from user content.

### Negative / risks

- **"Metadata only" is sometimes debatable in practice.** "Approximate token counts" could theoretically leak patterns if an attacker had side-channel access. Accepted tradeoff — operational telemetry is essential; if specific fields become risky, they get removed. Every new metadata field added to the control plane must be reviewed against this ADR.
- **Enterprise BYOK integration is future work with unknown surface area.** Real integration will require a v2.x ADR.
- **A future contributor may be tempted to "just add a small cache" on the server.** This ADR is the thing reviewers cite to reject that PR. The boundary is defended by convention backed by this document.

### Neutral

- No user-visible change in v1.4.9.
- No performance impact in v1.4.9.
- [ADR-0092](0092-data-security-at-rest-and-operational-hardening.md) historically governed DB-key storage; v1.4.9 supersedes that active contract with the local SQLite + OS boundary.
- [ADR-0099](0099-remote-first-server-canonical-architecture.md) — if/when a canonical server arrives, this ADR governs what it may do. The two ADRs are intended to compose.

---

## Founder commitment — 2026-04-20

The founder commits to the "metadata only, content never" rule as a firm boundary. Softening requires founder approval plus a named compensating control. The boundary is treated as load-bearing for the DailyOS strategy.

**Commercial corollary — how enterprise visibility gets served.** The anticipated enterprise objection ("leadership wants to see team activity") is answered structurally by the **publish framework** ([ADR-0117](0117-publish-boundary-pencil-and-pen.md)), not by softening this boundary. The publish flow is a **user-initiated manual push** to an enterprise storage layer (human-readable or machine-readable), configured by the user per destination. The push or sync setup is performed by the user; it is not the control plane reading user data. Enterprise storage is the user's sink, not DailyOS's outbound channel.

This means [ADR-0117](0117-publish-boundary-pencil-and-pen.md) is now strategically load-bearing for DailyOS's enterprise commercial story. It must support:

- User-configured enterprise destinations (S3, SharePoint, Confluence, custom webhook, etc.).
- Both human-readable (briefing Markdown / PDF) and machine-readable (structured JSON with provenance) output formats.
- Scheduled publishes as well as on-demand (scheduled remains user-initiated — the user sets the schedule).
- Clear visibility to the user that a publish is about to happen (Pencil/Pen protocol covers this).

A follow-on decision before the first enterprise conversation: what destinations ship in v1.4.2's first publish ability beyond P2. The publish framework in [ADR-0117](0117-publish-boundary-pencil-and-pen.md) R1.11 already supports this extension; enterprise destination set is a scope question, not an architectural one.

This commitment is not revisable by routine PR — amendment requires founder approval.

### Update — 2026-04-20 — Publish is reporting, not team intelligence (outside voice)

The codex outside voice on the aggregate v1.4.0 plan surfaced a sharper challenge: **publish is reporting, not team intelligence.** If DailyOS's substrate succeeds and produces trusted, provenanced, per-user customer intelligence, the enterprise demand will not be "let me export a snapshot" — it will be "let our team see the same account state." Publish (user-initiated, per-user, snapshot-based) does not solve that. Per-user SQLite forever (D1) + publish-as-export forecloses the shared-operational-truth path structurally.

The commitment to the metadata-only boundary stands. What changes: the **story about how enterprise visibility gets served** is acknowledged as incomplete. Publish is one path (reporting, exports, scheduled snapshots). A second path is needed for shared team operational truth — one that respects the boundary (content never leaves the device without user intent) but enables team-level composition of per-user state.

That second path is open architecture. Filed as [ADR-0121](0121-team-intelligence-architecture.md) (Open) as a forward-looking placeholder with the problem framing and option space. Implementation deferred; decision deferred. Acknowledged as strategic risk in the v1.4.0 strategy doc signature block.

**What does NOT change:**

- D1 (per-user SQLite forever) remains the v1.4.0/1.5.0 posture. Team intelligence architecture, if/when it ships, will not violate it without founder approval + compensating control.
- D2 (firm metadata-only commitment) remains.
- Publish framework ([ADR-0117](0117-publish-boundary-pencil-and-pen.md)) remains as the reporting/export channel.

**What does change:**

- ADR-0117's "Strategic Elevation" section is corrected to remove the overclaim that publish answers the full enterprise question (separate amendment to ADR-0117).
- Strategy doc records team intelligence as an open strategic tension, not a solved one.
- The next commercial touchpoint asking for team intelligence triggers [ADR-0121](0121-team-intelligence-architecture.md) work, not an ADR-0116 softening.

---

**v1.4.9 supersession note:** Revision R1 below is historical. Its DB-key provider, key-lease, key-cache, and key-metadata amendments document the v1.4.0 managed-key plan and are not active v1.4.9 storage requirements. Future enterprise BYOK or cryptographic revocation work needs a fresh ADR amendment.

## Revision R1 — 2026-04-19 — Reality Check

Adversarial review + reference pass confirmed the boundary principle is sound but flagged that the trait seam's "zero-impact" claim is wrong, the signature in the ADR doesn't match current code, and revocation is hand-waved for long-running desktop sessions.

### R1.1 Current signature — correct the seam shape

Ground truth from `src-tauri/src/db/encryption.rs:44`:

```rust
pub fn get_or_create_db_key(db_path: &std::path::Path) -> Result<String, String>
```

The function takes `&Path`, not `&UserIdentity`. Returns `Result<String, String>`, not `Result<EncryptionKey>`. There is a process-wide `OnceLock<String>` cache at line 30. The key is a hex string applied to SQLite via PRAGMA.

**Revised trait:**

```rust
pub trait DbKeyProvider: Send + Sync {
    fn get_or_create_key(&self, db_path: &Path) -> Result<DbKey, KeyError>;
    fn rotate_key(&self, db_path: &Path) -> Result<DbKey, KeyError>;
    fn invalidate_cache(&self) -> Result<(), KeyError>;
}

pub struct DbKey(Vec<u8>);  // Newtype; zeroized on drop

impl Drop for DbKey {
    fn drop(&mut self) { self.0.zeroize(); }
}

impl DbKey {
    pub fn to_pragma(&self) -> String { /* hex-encode for SQLite */ }
}
```

Key characteristics:

- Takes `&Path`, matching existing code.
- Returns a newtype `DbKey` wrapping `Vec<u8>`, not bare `String`. Addresses codex's zeroization concern — adopting the `zeroize` crate is a prerequisite.
- `invalidate_cache()` is new: allows forcing a re-fetch on the next DB open, required for long-running-session revocation (R1.4).

The existing `OnceLock<String>` process cache becomes the `LocalKeychain` impl's internal state. The trait does not prescribe caching.

### R1.2 Sync vs async trait — cannot commit to sync

Codex flagged: the trait is synchronous but enterprise KMS access is network-bound. If shipped sync, DB open blocks async runtimes or hides async work behind sync calls.

**Revised:** the trait is **split**:

```rust
pub trait DbKeyProvider: Send + Sync {
    fn get_or_create_key(&self, db_path: &Path) -> Result<DbKey, KeyError>;
    fn rotate_key(&self, db_path: &Path) -> Result<DbKey, KeyError>;
    fn invalidate_cache(&self) -> Result<(), KeyError>;
}

#[async_trait]
pub trait DbKeyProviderAsync: Send + Sync {
    async fn get_or_create_key(&self, db_path: &Path) -> Result<DbKey, KeyError>;
    async fn rotate_key(&self, db_path: &Path) -> Result<DbKey, KeyError>;
    async fn invalidate_cache(&self) -> Result<(), KeyError>;
}
```

`LocalKeychain` implements the sync variant (keychain access is local, fast, and synchronous). Future `TenantKmsWrapped` implements the async variant (KMS access requires network I/O). DB open sites that are currently sync continue to use `DbKeyProvider`; new async sites use `DbKeyProviderAsync`. An adapter `SyncFromAsync` blocks on a runtime handle when a sync site absolutely must call an async provider — documented as a last-resort and rare.

This is more honest than the original single-sync trait and makes the future KMS path feasible without re-architecting DB open.

### R1.3 DI migration is real — acknowledge and scope

Codex flagged: `ActionDb::open()` is zero-argument today; every call site (reference pass identified seven) self-provides the key. Forcing `Arc<dyn DbKeyProvider>` into every open is an app-wide DI migration.

**Revised:** v1.4.0 ships a **compatibility wrapper** rather than a forced migration:

```rust
pub fn open() -> Result<Self, DbError> {
    Self::open_with_provider(&default_provider())
}

pub fn open_with_provider(provider: &dyn DbKeyProvider) -> Result<Self, DbError> { /* new path */ }

fn default_provider() -> &'static dyn DbKeyProvider { &LOCAL_KEYCHAIN }
```

`ActionDb::open()` and `ActionDb::open_at()` remain zero-argument and call the default provider internally. Sites that want to inject a provider use `open_with_provider()`. This preserves the existing seven call sites unchanged.

`DbService::open_at` (reference pass identified as a separate pooled-connection path) gets the same treatment — a zero-argument variant plus an explicit-provider variant.

Forcing `Arc<dyn DbKeyProvider>` into every call site is a v2.x pre-requisite when the control plane first ships, not v1.4.0 work. Original §4 is revised accordingly.

### R1.4 Revocation for long-running sessions — invalidate the cache

Codex flagged: "next session start" does nothing for a user who leaves DailyOS open for a week. The cached key never clears.

**Revised §6:** the control plane emits a revocation signal to the client via an authenticated polling channel (v2.x; polling interval default 5 minutes). On signal receipt, the client calls `provider.invalidate_cache()` and closes the DB. Subsequent DB access triggers `get_or_create_key()`, which in the `TenantKmsWrapped` case queries the control plane, receives "revoked," and refuses to open. The user is logged out and the DailyOS window enters a "session revoked" state.

For v1.4.0 local-only use, the polling channel does not exist and `invalidate_cache()` is a manual invocation surface for user-triggered key rotation. The trait method exists; the control plane integration is future.

### R1.5 Metadata storage — acknowledge the schema cost

Codex flagged: key version, wrap-key ref, rotation state, rekey progress all need storage. The ADR's "no schema change ever" was too strong.

**Revised:** a small `db_key_metadata` table lands as part of the control-plane integration (v2.x), not v1.4.0:

```sql
CREATE TABLE db_key_metadata (
  key_id          TEXT PRIMARY KEY,
  provider_type   TEXT NOT NULL,     -- "local_keychain" | "tenant_kms_wrapped" | ...
  wrapping_key_ref TEXT,              -- KMS key identifier if TenantKmsWrapped
  version         INTEGER NOT NULL,
  rotated_at      TIMESTAMP NOT NULL,
  rekey_in_progress BOOLEAN NOT NULL DEFAULT 0
);
```

This stores metadata about the key, not the key material. Consistent with the boundary: metadata on the control plane mirrors the local metadata; key material stays local.

The original "no schema change ever" claim is retracted. The accurate claim is: **the encryption-at-rest schema** (which columns are encrypted, which tables are encrypted) does not change when key providers change. Key metadata is a separate concern and gets its own small table when needed.

### R1.6 Test provider — don't let production pick unencrypted

Reference pass flagged: tests rely on `open_at_unencrypted`. The provider seam must preserve that without letting a production misconfiguration choose unencrypted.

**Revised:** `NoEncryption` is a distinct `DbKeyProvider` impl only compiled under `#[cfg(test)]` AND explicitly selected via a test helper. Production binaries never register it. A boot-time check panics if `NoEncryption` is active outside test builds.

### R1.7 Metadata boundary clarification — provenance and publish records

Codex flagged: [ADR-0105](0105-provenance-as-first-class-output.md) provenance includes actor metadata; [ADR-0117](0117-publish-boundary-pencil-and-pen.md) publish records include destination refs. This ADR didn't classify whether those are content, control-plane metadata, or local-only metadata.

**Revised §3 clarification:** provenance and publish records are **local-only metadata**. They live on the device, encrypted like any other claim data, and never traverse the control plane. "Local-only metadata" is a third category distinct from "permitted control-plane metadata" and "forbidden content." The principle: metadata *about a user action* (auth state, capability grants) is permitted on the control plane; metadata *about an entity or claim* is local-only and follows content encryption rules.

### R1.8 Scope for v1.4.0 — revised

Ships in v1.4.0:
- `DbKeyProvider` sync trait + `LocalKeychain` implementation with zeroize-on-drop `DbKey` newtype.
- `ActionDb::open_with_provider()` + `DbService::open_at_with_provider()` as new methods; zero-arg defaults unchanged.
- `invalidate_cache()` method exposed but no polling channel.
- `#[cfg(test)]` `NoEncryption` provider + boot-time guard.

Out of scope for v1.4.0:
- `DbKeyProviderAsync` trait — defined in this ADR but not implemented until a user shows up.
- `db_key_metadata` table — lands with `TenantKmsWrapped`.
- Control-plane polling channel — v2.x.
- Full DI migration of all call sites — v2.x.
