# v1.4.2 — Personal Intelligence Engine: WordPress Foundation

**Linear project name (for `save_project` `name` field):** `v1.4.2 — Personal Intelligence Engine: WordPress Foundation`

**Summary (≤255 chars, for `save_project` `summary` field):**
The DailyOS runtime renders into a local WordPress site as its primary composable surface. Entity intelligence, briefings, claim corrections — all become Gutenberg blocks the user reads, edits, and shares.

---

The text below is the Linear project `description` body. Paste verbatim.

---

## Wave Planning Reference

Wave plan template: [Wave Plan Template](https://linear.app/a8c/document/wave-plan-template-e063a52ef0b5)

Use Linear milestones named `Wave N - ...` as the containers for wave execution. Implementation sessions should read the template before drafting, revising, or executing milestone-level plans. Per-wave detail lives in `.docs/plans/dos-546/v1.4.2-project/03-wave-plan.md` in the repo.

## Roadmap Position

This comes after:

* `v1.4.0 — Abilities Runtime Spine`
* `v1.4.1 — Abilities Runtime Completion`

v1.4.0 shipped the substrate spine. v1.4.1 makes the substrate durable, trustworthy, and validated. v1.4.2 turns the substrate into a surface a user actually composes against. Per [ADR-0129](https://github.com/Automattic/dailyos/blob/dev/.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md), that surface is WordPress Studio — a local WordPress install rendering DailyOS Gutenberg blocks against the paired Rust runtime.

This project **supersedes** the original v1.4.2 ("Entity Intelligence — Accounts, Projects, People on Tauri React surfaces"). Entity surfaces are still in the program; they ship on the WordPress foundation this project builds, in a later release (currently planned for v1.4.3 reframed). The Tauri-shaped entity-page work is parked — not abandoned — pending the surface foundation.

## Mission Gate

Every issue should pass the roadmap question:

> Does this make DailyOS better at maintaining and updating the user's working understanding of their professional world?

For v1.4.2, the answer should usually be: yes, because it makes the substrate composable, editable, and shareable on a surface the user already lives in — turning latent runtime capability into rendered, correctable intelligence.

## User Outcome

When a user installs the DailyOS bundle (Studio + plugin + theme + runtime) on a clean machine and opens their local WordPress site, the experience is:

* The site loads in WordPress Studio (or any local WP 6.9+ install) with the DailyOS theme and blocks ready.
* The user runs the pairing flow once: a one-time pairing code printed by the DailyOS runtime, entered in the WP admin, completes the handshake.
* A daily briefing page renders as a composed set of DailyOS Gutenberg blocks — entity overviews, claim summaries, callouts, prep blocks — each produced by an ability and sourced from the substrate.
* Trust bands (`likely_current`, `use_with_caution`, `needs_verification`) render inline on every claim block; provenance is one click away.
* The user edits a block, dismisses a claim, or submits a correction. The edit travels back through the surface bridge as a typed feedback event, lands on the substrate via the existing claim/feedback path, and re-renders the block on save.
* Three views — substrate (source of truth), WordPress DB (projection), markdown filesystem (durable archive) — stay consistent under realistic editing patterns, including tab-switching and concurrent edits.
* The same runtime serves headless agents (Claude Desktop, Cursor) over MCP without WordPress in the loop, because the WordPress side is a SurfaceClient, not a runtime.
* Nothing in the WordPress site is allowed to act as the substrate. The runtime is the authority. The plugin is a rendering and feedback client.

This is the foundation. v1.4.3+ builds the rich entity, briefing, claim/trust, and salience surfaces on top of it — but every block, every theme treatment, every editorial pattern in those releases lands as a Gutenberg surface, not a Tauri React component.

## Scope — what lands here

### Surface foundation

* **DailyOS WordPress plugin** — the WP-side SurfaceClient. Pairing handshake, ability proxy registration via the WP Abilities API, HMAC-signed loopback transport, presence-nonce enforcement, save-time feedback routing, admin diagnostics.
* **Magazine theme** — the editorial container for DailyOS surfaces. FolioBar, FloatingNavIsland, AtmosphereLayer, MagazinePageLayout, FinisMarker. Tokens ported from `.docs/design/`. No new design-system primitives unless explicitly promoted.
* **First Gutenberg block** — `dailyos/account-overview`. Producer/renderer split per ADR-0130. The block stores attributes only; re-renders by invoking the ability on read; carries trust-band rendering, provenance refs, claim refs, fallback projection.

### Substrate-to-surface contract

* **`SurfaceClient` actor class** — promoted from the spike. ADR-0111 §8 fourth actor with per-instance identity, audit attribution, scope grants.
* **Canonical `AbilityPolicy` schema** — W0-D amends ADR-0102 §7.1 to (a) promote `mcp_exposure: bool` to the tri-state enum `McpExposure { None | MetadataOnly | Invocable }` and (b) keep `client_side_executable: bool` as a separate field governing SurfaceClient invocability per Phase 0 artifact 05 lines 389-412 (MCP exposure and SurfaceClient invocability govern different trust boundaries — an MCP tool may be invocable by an agent while a WP block hydrates a different ability that is never MCP-listed, and vice versa). W0-D also lands `required_scopes` as the two-level enforcement field. Two-level enforcement is enforceable substrate-side; the W1-B macro compile-error gate codifies the SurfaceClient + non-empty `required_scopes` discipline.
* **`Composition` contract** — ADR-0130. Substrate-owned block authorship; surface-independent shape; provenance-by-reference; custom block fallback projection rules.
* **Loopback HTTP runtime endpoint** — PHP-to-Rust transport. Bind to `127.0.0.1:<random>`, HMAC-SHA256 request signing, Host/Origin guards, pairing handshake, ability invoke, feedback submit. Per Phase 0 artifact 15.
* **Pairing model** — token recovery defenses per Phase 0 artifact 01: Reinstall, DB-Restore, Site-Switch, Exfiltration — four named threat paths, four discrete defenses.

### Custom MCP server

* **WordPress-mediated MCP server** — DailyOS configures the [WordPress MCP Adapter](https://github.com/WordPress/mcp-adapter) with an explicit DailyOS ability allowlist, a dedicated low-capability WP user for substrate access, and read-mostly defaults. Substrate-backed abilities are **never** exposed by the default WP MCP server.
* **Ability-surface inventory** — machine-readable catalog binding each DailyOS ability to its WP Abilities API registration, MCP exposure rule, scope requirements, and rendering copy. Per Phase 0 artifact 05.
* **CI gate on ability descriptions** — descriptions enter the PII blocklist + internal-vocabulary scan regime, same as committed source.

### Three-view consistency

* **Concurrency contract** — server-assigned monotonic `claim_version: u64`; watermarks on every projected block; stale-write rejection at the substrate boundary. Per Phase 0 artifact 02.
* **Tamper detection contract** — projection signatures on write; verification on read; out-of-band edits (direct WP DB rows, direct markdown edits, DB restores, SQL imports) detected and quarantined, never silently promoted to canonical. Per Phase 0 artifact 03.
* **Markdown filesystem as durable read-side archive** — substrate is the authority and writes the projected markdown file on claim change; WP DB is also a projection. In v1.4.2 the substrate→markdown write path is in scope (substrate emits markdown when claims change) and markdown→substrate read-side detection is in scope (W4-C signature verification quarantines out-of-band markdown edits). Bidirectional markdown-as-input reconciliation (edit a markdown file, substrate ingests) is explicitly v1.4.6 scope per the Workspace Memory Refactor project; v1.4.2 does NOT ship that ingestion path.

### Hardening

* **Rate-limit matrix** — multi-axis budgets enforced in `SurfaceClientBridge`: per-SurfaceClient instance, per-WP-user, per-WP-site, per-ability, per-scope-class. Per Phase 0 artifact 09.
* **User-presence nonce lifecycle** — 60-second, single-use, server-bound nonces for feedback writes. Bound to `(session, wp_user_id, claim_id, field_path, action)`. Per Phase 0 artifact 10.
* **Audit attribution** — every substrate operation log entry carries SurfaceClient instance identity AND WP `user_id` (or `null` for headless). Per /cso refinement 4.
* **Negative fixture catalog** — Phase 1 ships the named failure cases from Phase 0 artifact 12 as actual tests at the boundary that fails, not only as end-to-end checks.

### Runtime-host reorientation

* **Dev-mode runtime launcher** — the Tauri app continues to exist; for v1.4.2 it hosts the runtime, MCP server, cron, signal propagation, claim writes, OS keychain access, local privacy gate, dev/admin surfaces. Per Phase 0 artifact 04 — the architecture recommendation is "Tauri continues, UI role reorients." This release does NOT decide the long-term Tauri UI fate. That call is deferred to empirical evaluation after the WP surface stabilizes.
* **First-run flow** — install bundle → Studio profile or existing WP install → runtime launches → pairing code displayed → user enters in WP admin → daily briefing renders.
* **Clean-machine validation** — the free-tier capability claim (per ADR-0129 §5) is empirical, not aspirational: a clean macOS box can install the bundle and reach a rendered briefing without DailyOS-hosted infrastructure.

## Non-goals

* **Full entity intelligence detail pages** (rich Account/Project/Person pages, role changes, multi-project accounts, cross-entity contamination handling) → moves to v1.4.3 (reframed). v1.4.2 ships one entity block (`account-overview`) as proof-of-shape; the full entity surface set comes after the foundation.
* **Briefing experience composition templates** → v1.4.3 / v1.4.4.
* **Claim/trust inspection UI, Activity Log, Lint Mode** → v1.4.4.
* **Recommendation contract, salience scoring** → v1.4.5.
* **Proactive triggers / notifications** → v1.4.6.
* **BYOM agent backend changes** — the existing PTY-based intelligence provider continues unchanged. ACP-shaped agent backend lands in a later release.
* **Multi-tenant hosted substrate / paid-tier hosting** — explicit ADR-0129 §5 non-goal. Free-tier capability proves the substrate; paid-tier engineering follows once free-tier customers exist.
* **Additional `SurfaceClient` instances beyond WordPress** — Cursor, future PWA, future mobile use the Rust runtime's direct MCP server (already shipped in v1.4.0). No new SurfaceClient transports in this release.
* **Production-grade install signing for the bundle** — code-signing pipeline expansion is tracked in a companion ticket and may follow this release. v1.4.2 builds the install bundle and ships dev-mode signing.
* **Tauri UI deprecation decision** — per ADR-0129 §7 the Tauri UI's long-term role (deprecate, power-user surface, or thin admin/status) is explicitly deferred to empirical evaluation after WP stabilizes. v1.4.2 does not make that call.
* **Skillify / abilities-as-WP-plugins** — the architectural alignment is noted in ADR-0129; the actual Skillify shape lands in a later release (`DOS-540`).
* **Bidirectional markdown↔substrate ingestion** — v1.4.2 emits substrate→markdown on claim change and detects out-of-band markdown edits (W4-C). Markdown-as-input edit propagation (markdown file edited → substrate ingests + reconciles) is v1.4.6 (Workspace Memory Refactor).
* **Per-ability `required_scopes` population beyond v1.4.0/v1.4.1 defaults** — the W1-B schema lands the canonical fields and the macro compile-error for SurfaceClient-exposed abilities. Per-ability scope authorship (what scope each specific ability requires) lands per-ability in consuming releases (v1.4.3+), gated by the W1-B compile-error gate.

## Depends On

* `v1.4.0 — Abilities Runtime Spine` (shipped) — registry, provenance, claim substrate, trust compiler core, MCP bridge.
* `v1.4.1 — Abilities Runtime Completion` (in flight) — durable signal infrastructure, scoring/trust depth, capability migrations, validation suite, eval harness. **Must merge before v1.4.2 wave kickoff.** v1.4.2 wires the surface foundation onto the completed substrate; partial substrate produces partial blocks.

## Supersedes

This project supersedes the original v1.4.2 (Linear ID `33411e87-987a-4bd0-8c88-1e9cc2a920d2`, "v1.4.2 — Entity Intelligence (Accounts, Projects, People)"). The original was scoped against a Tauri React surface. Per [ADR-0129](https://github.com/Automattic/dailyos/blob/dev/.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md) the entity surface work shifts onto the WordPress foundation this project builds.

**Migration of the old v1.4.2 scope:**

* Entity detail behavior (account, project, person), shared entity references, claim rendering on entity surfaces, user correction loops, ambiguous-identity fixtures → **routed to v1.4.3 (reframed) on the WP foundation**.
* The wave-shape, milestones, and reviewer matrix conventions from the original v1.4.2 plan stay as a reference for v1.4.3 scoping.
* No issues from the old v1.4.2 are deleted; James will route them to the parking-lot project (`v1.4.3 — Entity Intelligence on WP` or equivalent, created separately) when the foundation lands.

## Relationship to other projects

* `v1.4.0 — Abilities Runtime Spine` provides the registry/provenance/claim/trust foundation this project renders.
* `v1.4.1 — Abilities Runtime Completion` provides the durable substrate this project renders against.
* `v1.4.3 (reframed) — Entity Intelligence on WP` will consume the foundation this project ships.
* `v1.4.4 — Claim Experience & Trust UI` will land as Gutenberg block primitives over this foundation.
* `v1.4.5 — Salience & Recommendations` will drive block visibility and ordering on this foundation.
* `v1.4.6 — Workspace Memory Refactor` will own the markdown ↔ WordPress DB reconciliation that this project bootstraps.
* `v1.4.7 — MCP Server v2` will deepen the cross-surface story; the substrate-to-surface contract this project commits to is its load-bearing input.

## Definition of Done

1. Clean-machine install: macOS test box with no prior DailyOS install reaches a rendered daily briefing in WordPress Studio in under 15 minutes of total user time, with pairing as the only manual configuration step.
2. First Gutenberg block (`dailyos/account-overview`) renders end-to-end against a real account fixture with trust bands, provenance refs, and claim refs visible; correction submitted from save handler routes through the runtime feedback path; the block re-renders with the corrected state.
3. Three-view consistency: substrate is the authority, WP DB is a projection, markdown is a durable read-side archive. Edit-in-WP and concurrent-tab scenarios from Phase 0 artifact 02 negative fixtures all pass; out-of-band edits in either WP DB or markdown from Phase 0 artifact 03 are detected, quarantined, and never silently promoted. Bidirectional markdown↔substrate edit propagation is named as v1.4.6 (Workspace Memory Refactor) scope and is NOT a v1.4.2 DoD criterion.
4. Two-level enforcement: ability invocation outside `required_scopes` is rejected at `SurfaceClientBridge` before registry lookup; introspection outside `mcp_exposure` policy returns no ability metadata; both verified by negative tests from Phase 0 artifact 12.
5. WP MCP Adapter only exposes the explicit DailyOS allowlist; the dedicated low-capability WP user for substrate access cannot reach abilities outside the allowlist; verified by negative tests from Phase 0 artifact 12.
6. Pairing recovery: all four named threat paths (Reinstall, DB-Restore, Site-Switch, Exfiltration) ship with their named defense and a negative fixture per Phase 0 artifact 01.
7. Rate-limit matrix enforced in `SurfaceClientBridge`: each axis (instance, WP-user, WP-site, ability, scope-class) has a rejection test from Phase 0 artifact 09 plus its `429` envelope.
8. Audit log carries SurfaceClient instance identity + WP `user_id` for every WP-originated operation. The schema + emission contract land in W1-A0 (so every W2+ surface emits the canonical shape from inception); the forensic round-trip exercise + CI lint complete in W6-A. Manual audit-trace exercise reproduces the originator of a sample correction end-to-end.
9. Ability-description CI gate active: PII blocklist + internal-vocabulary scan blocks committed descriptions that violate the regime.
10. Negative fixture catalog from Phase 0 artifact 12 implemented; all named failure cases pass at the boundary that fails.
11. `cargo clippy -D warnings && cargo test && pnpm tsc --noEmit` green. WordPress plugin lints green (PHP_CodeSniffer per WP Coding Standards). Theme passes WP Theme Check.
12. Release gate: `pnpm release-gate -- --mode hermetic` exits zero against bundles 1-24 (1-18 from v1.4.1; 19-24 from v1.4.2: bundle 19 pairing+HMAC, 20 composition+watermark+tamper, 21 rate-limits+nonce, 22 custom block fallback, 23 MCP allowlist + ability discovery, 24 audit attribution).
13. Manual dogfood evidence captured against ≥10 real-dev briefings rendered in WordPress.
14. Brand/positioning reframe captured: README, marketing-adjacent copy, and onboarding flow describe DailyOS as "your personal intelligence runtime" rather than "a Mac app for Customer Success."
