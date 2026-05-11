# DOS-546 — L0 Prep Codex Review

**Phase:** L0 Prep (pre-formal-L0 plan hardening)
**Reviewer:** Codex via `/codex` skill, consult mode with adversarial framing
**Date:** 2026-05-10
**Session:** `019e1393-e600-7573-ad6f-5517ea91554a` (saved at `.context/codex-session-id`)
**Tokens used:** 186,726
**Subject:** [DOS-546](https://linear.app/a8c/issue/DOS-546) — WordPress Studio surface viability spike, validating [ADR-0129](../decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md)

## Summary of decisions on findings

Codex returned 11 P1 (Critical) and 8 P2 (Important) findings. Verdict: *not L0-ready as currently shaped.*

After review, James accepted most findings and dismissed the timebox-related ones as not load-bearing given parallel-codex execution capacity. Decisions:

| Finding cluster | Decision | Action |
|---|---|---|
| Auth/trust model undefined (P1 ×3) | **Accepted.** A malicious WP plugin should not be able to push content. | Folded into DOS-546 Phase 0 design artifacts. New `SurfaceClient(WordPress)` actor class required pre-kickoff. |
| PHP MCP client maturity v0.5.0 (P1) | **Accepted as substantive concern, dismissed as time concern.** | Folded into Phase 0: required transport-comparison artifact (MCP, Unix socket, loopback HTTP, stdio). |
| MCP session lifecycle inside WP request loop (P1) | **Accepted.** | Folded into Phase 1 measurement matrix. |
| Three-view consistency has no conflict-resolution contract (P1) | **Accepted.** | Folded into Phase 0: written consistency contract required pre-kickoff. |
| Direct markdown edits as second write ingress (P1) | **Accepted.** | Folded into Phase 0 consistency contract. |
| Polling unacceptable for trust-bearing UX (P1) | **Accepted.** | Folded into Phase 0: event-bus primary, watcher secondary, polling rejected. |
| Negative fixtures missing (P2) | **Accepted.** | Added to Phase 1 acceptance criteria. |
| 1-2 vs 2-4 week timebox conflict (P1) | **Dismissed.** James has unlimited Codex token capacity; parallel session execution makes wall-clock concerns irrelevant. | Timebox section removed from DOS-546. |
| AC4 timebox breaker (P1) | **Dismissed as time concern, accepted as scope.** | Production-grade install split into companion ticket; spike does dev-mode launcher. |
| WP plugin = 5d not 2d (P2) | **Dismissed.** | No timebox in spike. |
| Markdown reconciliation can't be answered in 1-2 weeks (P2) | **Accepted as design dependency.** | Consistency contract required pre-kickoff (Phase 0). |
| Tauri removal observational (P1) | **Accepted.** | Folded into Phase 0: runtime-host inventory required artifact. |
| Launchd packaging is its own problem (P1) | **Accepted.** | Production packaging split into companion ticket. |
| Tauri as supervisor option (P2) | **Accepted.** | Phase 0 inventory must produce a recommendation across three named options. |
| Make a call on minimum runtime (P2) | **Accepted.** | Spike exit artifact must include a yes/no on Tauri-as-runtime-host for v1.4.2. |
| "Authenticates as user" undefined (P1) | **Accepted.** | Phase 0: SurfaceClient actor class with scoped abilities, pairing, expiration. |
| /cso review mandatory (P1) | **Accepted.** | Moved from open question to L0 hard requirement. |
| Same-process WP plugin trust collapse (P1) | **Accepted.** | Phase 0 threat model must define hostile-plugin scope. |
| Feedback prompt-injection boundary (P2) | **Accepted.** | Typed feedback events with user nonce, claim version, field path, before/after required. |
| Studio doesn't validate "any local WP install" (P1) | **Accepted.** | Acceptance split: Studio-only pass + arbitrary-local-WP pass. |
| Free-tier validation not covered (P1) | **Accepted.** | Phase 2 added: free-tier clean-machine validation. |
| Onboarding deferred (P2) | **Accepted.** | Phase 1 includes minimal activation flow artifact. |
| Hold biases toward confirming pivot (P1) | **Accepted.** | Explicit cost-based backout thresholds added to exit options. |
| Projection ownership question missing (P2) | **Accepted.** | Phase 0 WP write class taxonomy required artifact. |
| Feedback-only conflicts with Gutenberg editability (P2) | **Accepted.** | Phase 0 write class taxonomy resolves this. |

Net: substantive findings folded into DOS-546 as Phase 0 (pre-kickoff design), Phase 1 (empirical spike), Phase 2 (free-tier clean-machine validation). Time-related findings dismissed because parallel Codex execution capacity removes wall-clock as a binding constraint.

## Post-review research correction (2026-05-10)

After James flagged that Codex misread two transport-related concerns, primary-source research verified the actual state. Findings:

- **WordPress Abilities API merged into WordPress core in 6.9 (December 2025).** The `WordPress/abilities-api` GitHub repo was archived 2026-02-05 because it was promoted to core, not abandoned. Client-side Abilities API merges into Gutenberg for WordPress 7.0 (shipping 2026-05-20). Sources: [Developer Blog announcement](https://developer.wordpress.org/news/2025/11/introducing-the-wordpress-abilities-api/), [WordPress AI Handbook](https://make.wordpress.org/ai/handbook/projects/abilities-api/), [Client-Side API in 7.0](https://make.wordpress.org/core/2026/03/24/client-side-abilities-api-in-wordpress-7-0/).
- **WordPress MCP Adapter ships as a plugin** (`github.com/WordPress/mcp-adapter`). Bridges Abilities API to MCP. Registered abilities become MCP tools automatically. Source: [Developer Blog announcement](https://developer.wordpress.org/news/2026/02/from-abilities-to-ai-agents-introducing-the-wordpress-mcp-adapter/).
- **The Codex finding on PHP MCP SDK maturity is moot under the corrected architecture.** DailyOS does not need a PHP MCP client; we register abilities into WP's Abilities API (in core) and let the MCP Adapter expose them. The Codex finding remains accurate as a statement about the PHP MCP SDK itself (which is genuinely experimental per its PHP Foundation + Symfony maintainers), but it is not architecturally relevant to this spike.
- **Versioning correction lands partly:** the v0.x label on the Abilities API was a maturing-into-core path (WordPress AI Building Blocks initiative), so v0.x there was not a robustness signal. The PHP MCP SDK at v0.5.0 is separately maintained by PHP Foundation + Symfony; its "experimental" README label is from those maintainers, not a WordPress versioning convention. Both true; the architectural change makes the PHP MCP SDK question moot regardless.
- **Citation correction:** an earlier version of the DOS-546 body cited a `make.wordpress.org/core/2025/06/06/...` URL that returned 404. That speculative citation has been removed and replaced with verified primary sources.

The substantive structural concerns Codex raised (session lifecycle inside WordPress request loop, SSR latency under N tool calls, request isolation, malicious-plugin trust collapse) still apply against the local PHP-to-Rust transport even with the corrected architecture. The composite-ability discipline (one ability call per render) and the measurement matrix remain required deliverables.

## Verbatim Codex output

What follows is the raw adversarial review Codex produced. Preserved unedited as the canonical artifact.

---

**1. Transport Call: MCP-As-Seam**

[P1] PHP MCP client maturity is not enough to make MCP-as-seam a production assumption
Where: ADR-0129 §4, `.docs/decisions/0129...md:72-78`; DOS-546 Outcome 1 / Approach sketch
Why it matters: The official PHP SDK exists and includes client support, but it is still pre-1.0 and explicitly experimental: latest GitHub release shown as `v0.5.0` on April 26, 2026, and README says "Until the first major release, this SDK is considered experimental." WordPress.com/Automattic are shipping MCP mostly as WordPress-as-server for AI agents, not WordPress plugin-as-client to a local Rust MCP runtime. That is the opposite direction from this spike.
What to do: Change the spike from "WP plugin opens an MCP session" to "prove or reject PHP MCP client viability under WP request lifecycle." Add a fallback transport comparison as a required artifact, not an open-ended note. Sources: [official PHP MCP SDK](https://github.com/modelcontextprotocol/php-sdk), [WordPress.com MCP support](https://wordpress.com/support/model-context-protocol-mcp-settings/), [Automattic MCP page](https://automattic.ai/mcp/).

[P1] The spike assumes an MCP transport but does not define the session lifecycle inside WordPress
Where: DOS-546 Approach sketch; ADR-0129 `.docs/decisions/0129...md:74-78`; ADR-0111 `.docs/decisions/0111...md:70-96`
Why it matters: WordPress SSR is PHP request/response. If every Gutenberg render initializes MCP, lists tools, calls one or more tools, and disconnects, page latency will be dominated by per-request handshakes and blocking I/O. If the plugin tries to keep a long-lived MCP client, standard PHP-FPM/request isolation fights it. ReactPHP helps inside one request, not across WP requests.
What to do: Add a measurement matrix: persistent vs per-request client, server-side render vs client fetch, 1/3/10 tool calls, cold/warm runtime. Require a composite `account_overview` ability so the block performs one MCP call, not N calls.

[P1] MCP-over-localhost breaks at the malicious-plugin boundary
Where: DOS-546 Open question 6; ADR-0129 `.docs/decisions/0129...md:61`; CLAUDE.md `.CLAUDE.md:69`
Why it matters: Any plugin in the same WordPress install can run PHP, read options, hook requests, inspect DailyOS plugin code/config, and call `localhost` directly. Bearer auth does not isolate one plugin from another. "Localhost-bound" is not a trust boundary inside WordPress.
What to do: Make security review a pre-kickoff requirement. The spike must define whether hostile same-WP-process code is in scope. If it is in scope, MCP from WP cannot get broad substrate write privileges without a much stronger authorization model.

[P2] Unix socket or purpose-built local HTTP may be better than MCP for the WP bridge
Where: ADR-0129 `.docs/decisions/0129...md:72-78`; ADR-0128 `.docs/decisions/0128...md:37-44`
Why it matters: ADR-0128 says MCP tools are question-shaped, not generic data access. That is right for host-model clients. A Gutenberg render path is not a host model. It needs stable, cacheable, projection-shaped data. MCP may be the canonical external protocol, but the WP bridge may need a narrower local projection API behind the same ability registry.
What to do: Require the spike to compare: MCP, Unix domain socket HTTP, loopback HTTP REST, and spawned stdio. Evaluate auth, PHP support, latency, streaming, install complexity, and ability/provenance contract fidelity.

**2. Markdown Filesystem Vs WP DB Three-View Consistency**

[P1] "Substrate of record, WP DB projection, markdown archive" has no conflict-resolution contract
Where: ADR-0129 `.docs/decisions/0129...md:64`, `.docs/decisions/0129...md:141-142`, `.docs/decisions/0129...md:171`; DOS-546 Outcome 3
Why it matters: User edits a Gutenberg block while a background ability updates the same claim. The ticket does not define field-level ownership, version checks, merge semantics, tombstones, or loser preservation. Without this, WP can clobber fresher substrate state or substrate can silently erase user edits.
What to do: Add an explicit concurrency acceptance case: stale WP edit against newer claim version must produce deterministic behavior: reject, merge, or create a correction event. Require claim/version watermarks in block attributes.

[P1] Direct markdown edits are not a projection problem; they are a second write ingress
Where: ADR-0129 `.docs/decisions/0129...md:64`, `.docs/decisions/0129...md:128`; DOS-546 Markdown reconciliation strategy
Why it matters: If Obsidian/vim changes markdown while WP displays the same claim, WP has no truth signal unless the runtime ingests markdown changes as source events. A filesystem watcher alone notices bytes changed; it does not know whether the edit is a correction, a formatting change, a stale overwrite, or a new claim.
What to do: Define whether markdown is writable input or archive-only output. If writable, add a markdown ingestion contract with claim IDs, version stamps, author/source attribution, and conflict behavior. If archive-only, say so and remove "direct markdown edit" from the implied model.

[P1] The three candidate strategies are not equivalent; polling is not acceptable for trust-bearing UX
Where: DOS-546 Approach sketch; CLAUDE.md `.CLAUDE.md:7-14`
Why it matters: Trust bands and claim lifecycle cannot lag unpredictably. Event-pushed updates are the only strategy that fits user-visible intelligence surfaces. Filesystem-watch can supplement runtime-owned markdown writes. Polling is a last-resort repair loop, not the consistency spine.
What to do: Change the open question from "event-pushed, polled, or filesystem-watched" to a required design: runtime event bus is primary; WP projection subscribes or invalidates via event; filesystem watcher is reconciliation/repair; polling is bounded fallback with explicit stale-state UI.

[P2] The spike will miss hidden divergence because it tests one block, one entity, one happy path
Where: DOS-546 Approach sketch; ADR-0105 `.docs/decisions/0105...md:313-325`
Why it matters: Provenance can be large, field-level, and versioned. A one-claim happy path will not expose schema drift, stale block attributes, large provenance payloads, source revocation masking, or partial projection failures.
What to do: Add at least one negative fixture: stale WP DB projection, revoked source/provenance mask, markdown write failure, and ability output schema bump.

**3. Timebox**

[P1] The ticket's 1-2 week timebox conflicts with ADR-0129's 2-4 week validation window
Where: ADR-0129 `.docs/decisions/0129...md:135-144`; DOS-546 Timebox
Why it matters: The ADR says 2-4 weeks for instant-launch and markdown/WP write-path validation. DOS-546 compresses that to 1-2 weeks while adding install surface, plugin/theme bundle, runtime auto-start, write-back, and stack-shape observations. That is not a smaller spike; it is a larger spike with a shorter clock.
What to do: Align the ticket to 2-4 weeks or split it into two spikes: seam/render/write-back first; install/runtime packaging second.

[P1] AC4 is the timebox breaker
Where: DOS-546 Acceptance criterion 4; ADR-0129 `.docs/decisions/0129...md:169-171`
Why it matters: "Rust runtime auto-started as a background service" on macOS means launchd agent, signed binary, possibly notarization, keychain access, lifecycle recovery, log visibility, uninstall, upgrade, port/socket discovery, collision handling, and failure UX. That is not a minor plugin task.
What to do: Downgrade AC4 for this spike to a documented prototype launcher with manual install, or split production-grade auto-start into a foundation ticket.

[P2] "WordPress plugin as MCP client" is closer to 5 days than 2 days, and can become 2 weeks if auth is included
Where: DOS-546 Outcome 1; MCP authorization spec
Why it matters: A basic PHP MCP call may be fast. A WordPress-compatible plugin with Composer dependency packaging, WP activation checks, timeout/error handling, nonce/admin UX, auth token storage, and SSR behavior is not. If OAuth-style protected-resource auth is required, the spec requires bearer tokens per HTTP request, resource binding, PKCE, and secure token storage.
What to do: Add separate estimates for toy client, WP plugin integration, and secure local auth. Do not let a toy client satisfy AC1. Source: [MCP authorization spec](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization).

[P2] Markdown reconciliation is not answerable by implementation alone in 1-2 weeks
Where: DOS-546 Open question 2; ADR-0129 `.docs/decisions/0129...md:141-142`
Why it matters: This is a data-authority decision, not just an engineering experiment. The prototype can measure watcher latency; it cannot validate the semantic conflict model unless that model exists first.
What to do: Require a written consistency contract before coding the prototype.

**4. Stretch Outcome: Rust + Tauri**

[P1] The spike treats Tauri removal as observational, but Tauri currently owns runtime-host responsibilities
Where: ADR-0129 `.docs/decisions/0129...md:101-108`
Why it matters: ADR-0129 says Tauri hosts Rust services, cron, MCP server, privacy gate, abilities execution, signal propagation, and claim writes. A launchd daemon must replace app lifecycle, single-instance behavior, logs, update flow, keychain access, permission prompts, tray/status UI, crash recovery, and user-visible controls.
What to do: Add a mandatory runtime-host inventory: what Tauri provides today, what the daemon must replace, what remains in Tauri, and what becomes impossible or worse.

[P1] A launchd-managed Rust binary changes the distribution problem
Where: DOS-546 Outcome 4/5; ADR-0129 `.docs/decisions/0129...md:148-150`
Why it matters: A background daemon is not "Tauri minus React." It needs install/uninstall, codesign, notarization, update channel, launch agent plist ownership, restart policy, logs, user consent, and secure local endpoint binding. WordPress plugin activation cannot safely install privileged or persistent macOS services by itself.
What to do: Treat daemon packaging as its own architecture decision. The spike should only prove a dev-mode daemon unless it explicitly includes production packaging.

[P2] Tauri may be the wrong primary UI, but still the right runtime supervisor
Where: ADR-0129 `.docs/decisions/0129...md:103-106`
Why it matters: Background services need user-facing status, repair, permission, pairing, and privacy controls. WordPress cannot own macOS-native permissions or runtime health well. Removing Tauri entirely may make onboarding and support worse, even if React magazine becomes secondary.
What to do: Reframe stack-shape output as one of three concrete options: Tauri supervisor + WP primary surface; pure daemon + separate helper UI; Tauri remains primary. Require a recommendation, not just observations.

[P2] The spike should make a call on minimum runtime shape
Where: DOS-546 Outcome 5; ADR-0129 `.docs/decisions/0129...md:135-144`
Why it matters: v1.4.2 depends on the surface decision. "Surface trade-offs" is too weak if the next wave needs to know whether to build for Tauri-hosted runtime, daemon-hosted runtime, or WP-only assumptions.
What to do: Make the spike exit artifact include a yes/no on "Tauri remains required as runtime host for v1.4.2 foundation."

**5. Trust Boundary**

[P1] "Authenticates as the user" is undefined and cannot remain undefined
Where: DOS-546 Outcome 1 / Approach sketch; ADR-0111 `.docs/decisions/0111...md:17-29`; MCP spec auth
Why it matters: ADR-0111 distinguishes `User`, `Agent`, and `System` actors. A WP plugin calling MCP is not automatically the user. It is code running in a CMS runtime. If it gets `User` power, every compromised WP plugin may get user-level substrate access. If it gets `Agent` power, some feedback/write paths may not be allowed.
What to do: Define a new actor or credential class: `SurfaceClient(WordPress)` with scoped abilities, scopes, pairing, expiration, and per-site identity.

[P1] /cso review is mandatory before kickoff, not an optional L0 question
Where: CLAUDE.md `.CLAUDE.md:69`; DOS-546 Open question 6
Why it matters: This touches MCP, filesystem, claim/provenance, privacy, and write-path behavior. The project convention says security review is required for those paths.
What to do: Move trust-boundary review from "open question" to "L0 required reviewer and acceptance gate."

[P1] Same-process WordPress plugins collapse the intended permission model
Where: ADR-0129 `.docs/decisions/0129...md:55-62`; DOS-546 Trust-boundary premise
Why it matters: WordPress role permissions protect WP actions, not substrate actions. A malicious plugin can issue raw HTTP to localhost, read DailyOS plugin options, hook outgoing requests, or alter rendered blocks. The DailyOS runtime cannot tell benign DailyOS plugin code from hostile PHP code if both present the same token.
What to do: Scope the spike threat model explicitly. If hostile WP plugins are in scope, require per-request user presence or capability-specific short-lived challenge tokens. If out of scope, document that arbitrary local WP plugins are trusted code, which weakens the "any local WP install" claim.

[P2] Feedback write-back crosses the prompt-injection boundary
Where: ADR-0102 `.docs/decisions/0102...md:304-323`; ADR-0128 `.docs/decisions/0128...md:65-77`; DOS-546 Outcome 3
Why it matters: Feedback is the only MCP write, but it still mutates claim state. A rendered block can contain attacker-influenced claim text. The save hook must distinguish user-authored correction from machine-generated or block-mutated content.
What to do: Require typed feedback events with user nonce, original claim version, field path, before/after, and explicit user action. Raw block save diffs must not become corrections.

**Other Findings**

[P1] The spike does not prove "Studio or any local WP install"
Where: ADR-0129 `.docs/decisions/0129...md:55`; DOS-546 Install surface
Why it matters: Studio has custom local domains, auto-login, Blueprints, managed paths, WP-CLI affordances, and a desktop wrapper. Arbitrary local WP installs vary across Local, DevKinsta, Docker, Apache/Nginx, PHP versions, filesystem permissions, HTTPS, and loopback networking. A Studio happy path does not validate arbitrary local WordPress.
What to do: Split acceptance: Studio-only pass, arbitrary-local-WP pass, hosted-WP unsupported. Test at least one non-Studio local stack before claiming "any local WP install." Source: [WordPress Studio docs](https://developer.wordpress.com/docs/developer-tools/studio/), [Studio product page](https://developer.wordpress.com/studio/).

[P1] Free-tier validation is not actually covered
Where: ADR-0129 `.docs/decisions/0129...md:80-87`; DOS-546 Acceptance criteria
Why it matters: The free tier claims local WordPress + plugin/theme + local Rust runtime + BYOM backend works end-to-end. The spike tests one developer setup using one real account. It does not validate install repeatability, nontechnical onboarding, BYOM configuration, or no-hosted-infra operation.
What to do: Add a clean-machine/free-tier proof: no Tauri intervention, no hosted DailyOS service, no manual CLI beyond documented install, fresh Studio install, fresh plugin bundle, runtime pairing, first successful render.

[P2] Onboarding is deferred even though install surface is an acceptance criterion
Where: ADR-0129 `.docs/decisions/0129...md:169-171`; DOS-546 AC4
Why it matters: "Plugin install + activation replaces Tauri onboarding" is a UX claim, not just a technical claim. Activation needs runtime detection, pairing, permissions, failures, recovery, and first-data selection. Deferring onboarding means AC4 cannot be honestly passed.
What to do: Add a minimal activation flow artifact: runtime missing, runtime found, auth paired, account selected, block inserted, failure state.

[P1] "Back out" is not a real exit if Hold still commits the pivot
Where: DOS-546 Exit options; ADR-0129 `.docs/decisions/0129...md:135-144`
Why it matters: "Hold" says criteria 1-3 reveal substrate API gaps but pivot still committed. That biases the spike toward confirming ADR-0129 unless the seam catastrophically fails. If install surface or consistency is expensive but technically possible, the current exit taxonomy still pushes forward.
What to do: Add explicit cost-based backout thresholds: time overrun, auth model unacceptable, install requires Tauri anyway, consistency needs a new sync layer, warm render misses target with realistic volume.

[P2] The open questions miss the most load-bearing question: projection ownership
Where: DOS-546 Open questions; ADR-0129 `.docs/decisions/0129...md:60-64`
Why it matters: Custom post types are projections, but WordPress editing makes them feel authoritative. The spike does not ask which fields WP owns, which fields substrate owns, and which edits become feedback versus direct authored content.
What to do: Add an open question: "What is editable in WP, and what is feedback-only?" This must be answered before block architecture.

[P2] Existing ADR-0128 feedback-only write posture conflicts with Gutenberg editability
Where: ADR-0128 `.docs/decisions/0128...md:65-77`; ADR-0129 `.docs/decisions/0129...md:89-99`
Why it matters: Gutenberg's value is editable persistent blocks. ADR-0128 says MCP writes are feedback only, not direct claim creation/editing. The spike must decide whether WP is a richer first-party surface with user writes or just an MCP head with nicer rendering.
What to do: Define WP write classes: block layout edits, user-authored notes, claim feedback, claim correction, generated content publication. Route each through the correct substrate/service path.

**Verdict:** Not L0-ready. The spike needs sharpening first: auth/trust model, consistency contract, install/runtime scope, transport fallback criteria, and a real exit threshold.

---

## Follow-up consult (2026-05-10, same session)

After the research correction and DOS-546 rewrite, Codex was re-invoked via `/codex consult` resume on session `019e1393-e600-7573-ad6f-5517ea91554a` to verify whether prior findings still held under the corrected architecture and surface any new concerns. 386K tokens used.

### Prior findings — confirmed still applying

- [P1] Same-process WP plugin trust collapse still holds. WP-native auth protects REST/MCP/browser callers but does not isolate PHP code already running in the same WP process. Hostile plugin can call `wp_get_ability()->execute()`, hook filters, read options/transients, or call the local Rust transport if it obtains pairing material.
- [P1] /cso review remains mandatory. The risk moved; it did not disappear.
- [P1] Conflict-resolution contract still missing. The WP Abilities API does not solve substrate-vs-WP-vs-markdown authority.
- [P1] Direct markdown edits still a second write ingress. Architecture correction is orthogonal.
- [P1] Polling still not the consistency spine. Trust-bearing UI needs invalidation tied to substrate events.
- [P1] Runtime-host inventory still holds. Corrected WP path removes PHP-as-MCP-client, not the runtime host problem.
- [P1] Launchd packaging still separate production work.
- [P1] Composite ability + SSR latency still apply against the local PHP-to-Rust transport.
- [P2] Feedback write-back still crosses the prompt-injection boundary.
- [P2] Negative fixtures still required.
- [P2] Tauri-as-supervisor becomes *more* likely under the corrected architecture, not less — because the Rust runtime still exposes its own MCP server for non-WP surfaces, so Tauri's MCP-hosting/status/pairing role stays load-bearing even if WP becomes the visual surface.
- [P2] Minimum-runtime call still required as exit artifact.

### New findings from the corrected architecture

**[P1] ADR-0129 is now stale against the corrected architecture.**
ADR-0129 §4 still says "WP plugin opens an MCP session to localhost-bound runtime." The DOS-546 body was rewritten; the ADR was not. Either amend ADR-0129 or add a superseding note before formal L0.

**[P1] MCP Adapter broadens DailyOS invocation to every configured MCP client of the WP site.**
Registered DailyOS abilities can become agent-callable through WordPress. Claude Desktop, Cursor, VS Code, or any configured MCP client can invoke DailyOS abilities as a logged-in WP user. This is a new external-agent surface in addition to the Rust runtime's own MCP server. *What to do:* DailyOS abilities must be excluded from the default WP MCP server unless deliberately exposed. Use a custom DailyOS MCP server with an explicit ability allowlist, dedicated low-capability WP user, and read-mostly defaults.

**[P1] WP capabilities are not the right DailyOS isolation boundary.**
Capabilities like `edit_posts` or `manage_options` are site-wide. They do not express DailyOS substrate scopes, claim subject ownership, source sensitivity, or feedback authority. Other plugins can register abilities under the same user/capability regime. *What to do:* Permission callbacks must check both WP capability AND DailyOS SurfaceClient scope. Add an allowlist so only `dailyos/*` abilities intended for MCP exposure appear in the adapter.

**[P1] Loopback HTTP introduces local endpoint exposure and token-handling problems.**
Any local process can try `127.0.0.1` ports. Any WP plugin may call the same endpoint. If the Rust runtime accepts a long-lived bearer token from WP, compromise of WP state becomes substrate compromise. *What to do:* Bind to localhost only, randomize port, require pairing, use short-lived scoped tokens, reject browser origins, include request signing or mTLS-equivalent local proof if feasible, log every write with SurfaceClient identity.

**[P2] Client-side Abilities API (WP 7.0, 2026-05-20) creates a second in-browser invocation path.**
Server abilities become available in admin JS by default. Blocks, browser extensions, injected admin JS, or future browser agents can execute abilities through REST. SSR is no longer the only render path; mutation idempotency matters. *What to do:* Spike block architecture must test both SSR and client-side `executeAbility()` behavior, including duplicate execution, nonce expiry, permission denial, and stale claim version handling.

**[P2] Ability metadata becomes model-facing AND browser-facing API surface.**
Labels, descriptions, schemas, annotations, and categories are exposed to agents and admin JS. Bad descriptions can leak sensitive concepts or steer agents into wrong tools. *What to do:* Add an ability-surface inventory artifact: name, description, category, annotations, WP permission, DailyOS scope, MCP exposure yes/no.

**[P2] Unix domain socket fallback is transport-hardening, not authorization.**
Socket permissions reduce random local-process access, but the PHP worker user still needs access. Any plugin running as that user inherits it. Studio/Docker/Local variants differ on socket path and mount behavior.

**[P2] Spawned stdio fallback is operationally dangerous beyond dev mode.**
Per-request spawning explodes latency and lifecycle complexity. Mark as last-resort diagnostic transport only unless measured cold-start and concurrency behavior are acceptable.

### Updated verdict

Still not L0-ready until three things are explicit:

1. **Security model:** SurfaceClient scopes, hostile-plugin stance, MCP Adapter exposure policy (custom server + allowlist), and Rust transport token rules (short-lived, scoped, bound, signed).
2. **Consistency model:** substrate/WP/markdown authority, conflict resolution, markdown writable-vs-archive decision.
3. **Runtime/transport contract:** ADR-0129 amended away from "WP plugin as MCP client," plus measured loopback/UDS/stdio behavior and Tauri-vs-daemon recommendation.

The corrected architecture removed the PHP MCP SDK blocker. It did not remove the hard parts — it moved them to WordPress ability exposure, local transport auth, and projection consistency.
