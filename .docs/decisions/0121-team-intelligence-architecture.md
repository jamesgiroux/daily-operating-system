# ADR-0121: Team Intelligence Architecture — Open Strategic Question

**Status:** Proposed (Open) — no decision made; placeholder framing the problem space
**Date:** 2026-04-20
**Target:** v1.5.0+ research spike; decision required before any team/enterprise tier ships
**Origin:** Codex outside voice on aggregate v1.4.0 plan, finding #1; founder decision 2026-04-20 ("Codex is right — revisit")
**Related:** [ADR-0116](0116-tenant-control-plane-boundary.md), [ADR-0117](0117-publish-boundary-pencil-and-pen.md), [ADR-0113](0113-human-and-agent-analysis-as-first-class-claim-sources.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)

## Context

DailyOS v1.4.0 architecture commits to per-user SQLite forever (founder decision D1, 2026-04-20). Content is stored locally on the user's device; no server-side component holds user data. The publish framework ([ADR-0117](0117-publish-boundary-pencil-and-pen.md)) is the commercial channel for external reporting — user-initiated manual push (or user-scheduled push) to customer-controlled destinations like S3, SharePoint, Confluence, or webhooks.

The initial framing of [ADR-0117](0117-publish-boundary-pencil-and-pen.md) (Strategic Elevation section, 2026-04-20) described publish as "the commercial interface between DailyOS and any external destination the user configures — including enterprise storage for reporting purposes" and implied it was DailyOS's answer to enterprise demands for leadership visibility.

Codex adversarial review (2026-04-20) challenged this framing:

> Publish is not team intelligence. It is reporting. If the substrate succeeds, DailyOS creates high-value personal truth graphs trapped in per-user SQLite. Enterprise buyers will want shared operational truth, not scheduled user exports to S3/SharePoint. The plan has converted "team intelligence" into "users manually or periodically push summaries elsewhere." That is a product downgrade hidden as an architecture principle.

Founder accepted the finding: publish solves *reporting*, not *team intelligence*. [ADR-0117](0117-publish-boundary-pencil-and-pen.md) has been corrected to remove the overclaim. [ADR-0116](0116-tenant-control-plane-boundary.md)'s Founder Commitment section acknowledges the tension.

This ADR is the placeholder. No decision is made here. The purpose is to **name the problem space**, enumerate **option classes**, and establish **the next-action trigger** so the team has a map when the pressure arrives.

## The problem

Three related but distinct questions:

1. **Shared operational truth.** A CS team has 10 accounts across 3 CSMs. At any moment, "what's true about Acme" should be one claim set, not three slightly-different local copies. A manager or teammate looking at Acme should see the same state the primary CSM sees. Today, per-user SQLite makes three independent truth graphs about the same account inevitable.

2. **Leadership views.** A VP of CS wants to see health scores, risk signals, and intervention status across all accounts their team owns. Publish can provide a snapshot report, but a *live view* that updates as CSMs interact with accounts requires shared state.

3. **Cross-user continuity.** A CSM leaves the company. Their accounts need to transfer to the new owner with full history intact. Today, their local SQLite goes with them (or stays encrypted on their device); there's no transfer mechanism that doesn't break [ADR-0116](0116-tenant-control-plane-boundary.md).

None of these are solved by publish. Publish solves: "CSM takes a snapshot and puts it in company-controlled storage on a cadence." That's a 2002-era answer to a 2026 problem.

## What [ADR-0116](0116-tenant-control-plane-boundary.md) + D1 *do* provide

Worth naming what's not in question:

- No server-side component of DailyOS sees user content. That commitment stands.
- Per-user local storage is the posture today and for the foreseeable v1.4.0/1.5.0 horizon.
- Publish framework handles reporting/export well; that shipping code path is real value.
- Enterprise security teams can see "DailyOS never sees your content on its servers" as a clean claim.

What's not in scope for this ADR's eventual decision: softening the metadata-only boundary. The decision is what *legitimate architecture* delivers team intelligence *while preserving* that boundary.

## Option classes

Each of these is a broad class; any specific architecture picks one or a hybrid. Listed for completeness, not ranked.

### Option A — Explicit user-to-user sharing, peer-to-peer

Users explicitly share specific entities (accounts, meetings, projects) with teammates. Sharing happens device-to-device or through a relay that never decrypts. Control plane coordinates sharing permissions (metadata only); content is encrypted end-to-end between users.

- Pros: full alignment with [ADR-0116](0116-tenant-control-plane-boundary.md). Users explicitly consent to share. Content never unencrypted on a server.
- Cons: complex crypto. Revocation is hard (a former teammate may have already synced a local copy). Doesn't solve leadership/cross-team views — only one-to-one or one-to-few sharing.

### Option B — Customer-operated shared storage (BYOK team pool)

The enterprise customer operates their own DailyOS instance on their own infrastructure (on-prem, private cloud, or BYOC). DailyOS clients sync to this customer-owned backing store rather than to each user's laptop. The customer's DB sees content, but DailyOS the vendor does not.

- Pros: enterprise control over their own data. No DailyOS-operated server sees anything. Full team intelligence.
- Cons: operational complexity for the customer. Deployment story is an extra product. Not applicable to small teams who don't want to operate infrastructure.

### Option C — Encrypted aggregation via zero-knowledge relay

A relay server shuffles encrypted claims between user devices without decrypting. Users who share accounts have a shared decryption key. The relay sees only encrypted blobs routed by team ID; content stays encrypted at rest on the server.

- Pros: DailyOS-hosted simplicity from user's perspective. Content is never unencrypted on DailyOS's server.
- Cons: crypto is hard to get right. Querying encrypted data is limited. "The relay sees encrypted blobs" is not the same as "stays on the laptop"; enterprise security review still has questions.

### Option D — Per-team tenant with hard-boundary tenant isolation

Each team gets a dedicated logical tenant. DailyOS operates servers but each tenant's data is cryptographically isolated; keys held only by the tenant's designated admins (BYOK per [ADR-0116](0116-tenant-control-plane-boundary.md) R1.5's future `TenantKmsWrapped`). DailyOS can't decrypt tenant data even with a subpoena.

- Pros: standard SaaS pattern. Clean answer to "where does my team's data live?" BYOK cryptographically enforces the boundary.
- Cons: requires amending D1 (per-user SQLite forever). Requires DailyOS to build + operate multi-tenant infrastructure. Significantly larger product.

### Option E — Defer; stay per-user indefinitely; tell enterprises no

DailyOS is a personal chief of staff. Team intelligence is out of scope. Enterprises that want team intelligence use a different product. The positioning is "DailyOS makes individual CSMs 10× better; your team still needs whatever central system you're using."

- Pros: maximum clarity. Preserves the architectural posture indefinitely.
- Cons: caps addressable market at individual-buyer segment. Leaves demand on the table. Potentially trappable by a competitor who does solve team intelligence.

### Option F — Hybrid: publish-plus-light-team-graph

Publish handles reporting. A lightweight team-level *graph* (not full claim data — just entity references and identity-level relationships) syncs through a metadata-only control plane. Team members see "James owns Acme; Sam owns Globex" level of coordination. Full intelligence stays per-user.

- Pros: incremental. Preserves [ADR-0116](0116-tenant-control-plane-boundary.md). Solves account-assignment and cross-team coordination without sharing claim content.
- Cons: doesn't solve shared operational truth (each CSM's claims about Acme are still their own). Is it enough?

## What's *not* a solution

Listed so they don't accidentally creep back in:

- **Softening [ADR-0116](0116-tenant-control-plane-boundary.md) without named compensating control.** The metadata-only boundary is a founder commitment. Any solution that requires DailyOS servers to see user content requires founder approval + explicit amendment.
- **Inferring team state from anonymous telemetry.** The ADR-0120 opt-in aggregate telemetry path is for product quality measurement, not team intelligence. Using it for team intelligence would violate its bounding.
- **"Just use Salesforce."** Customers already do. DailyOS's value prop is what Salesforce doesn't give them. Delegating team intelligence back to Salesforce is giving up the thing that was differentiated.

## Next-action triggers

This ADR stays Open (no decision) until one of:

1. **First enterprise conversation** specifically asking for leadership views or shared operational truth. At that point, file a research spike issue, interview the prospect to understand what they mean by "team intelligence," and start a focused design cycle.
2. **Second prospect raises the same objection.** Pattern is established; time to design.
3. **v1.5.0+ planning cycle** — if we're past v1.5.0 without triggering 1 or 2, revisit whether this is still a real market signal or whether DailyOS has validated the personal-chief-of-staff positioning alone.

Until then: the substrate continues its current trajectory. Publish framework ships. Per-user SQLite persists. Team intelligence is acknowledged as an open strategic question, not a solved problem.

## Consequences

### Positive

- Strategic honesty. Publish is no longer overclaimed as the enterprise answer; the team intelligence question is explicitly open, with option classes mapped, so it's not caught flat-footed when pressure arrives.
- Preserves substrate investment. Per-user SQLite, claims substrate, provenance, Trust Compiler — all valuable regardless of which option class eventually lands for team intelligence.
- Enables informed pushback. "DailyOS doesn't solve team intelligence today" is a defensible product positioning if the individual chief-of-staff story is strong enough.

### Negative / risks

- Leaves a commercial gap. If team intelligence is the actual enterprise need, DailyOS is blocked at the individual-buyer ceiling until a decision lands.
- Invites premature pivoting. Each enterprise conversation may push toward a different option class; discipline required to collect signals before deciding.
- "Open" status has a half-life. If this ADR stays Open for 18 months while substrate proceeds, the cost of picking an option grows (more patterns to retrofit). Trigger #3 (v1.5.0+ planning cycle) is the hard deadline for reassessment.

### Neutral

- v1.4.0 substrate work is unaffected. This is strategic framing, not architectural dependency.
- [ADR-0116](0116-tenant-control-plane-boundary.md) and [ADR-0117](0117-publish-boundary-pencil-and-pen.md) remain as-is (with amendments applied 2026-04-20). This ADR is forward-looking only.
