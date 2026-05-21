# L0 Packet — v1.4.4 Wave Plan (WordPress Surface Migration)

**Current revision: V1.1 (cycle-1 reviewer folds, 2026-05-20).**

## 1. Header

Date: 2026-05-20
Project: [v1.4.4 — WordPress Surface Migration](https://linear.app/a8c/project/v144-wordpress-surface-migration-877aaa780177) (id `f8b805d9-f3d4-41b4-a446-51bbb7e05f2e`)
Scope: full v1.4.4 wave program — W0 (surface audit) → W6 (parity proof + Tauri shell deprecation). This is the **master wave-level L0**. Per-wave sub-L0 packets (W1 substrate gaps, W2 entity surfaces, W3 briefing surfaces, W4 action surfaces, W5 system/history) author after W0 audit lands and inherit invariants from this packet.

Primary anchors:
- **ADR-0129** — Composable surfaces: WordPress Studio as primary surface. The reorientation authority.
- **ADR-0130** — Surface-independent Composition contract. §2 `Composition` model, §3 `BlockType` taxonomy, §4 renderer-not-author boundary, §6 SurfaceClient consumption.
- **ADR-0102, 0105, 0108, 0111, 0125, 0128** — abilities runtime, provenance, rendering/privacy, surface-independent invocation, sensitivity, headless heads. Consumed unchanged.
- **`.docs/plans/wp-foundation-roadmap-reorientation.md`** lines 80–106 — canonical v1.4.4 wave-structure source.
- **v1.4.3 close** — strict prerequisite per reorientation doc §C2 ("v1.4.5+ versions are strict-gated on v1.4.4 closing"); v1.4.4 itself gates on v1.4.3 W6 clean-machine validation closing.

Surfaces in scope: ~18 active Tauri user-facing surfaces, to be enumerated by DOS-677 (W0 surface audit) from `src/router.tsx` + `.docs/design/INVENTORY.md` + nav reachability. Each migrates to a WordPress Gutenberg composite block consuming real substrate (no façades over old data model).

Pulled-forward work already shipped on this branch (sha range `52d25db5..26062496` inclusive — DOS-721 lives at `52d25db5`, DOS-729 at `771a3d5d`; `26062496` is DOS-336 theme.json wiring, not chrome lane proper; branch `docs/k-out-session4-workflow-learnings`):
- **DOS-721** — canonical refresh-button pre-lift upgrade
- **DOS-729** — chrome lane Tier 1 (tokens + aliases)
- **DOS-730** — chrome lane Tier 2 + 3 (chrome CSS modules + fonts)
- **DOS-731** — chrome lane Tier 4 (chrome.js + sync tooling)
- **DOS-732** — chrome lane Tier 5 (functions.php + header/footer)
- **DOS-724** — IntersectionObserver scroll-spy for FloatingNavIsland
- **DOS-722** — Pill primitive duality + collision gate
- **W1-RECEIPT carve-out** — shared Receipt DTO substrate (DOS-339 partial; full wiring deferred to W4)

These shipped via the v1.4.4 W3 chrome lane pulled-forward parallel to v1.4.3 W4+. See `L0-packet-W3-chrome-lane-pulled-forward.md` for that lane's L0 close. The wave-level L0 here **acknowledges** chrome shipped and treats it as substrate that W3 briefing surfaces and W2 entity surfaces consume, rather than re-litigating.

**Intelligence Loop integration check — applies at sub-L0 granularity.** This master packet does not introduce claim/table/surface/signal/feedback changes itself; it ranges over the whole wave. Per-wave sub-L0 packets (especially W1 substrate gaps) MUST answer the 5 Intelligence Loop questions from CLAUDE.md before their implementation starts. W2/W3/W4 sub-L0 packets ALSO must answer them where a new claim field, projection, or feedback path is introduced by a composite block.

## 2. Changelog

- **V1.1 (2026-05-20):** cycle-1 reviewer folds across 5 verdicts. Architecture (8 findings): F1 HIGH outer/inner ADR-0130 mapping (Reading A locked); F2 HIGH cursor envelope contract codified; F3 MEDIUM W0 5th list (dev/admin surfaces); F4 MEDIUM mid-wave gap escalation in AC #W5; F5 MEDIUM refresh-model invariant + uniform Intelligence Loop §3 answer; F6 MEDIUM forward-coupling hot spots (schema_version semver-additive + review-queue slot coordination); F7 LOW Gutenberg-default rule on runtime-injection; F8 LOW design-system canonicity rule extension. Design-lens (2 BLOCKING): state-coverage matrix per wave (W3/W4/W5); cross-wave dogfooding flow (briefing → meeting → entity → action). WP-skill (7 findings): F1 HIGH InnerBlocks + `templateLock: false` + global inner-block inventory + `providesContext`/`usesContext` + `do_blocks($content)`; F2 HIGH `executeAbility()` (WP 7.0) + `useAbilityCursor()` hook + list envelope shape `{ items, next_cursor, total_hint }`; F3 MEDIUM per-project tint via CSS custom property on outer wrapper; F4 MEDIUM flag-flip preserves externalBin + build-mcp.sh + runtime-still-running smoke; F5 LOW dailyos block category already registered (don't re-register); F6 LOW apiVersion 3 mandatory; F7 LOW synced patterns for composition templates, template parts for chrome. Codex challenge (4 findings): F1 HIGH W6 parity checklist tightened with state-matrix/trust-fidelity/feedback-round-trip; F2 HIGH outer-envelope-carries-first-page-slice clause on §13 #1; F3 HIGH C4 supersedes path-α when substrate gap blocks named AC; F4 MEDIUM W0 enumerates all `createFileRoute` + cmd+k + feature flags + pre-flip dry-run. Codex consult (3 findings): F1 sha range fix `52d25db5..26062496 inclusive`; F2 `26062496` is DOS-336, not chrome lane; F3 ADR-0132 added to K-in + References.
- **V1.0 (2026-05-20):** initial L0 draft. Converts reorientation doc §"v1.4.4 — WordPress Surface Migration" + Linear project description into a reviewable wave-level packet. Treats W3 chrome lane (pulled forward) and DOS-339 W1-RECEIPT carve-out as already-shipped substrate.

## 3. K-in record (substrate-grep audit, 2026-05-20)

Per CLAUDE.md "Knowledge store discovery" + engineering-ladder.md L0 K-in obligation.

### `docs/solutions/` — 20 .md files at scan time

Greps run against the full inventory for keywords: `chrome`, `block`, `gutenberg`, `theme.json`, `wp`, `wordpress`, `surface`, `substrate`, `migration`, `composition`, `entity`, `briefing`, `claim-receipt`, `feedback-router`, `parity`, `wave-plan`, `L0-amendment`, `synced-from`.

**Relevant cross-references the wave packet consumes:**

- `docs/solutions/workflow-issues/parallel-wave-synced-from-conflicts-2026-05-19.md` — chrome lane experience with `.synced-from` conflicts when multiple wave agents touch synced canonical files. Wave-level constraint: W3 briefing surfaces in v1.4.4 W3 will likely consume the chrome lane's `.synced-from` discipline; sub-L0 packets must reference this entry when authoring sync-touching work.
- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — substrate already exists for surface nonces / feedback router; v1.4.4 sub-L0 packets MUST grep substrate-type names, not proposed-name aliases.
- `docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md` — precedent for wave-plan amendments. This packet IS a wave-plan amendment relative to the placeholder `v1.4.4-waves.md`; no precedent conflict.
- `docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md` — reviewer L0 cycle policy. Wave packet must be sized so 5+ net-new findings per cycle = scope reset, not fold-and-continue.
- `docs/solutions/workflow-issues/premise-check-production-vs-dev-friction-before-scoping-waves-2026-05-20.md` — premise-check policy for ACs.
- `docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md` — codex dispatch hygiene; relevant when sub-L0 packets fan out parallel work.

### `.docs/decisions/` — 9 relevant ADRs

ADR-0083, 0102, 0105, 0108, 0111, 0125, 0128, 0129, 0130, 0132. Consumed unchanged. **No ADR overrides.** ADR-0129 §9's wave reorientation has been superseded by the 2026-05-17 renumber (locked in reorientation doc; v1.4.4 today is the WP Surface Migration, not the original v1.4.4 Claim Experience). ADR-0129's substrate decisions remain authoritative. ADR-0132 (Pill primitive dual existence) is the durable contract for the chrome `.Pill_*` vs block `.dailyos-pill*` split referenced in §10 invariants and DOS-722 pulled-forward work [consult F3].

**Pulled-forward chrome lane L0 packet** at `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` is itself K-in for this wave packet — it documented the chrome runtime-injection scope decision (4 shell modules: FolioBar / FloatingNavIsland / AtmosphereLayer / MagazinePageLayout) and the substrate-consumed inventory the W3 briefing surfaces will compose against.

**Verdict: K-in complete. No documented substrate reinvented.** Every named v1.4.4 producer either (a) already exists in v1.4.0/v1.4.1 substrate (abilities runtime, claim/provenance, signals), (b) shipped in v1.4.3 (primitives, magazine theme, starter kit, feedback infra), or (c) is the explicit substrate-gap target of W1 (envelope, touchpoints/open-loops, receipt DTO, prep status, daily briefing ability).

## 4. Scope summary

| Wave | Subject | Surfaces in scope | Substrate consumed | LOC ballpark |
|---|---|---|---|---|
| **W0** | Surface audit (gate) | All ~18 candidates (active / carry-forward / inactive / open-questions) | `src/router.tsx`, `.docs/design/INVENTORY.md`, nav reachability | ~1 audit doc + Linear sign-off |
| **W1** | Substrate gaps | n/a (substrate only) | v1.4.0/v1.4.1 abilities runtime; v1.4.3 primitives; existing entity services; existing claim/provenance | ~2,500–4,000 (10 substrate items) |
| **W2** | Entity surfaces | Account / Project / Person / Meeting detail blocks + entity list shells + metadata proposals | W1 envelope + open-loops + touchpoints + DOS-477 trust-boundary + v1.4.3 primitives + chrome lane shell | ~3,000–5,000 (5 composite blocks) |
| **W3** | Briefing surfaces | Daily Briefing + Meeting Briefing blocks (FolioBar + FloatingNavIsland already shipped via chrome lane) | W1 `get_daily_briefing` + meeting prep DTO + entity envelope + chrome FolioBar | ~1,500–2,500 (2 composite blocks) |
| **W4** | Action surfaces | Actions/Work, Activity Log, Action Detail, Lint Mode, review queue, contradiction UX, per-claim_type render rules | W1 Receipt DTO + claim review queue + DOS-8 semantic feedback + existing actions service | ~3,000–5,000 (4 composite blocks + UX integrations) |
| **W5** | System / history surfaces | History block; Email / Settings surfaces (subject to W0 audit); adversarial fixtures | Existing audit + claim history substrate; v1.4.3 W6 audit attribution | ~1,500–3,000 (scope depends on W0) |
| **W6** | Parity proof + Tauri shell deprecation | Tauri ↔ WP parity gate; DOS-458 release gate concept; Tauri shell deprecation flag | All prior waves' surfaces | ~500–1,000 (proof artifacts + deprecation flag) |

Wave total: ~12k–20k LOC across ~18 surfaces. Translation reality (per reorientation doc): most surfaces already exist as HTML + Tauri React, so this is mostly translation work + W1 substrate gaps + composition wiring, not net-new design.

## 5. Detailed sections

### §5.0 W0 — Surface audit (gate)

**Issue:** [DOS-677](https://linear.app/a8c/issue/DOS-677) — v1.4.4 W0: Surface audit — active vs carry-forward vs inactive. Priority: High. **Gates W1+ start.**

**Outputs:** five lists committed to `.docs/plans/v1.4.4-surface-audit.md`:
- **Active** — Tauri surfaces routed + reachable from nav + still relevant. Migrate. Each carries route path, component path, primary substrate consumed, wave assignment (W2/W3/W4/W5), substrate gaps.
- **Carry-forward** — design-system surfaces not yet shipped in Tauri. Migrate if scope allows; queue otherwise.
- **Inactive/legacy** — surfaces no longer relevant. Mark deprecated; remove from Tauri shell at W6.
- **Dev/admin surfaces** [arch F3] — surfaces that stay visible post-W6 flag-flip because they are runtime-host concerns, not end-user surfaces: settings-runtime, keychain debug, ability runtime status, MCP server status, dev tooling. These are NOT migration targets and NOT flag-flip targets. The list is produced by the W0 audit; it is not pre-enumerated at the wave-L0 layer.
- **Open questions** — surfaces where James's call is required before W1 starts.

**Audit-completeness procedure [codex challenge F4]:** W0 enumerates surfaces from four authoritative sources, not just nav reachability: (a) all `createFileRoute(...)` entries in `src/router.tsx`, (b) command-palette entries (`cmd+k` registry), (c) feature-flagged surfaces (build flags, runtime flags), (d) nav reachability + `.docs/design/INVENTORY.md` cross-reference. Surfaces reachable only by deep-link or palette but not from nav still land on one of the 5 lists. W6 sub-L0 inherits a pre-flip dry-run procedure: scan Tauri shell for every routed component, cross-check against W0 lists, fail W6 if any unaccounted-for route exists.

**Substrate gap feedback loop:** the active list's "substrate gaps" column feeds W1 scope. W1 cannot be sized until W0 lands.

**Key open questions:** which Tauri Email surfaces stay? Settings: full migration or thin admin? Lint Mode: W4 (action surface) or W5 (system)? Report-shaped surfaces (BookOfBusiness, EbrQbr, SWOT, AccountHealth-as-report) explicitly deferred to v1.4.8.

**Acceptance shape:** James signs off on all four lists; substrate-gap column for active list feeds W1; sign-off captured as Linear comment on DOS-677.

### §5.1 W1 — Substrate gaps

**Mission:** ship the substrate the W2–W5 surfaces consume. No surface ships against old data model; substrate without consumers doesn't get designed right (per execution constraint C4).

**Named substrate items (from reorientation doc + Linear project §W1):**

| Item | Linear | Consumed by |
|---|---|---|
| `get_entity_intelligence` envelope (typed, schema-versioned, Read ability) | [DOS-459](https://linear.app/a8c/issue/DOS-459) | W2 Account / Project / Person Detail |
| Canonical entity touchpoints + open-loops contract | [DOS-460](https://linear.app/a8c/issue/DOS-460) | W2 entity detail + W3 briefing |
| Entity fixture harness + no-bypass checks | [DOS-461](https://linear.app/a8c/issue/DOS-461) | W2 acceptance verification |
| Entity-detail trust-boundary hardening (CSO work) | [DOS-477](https://linear.app/a8c/issue/DOS-477) | W2 (gates DOS-462/483/484 close) |
| `get_daily_briefing` Read/User-only ability | (new ticket at W1 L0 kickoff) | W3 Daily Briefing block |
| Meeting prep / readiness DTO (single service-owned status) | [DOS-335](https://linear.app/a8c/issue/DOS-335) | FolioBar + event rows + daily briefing + meeting detail |
| Shared Receipt DTO + semantic feedback action surface | [DOS-339](https://linear.app/a8c/issue/DOS-339), DOS-8 | W4 Actions/Work + claim-bearing surfaces |
| `services::claim_receipt::*` + claim review queue extensions | (carve-out partial; full wiring W1) | W4 review queue + DOS-336 candidate extension |
| Receipt privacy / redaction rules | DOS-341 | All claim-rendering blocks |
| Receipt vs operational audit boundary | DOS-340 | W5 audit blocks vs W4 receipt rendering |

**W0 audit may surface additional substrate gaps** — they land in W1 alongside the named items, not deferred.

**Substrate dependencies:** v1.4.0/v1.4.1 abilities runtime (unchanged); v1.4.2 W4 composition substrate (`Composition`, `ProjectedComposition`); v1.4.3 W4 feedback wire-through (DOS-683 merged at `d781f2b4`); v1.4.3 primitives.

**Key acceptance shape:** every named substrate item lands with (a) tests covering the empty-state matrix called out in DOS-459 (not connected, not processed yet, filtered by subject scope, etc.), (b) Intelligence Loop integration check answered in the sub-L0 packet, (c) no direct DB writes from command handlers, (d) producer/projection/renderer fixture passes the v1.4.3 W1 starter kit harness.

**Key open questions for W1 sub-L0:**
- DOS-459 envelope `schema_version` lifecycle policy — semver-style? major-version-only ABI?
- DOS-460 `candidate_set` / `inclusion_reason` / `exclusion_reason` — claim-shaped or service-derived?
- DOS-339 Receipt DTO mapping from rendered provenance — adapter pattern or direct projection?

### §5.2 W2 — Entity surfaces

**Mission:** Account / Project / Person / Meeting detail blocks render via the W1 envelope; entity list shells + metadata proposals interaction.

**Surfaces in scope:**
- Account Detail block → consumes envelope from DOS-459; absorbs [DOS-462](https://linear.app/a8c/issue/DOS-462) (route `/accounts/$accountId`, component `src/pages/AccountDetailPage.tsx`). Primary proof surface for the envelope.
- Project Detail block → absorbs [DOS-483](https://linear.app/a8c/issue/DOS-483) (route `/projects/$projectId`, component `src/pages/ProjectDetailEditorial.tsx`).
- Person Detail block → absorbs [DOS-484](https://linear.app/a8c/issue/DOS-484).
- Meeting Detail block.
- Entity list shells (Accounts / People / Projects index pages).
- Metadata proposals interaction → absorbs [DOS-328](https://linear.app/a8c/issue/DOS-328) (accept/dismiss/edit feeding claim/proposal loop).

**Substrate dependencies:** W1 envelope (DOS-459); W1 open-loops/touchpoints (DOS-460); W1 trust-boundary hardening (DOS-477) — gates close; W1 entity fixture harness (DOS-461) — gates verification; v1.4.3 primitives (TrustBandBadge, EntityChip, FreshnessIndicator, ProvenanceTag, IntelligenceQualityBadge, HealthBadge); chrome lane FolioBar (already shipped) + chrome shell.

**Key acceptance shape:** per DOS-462, every claim-backed intelligence section (Health view: SentimentHero / TriageSection / DivergenceSection / OutlookPanel / SupportingTension / AboutIntelligence; Context view: StakeholderGrid / StrategicLandscape / ValueCommitments / QuoteWall / CommercialShape / etc.; Work view; Record/Timeline) routes through the envelope, not legacy `detail.intelligence` / local freshness fragments / page-local raw-state composers. Required visible QA states: full / empty / stale / `needs_verification` / corrected/superseded / proposal cue+drawer / proposal accepted/dismissed/edited / touchpoints present / cite chip evidence drawer.

**Key open questions for W2 sub-L0:**
- Many-blocks-not-few granularity inside Account Detail composite (per anchored decision #1, reorientation doc): one `dailyos/account-detail` outer block with N inner blocks for each chapter? Or N peer blocks under a template?
- Entity list shell pagination/filtering inside Gutenberg — server-side via `Composition` `sections` or client-side via block-level state?
- DOS-725 `dailyos_project` tint resolution (from chrome lane L0 §8) — gates Project Detail block CPT registration.

### §5.3 W3 — Briefing surfaces

**Mission:** Daily Briefing + Meeting Briefing composite blocks. **FolioBar + FloatingNavIsland already shipped** via the chrome lane (DOS-729/730/731/732 + DOS-724 + DOS-722) — W3 briefing surfaces compose against the existing chrome runtime, not re-author it.

**Surfaces in scope:**
- Daily Briefing block — renders `get_daily_briefing` ability output (W1) per ADR-0130 `Composition` model.
- Meeting Briefing block — renders meeting prep DTO (W1 DOS-335) + entity envelope (W1 DOS-459).

**Substrate dependencies:** W1 `get_daily_briefing` ability; W1 meeting prep DTO (DOS-335); W1 entity envelope (DOS-459) for entity binding; chrome lane FolioBar (already shipped) for top-bar status indicators; chrome lane FloatingNavIsland for in-briefing chapter navigation; v1.4.3 primitives.

**Key acceptance shape:** Daily Briefing renders with realistic claim volume; trust band, provenance, empty, error, stale, ambiguity, correction states all reachable (per dissolved v1.4.4 W3 Wave 3 milestone "Design cutover and trust-state QA"); meeting prep status converges across FolioBar / briefing chapter / meeting detail (per DOS-335 ACs).

**Key open questions for W3 sub-L0:**
- Briefing surfaces consume FolioBar from chrome lane runtime-injection OR get a block-level wrapper? Chrome lane L0 §10 invariant restricts runtime-injection to 4 shell modules — briefing must consume, not re-inject.
- FloatingNavIsland scroll-spy (DOS-724) — does it work cleanly with Gutenberg block-tree DOM? Validate during W3 sub-L0.

### §5.4 W4 — Action surfaces

**Mission:** Actions/Work, Activity Log, Action Detail, Lint Mode, review queue, contradiction UX, per-claim_type render rules. All claim-bearing surfaces converge on the shared Receipt DTO (DOS-339).

**Surfaces in scope:**
- Actions/Work block → [DOS-514](https://linear.app/a8c/issue/DOS-514). Primary consumer of Receipt DTO.
- Activity Log block → [DOS-444](https://linear.app/a8c/issue/DOS-444).
- Action Detail block.
- Lint Mode block → [DOS-445](https://linear.app/a8c/issue/DOS-445).
- Review queue → [DOS-443](https://linear.app/a8c/issue/DOS-443).
- Contradiction UX → [DOS-318](https://linear.app/a8c/issue/DOS-318).
- Per-claim_type render rules → [DOS-447](https://linear.app/a8c/issue/DOS-447).

**Substrate dependencies:** W1 Receipt DTO (DOS-339); W1 `services::claim_receipt` + claim review queue extensions; W1 receipt privacy/redaction rules (DOS-341); W1 receipt-vs-operational-audit boundary (DOS-340); DOS-8 semantic feedback action surface; v1.4.3 W4 feedback router (merged `d781f2b4`); chrome shell.

**Key acceptance shape:** receipts surface across Actions/Work + at least one upstream proof path (per DOS-339 ACs); receipt rendering supports `likely_current` / `use_with_caution` / `needs_verification` bands; semantic actions feed DOS-8 / DOS-318 service-owned reconciliation; no component writes claim/feedback/lifecycle state directly (all mutations through services per CLAUDE.md critical rule); per-claim_type render rules covered for all `BlockType` taxonomy entries from ADR-0130 §3.

**Key open questions for W4 sub-L0:**
- Inline-edit-affordance contract (per anchored decision #2): wire shape codification — "edit captured as `FeedbackAction::ClaimCorrection` (or analogous), routed through SurfaceClient → substrate, returned projection re-renders." Open question from reorientation doc §"Open architectural questions" #2.
- DOS-336 candidate extension hook — does Salience (v1.4.6) hook into W4 review queue, or does W4 carry a no-op slot that v1.4.6 fills?

### §5.5 W5 — System / history surfaces

**Mission:** History block + Email surfaces + Settings surfaces. Final scope decided post-W0 audit. Adversarial fixtures land here.

**Surfaces in scope (subject to W0 audit):**
- History block.
- Email surfaces — W0 audit decides active vs deprecate.
- Settings surfaces — W0 audit decides scope; pieces may defer.
- Adversarial fixtures → [DOS-446](https://linear.app/a8c/issue/DOS-446).

**Substrate dependencies:** existing audit substrate; v1.4.3 W6 audit attribution (DOS-576); claim history primitives; existing settings persistence.

**Key acceptance shape:** History block renders user-relevant claim history (not raw audit log per receipt-vs-operational-audit boundary DOS-340); Email/Settings surfaces decided per W0; adversarial fixtures (DOS-446) exercise the negative-case matrix.

**Key open questions for W5 sub-L0:**
- Settings: are theme/runtime/keychain/feedback settings authored as Site Editor preferences + WP options OR as a custom DailyOS settings block? Migration path matters.
- Self-Healing surfaces — explicitly deferred to v1.4.9 unless W0 audit finds existing Tauri equivalent already in production.

### §5.x Visible-QA-state matrix — wave obligations [design-lens Finding 1]

Per-wave state lists each sub-L0 packet MUST enumerate before its implementation starts. W2 anchored by DOS-462; W3/W4/W5 lists below are starting checklists, not exhaustive.

| Wave | Required visible states (sub-L0 obligation) |
|---|---|
| **W2 (entity)** | full / empty / stale / `needs_verification` / corrected/superseded / proposal cue + drawer / proposal accepted/dismissed/edited / touchpoints present / touchpoints empty / cite-chip evidence drawer / subject-scope filtered (per DOS-462) |
| **W3 (briefing)** | full / empty ("no meetings today") / briefing-generating / all-meetings-complete / prep-ready vs prep-not-started (per DOS-335) / ambiguity (conflicting claim signals for same account) / stale / `needs_verification` / `likely_current` / `use_with_caution` trust-band variants |
| **W4 (action)** | empty queue (no open actions) / open-actions populated / claim-pending-review (stuck in review queue) / contradiction-flagged (distinct from corrected/superseded) / contradiction-resolved/superseded / inline-edit in-flight (feedback claim emitted, projection not yet returned) / inline-edit success (projection re-rendered) / inline-edit error / per-claim_type render rule branches (ADR-0130 §3 BlockType taxonomy) |
| **W5 (system/history)** | empty (no inbox activity this week) / populated-but-unintelligent (inbox activity, no claim outcomes) / populated-with-outcomes / receipt-vs-operational-audit boundary respected (per DOS-340) / adversarial fixture coverage (per DOS-446) |

W6 parity proof checks the matrix is reachable for every active surface; missing states blocks W6 close.

### §5.x Primary dogfooding flow across waves [design-lens Finding 2]

Customer-zero journey naming the wave boundaries the L0 packet's wave sequencing implies. Mixed-stack navigation is acceptable through W6 per the flag-flip-at-W6 decision (§13 #5): Tauri surfaces stay reachable until W6 hides them; cross-stack navigation works during the build-out period.

| Step | Surface | Owning wave |
|---|---|---|
| (a) Open Daily Briefing (start of day) | Daily Briefing block | **W3** |
| (b) See a meeting in the briefing | Daily Briefing block → meeting summary chapter | **W3** |
| (c) Click through to Meeting Detail to prep | Meeting Briefing block | **W3** |
| (d) From meeting context, open relevant Account Detail | Account Detail block | **W2** |
| (e) From account detail, see open actions | Account Detail block → actions chapter | **W2** consumes / **W4** action surfaces |
| (f) Act on an action (semantic feedback, claim correction) | Actions/Work block + Action Detail block | **W4** |
| (g) Confirm action outcome in history | History block | **W5** |

**Mixed-surface period semantics (W2/W3/W4 build-out window):** during the build-out, Daily Briefing (`/`) may still be Tauri while an entity-detail block is already WP, or vice versa. Cross-stack navigation MUST work — links from a WP block to a Tauri route resolve through the existing Tauri router; deep-links into WP blocks resolve through WordPress permalinks. FloatingNavIsland scroll-spy (DOS-724) handles per-page chapter nav on whichever stack rendered the page. The flag-flip at W6 (§13 #5) is the ONLY step that breaks cross-stack reachability — until then, both surfaces are live. Sub-L0 authors design navigation affordances against this mixed-stack reality.

MePage report entry-point links (`/me/reports/...`) remain as Tauri routes through v1.4.4 and v1.4.4 W6 flag-flip context where the relevant report surfaces ship in v1.4.8 [design-lens Finding 3]. W5 WP block does not suppress or stub these links during v1.4.4.

### §5.6 W6 — Parity proof + Tauri shell deprecation

**Mission:** end-to-end acceptance that every active Tauri surface has a working WP block equivalent reading real substrate. Tauri shell marked deprecated; primary flips to WordPress.

**Scope:**
- Parity proof artifact for every active surface from W0 list.
- DOS-458 release gate concept absorbed here as the parity gate.
- Tauri shell deprecation flag (staged removal or flag-flip — open question, see §13).

**Key acceptance shape:** for each active surface, side-by-side proof bundle (Tauri screenshot + WP block render screenshot + substrate-read evidence + user-can-do-the-thing acceptance, per reorientation doc "Acceptance is 'user can do the thing the Tauri surface enabled,' not 'WP looks identical to Tauri.'"). Tauri shell deprecation strategy decided + executed.

**Parity-proof checklist (yes/no per surface, unanimous-yes to pass) [codex challenge F1]:**
- (i) Full visible-QA-state matrix for the owning wave (§5.x table above) reachable in the WP equivalent — not just happy path.
- (ii) Trust-band + provenance + freshness signaling matches the Tauri reference within tolerance (no-drift per memory `project_memory_plus_judgment_equals_trust`); band color, freshness chip, provenance drawer evidence equivalent.
- (iii) At least one feedback round-trip per claim-bearing surface returns a projection diff in the same session (correction submitted → reaches `services::claims::record_claim_feedback` → projection re-renders the corrected state).
- (iv) Runtime-still-running smoke after flag-flip [WP-skill F4]: MCP server responds; keychain reachable; signal propagation fires. `src-tauri/tauri.conf.json` `externalBin` packaging preserved; `build-mcp.sh` stub-create-before-cargo-build sequence intact.

**Flag-flip scope [arch F3 + WP-skill F4]:** the W6 build flag hides W0-classified *end-user* Tauri React UI only (`src/pages/*` magazine surfaces, entity detail pages, briefing pages, action surfaces). Dev/admin surfaces from the W0 5th list remain visible — settings-runtime, keychain debug, ability runtime status, MCP server status, dev tooling. Tauri tray / dock / window remains for runtime-host duties; Tauri does NOT go fully background-headless at W6.

**Key open questions for W6 sub-L0:**
- Parity-proof artifact format — HTML report? Linear-attached markdown? Per-surface video walk-through?
- Tauri shell deprecation — staged removal (UI surfaces removed per-wave during v1.4.4) or flag-flip (all Tauri React UI hidden behind a build flag at W6 close)?

## 6. Substrate consumed (K-in companion)

Inventory of substrate the wave reads, by source. None of this is reinvented; every named item is a consumer relationship.

### From v1.4.0 / v1.4.1 (abilities runtime spine, unchanged)
- Abilities runtime + invocation per ADR-0102 / ADR-0111.
- Claim model + lifecycle + sensitivity + trust bands per ADR-0125.
- Provenance envelope per ADR-0105.
- Signal propagation + invalidation per ADR-0080.

### From v1.4.2 (composition substrate, surface bridge)
- `Composition` / `ProjectedComposition` / `ProjectedBlock` per ADR-0130.
- W4-F local-to-local read path: `surface_runtime/project_composition` route, signed-loopback transport, surface session lifecycle.
- `dailyos/account-overview` block as the v1.4.2 reference implementation v1.4.4 generalizes.

### From v1.4.3 (WordPress foundation — required prerequisite)
- 10 Wave-1 primitive blocks: `Pill`, `HealthBadge`, `StatusDot`, `Avatar`, `TrustBandBadge`, `IntelligenceQualityBadge`, `FreshnessIndicator`, `ProvenanceTag`, `EntityChip`, `TypeBadge`.
- C1 starter kit: `block.json` scaffold + `render.php` template + producer template + projection rule template + shared producer→projection→renderer integration test fixture.
- DailyOS magazine theme (DOS-574) — token-to-`theme.json` generator + templates + parts.
- Feedback write infrastructure: DOS-571 user-presence nonce + DOS-573 click-bound feedback router (merged at `d781f2b4`); REST route `/dailyos/v1/nonce/verify`.
- Audit + clean-machine validation: DOS-575 negative fixtures, DOS-576 SurfaceClient audit attribution, DOS-577 clean-machine Studio validation.
- v1.4.3 stabilization fixes: DOS-671..675 lifecycle hardening; Studio sandbox compatibility (C3).
- `dailyos` block category registration (`block_categories_all` filter in `wp/dailyos/includes/class-dailyos-plugin.php`) — consumed unchanged by all new W2–W5 blocks; do NOT re-register [WP-skill F5].

### From v1.4.4 W3 chrome lane (already shipped on this branch)
- `wp/dailyos/theme/functions.php` enqueue chain + chrome_config().
- `wp/dailyos/theme/assets/chrome/styles/{FolioBar,FloatingNavIsland,AtmosphereLayer,MagazinePageLayout,Pill}.module.css` (5 modules, lifted from canonical).
- `wp/dailyos/theme/assets/chrome/chrome.js` (synced + patched, idempotent via Patch 9a).
- `wp/dailyos/theme/assets/chrome/styles/{design-tokens,token-aliases}.css` token bridge.
- `wp/dailyos/theme/tools/{sync-chrome.sh,patch-chrome-js.py}` sync tooling.
- DOS-724 IntersectionObserver scroll-spy for FloatingNavIsland local pill.
- DOS-722 Pill primitive duality (chrome `.Pill_*` vs block `.dailyos-pill*`) + CI collision gate.

### Existing services consumed unchanged
- `services::claims::record_claim_feedback` (and W4-E nonce sibling) for feedback writes.
- Entity services (account / project / person) — operational shell data only; intelligence sections route through envelope.
- Calendar / meeting / action services — readers only for W1 envelope assembly.
- Keychain + surface session services — runtime-host concern (Tauri side-process per C3).

## 7. Acceptance criteria — wave-level

Wave-level ACs span the full v1.4.4 program. Per-surface ACs defer to sub-L0 packets (W1/W2/W3/W4/W5/W6).

**AC #W1 — Tauri ↔ WP parity gate (W6).** Every surface on the W0 active list has a working WP block equivalent that (a) renders real substrate (no façade over old data model), (b) supports the visible-QA-state matrix called out per W2 in DOS-462 (full / empty / stale / `needs_verification` / corrected/superseded / proposal cue / etc.), (c) user can do the thing the Tauri surface enabled.

**AC #W2 — No-shells rule (per C4).** No surface ships against the old data model. If a W2/W3/W4/W5 sub-L0 packet names a substrate item not in v1.4.0–v1.4.3 + v1.4.4 W1 inventory, that item lands in W1 same-wave, not deferred. CI gate: per-block integration test against the v1.4.3 starter kit harness must pass with realistic substrate fixture, not stub.

**AC #W3 — Inline-edit-affordance contract (per anchored decision #2).** Every claim-bearing block supports inline edit captured as feedback claim through SurfaceClient → substrate → returned projection re-renders. No direct DB writes from WP-side JS. Wire shape codified by W4 sub-L0.

**AC #W4 — Many-blocks-not-few rule (per anchored decision #1).** Each Tauri component or pattern gets a Gutenberg block equivalent. Composite surfaces (Daily Briefing, Account Detail) compose many small blocks rather than one monolithic block. "Default page composition template" ships as a **synced pattern** (`patterns/*.php`) per composite surface (Account Detail default, Project Detail default, Daily Briefing default, Meeting Briefing default) [WP-skill F7]. Theme **template parts** (`parts/*.html`) are reserved for chrome-shaped regions (header/footer/sidebars) consistent with current `theme.json` `templateParts` declaration. W2/W3 sub-L0 packets ship synced patterns alongside outer-block registrations. User-authored free-text inside claim-bearing blocks (correction notes, feedback prose) passes through the ADR-0108 §3 sanitizer (parity with LLM-emitted text) before storage and on every re-render.

**AC #W5 — Substrate-in-same-wave (C4).** Every substrate item named in W1 ships in W1, not W2+. W0 audit's substrate-gap column drives W1 scope; any gap surfaced mid-W2 escalates W1 reopening, not W1 deferral.

**Mid-wave substrate gap escalation procedure [arch F4]:** when a W2/W3/W4/W5 sub-L0 packet surfaces a substrate gap not in the W1 inventory, the gap is added to W1 scope via W1 sub-L0 amendment (not as a new Linear ticket in v1.4.5+). The downstream wave sub-L0 packet documents the W1 amendment reference and the unblock criterion. **Reviewer panel rejects sub-L0 packets that defer gaps to v1.4.5+ for substrate v1.4.4 surfaces consume.**

**C4-vs-path-α tie-breaker [codex challenge F3]:** C4 (substrate-in-same-wave) supersedes path-α offload when the substrate gap blocks a sub-L0 packet's named acceptance criterion. Path-α applies ONLY when the finding is hardening unrelated to a named AC. Worked example: a W2 reviewer finds DOS-459 envelope does not expose `inclusion_reason` for the StakeholderGrid chapter — the W2 visible-QA-state matrix for "subject-scope filtered" cannot be exercised → this is a **C4 reopen** of W1, NOT a path-α maintenance ticket. Memories `feedback_no_deferrals_period` (no deferrals) and `feedback_l2_path_alpha_to_maintenance_project` (path-α to maintenance) are not in conflict once this tie-break rule applies: C4-blocking findings stay in-wave; non-blocking hardening goes to maintenance.

**AC #W6 — Intelligence Loop integration check answered at sub-L0.** Every W1/W2/W3/W4 sub-L0 packet answers the 5 questions from CLAUDE.md "Critical Rules" for each new claim field / table column / user-visible intelligence surface. Reviewer panel cites the answer location in verdict.

**AC #W7 — Tauri UI freeze in effect throughout the wave.** No new UI work in Tauri React from 2026-05-15 forward (per memory `feedback_tauri_ui_freeze`). Existing Tauri surfaces in stasis until WP parity (W6). Tauri continues hosting runtime + MCP + keychain + dev/admin surfaces.

## 8. Out of scope (explicit deferrals)

| Out of scope | Where it goes |
|---|---|
| Report-shaped surfaces (BookOfBusiness, EbrQbr, SWOT, AccountHealth-as-report) | **v1.4.8** Reports as Shareable Intelligence (audience-aware shareable treatment) |
| Salience surfaces (Suggested Next Steps, What's Unusual, Why This Now) | **v1.4.6** Salience & Recommendations (paired with substrate) |
| Workspace memory surfaces (sources view, source detail, entity-intake) | **v1.4.5** Workspace Memory Refactor (paired with substrate) |
| MCP tool surface expansion (direct headless MCP) | **v1.4.7** MCP Server v2 (Abilities-First) |
| Self-Healing surfaces (review queue, audit, skillify proposals) | **v1.4.9** Self-Healing v2 — unless W0 audit reveals existing Tauri equivalent already in production |
| DOS-297 longitudinal topic threading | **v1.4.5** Workspace Memory |
| Recommendations layer (typed `RecommendationProposal` claims, salience scoring) | **v1.4.6** Salience |
| Causal lineage between claims | **v1.5.x** (per memory `project_causal_lineage_deferred`) |
| New Tauri React UI work | **Frozen** — no new Tauri UI from 2026-05-15 (memory `feedback_tauri_ui_freeze`). Tauri continues as runtime-host. |
| WordPress.com sync as publish target | Out of scope at v1.4.x (per ADR-0129 §5 paid-tier future scope) |
| Multi-tenant hosted substrate | Out of scope at v1.4.x (per ADR-0129 §5) |
| Hosted DailyOS agent backend | Out of scope at v1.4.x (BYOM only at free tier) |

**Tauri UI freeze reminder (memory `feedback_tauri_ui_freeze`):** existing Tauri surfaces stay in stasis until WP parity. Bug-fix-only triage applies to Tauri surfaces; no new UI features. Tauri continues hosting runtime + MCP server + keychain + dev/admin surfaces (NOT frozen).

## 9. Migration slots

**None at wave level.** Wave-level packet does not pre-claim migration slots.

**Per-W1 substrate item flagging:** W1 sub-L0 packet MUST enumerate which named substrate items need migrations (likely: receipt DTO if a `claim_receipt` table lands; review queue extensions if DOS-336 hook adds columns; meeting prep DTO if persisted state needed). Slots claimed per parallel-wave migration slot reservation rule (CLAUDE.md) in the W1 sub-L0 packet, not here.

**Coordination with v1.4.5+:** v1.4.5 Workspace Memory and v1.4.6 Salience also need migration slots in the same v1.4.x sequence. v1.4.4 W1 sub-L0 must coordinate slot blocks with v1.4.5/v1.4.6 pre-L0 to avoid the v1.4.1 W3-C/W4-A/W4-B v155 collision pattern (CLAUDE.md "Parallel-wave migration slot reservations").

**Forward-coupling hot spots [arch F6]:** (a) DOS-459 envelope `schema_version` lifecycle is forward-coupling-critical for v1.4.5/v1.4.6 envelope extension; W1 sub-L0 picks **semver-style with additive-only minor bumps** (new fields land at minor; breaking changes at major); v1.4.5 Workspace Memory adds source/ingestion claims and v1.4.6 Salience adds `RecommendationProposal` claims as **minor bumps** to the envelope shipped at W1. (b) Review-queue migration slot in v1.4.4 W4 is coordinated with v1.4.6 review-queue hook migration slot (per §13 #6 — v1.4.6 lands the DOS-336 candidate extension hook) to avoid v155-pattern collision; W4 sub-L0 documents the reserved slot block, v1.4.6 pre-L0 claims a disjoint block.

## 10. Architecture invariants

Wave-level invariants that govern every sub-L0 packet and every PR in v1.4.4.

| Invariant | Source | Sub-L0 mechanism |
|---|---|---|
| **Substrate-in-same-wave (C4).** Substrate the surfaces consume lands in W1, in v1.4.4, not deferred. | Reorientation doc §C4 | W2+ sub-L0 packets cite W1 substrate item as already-existing or already-scheduled-this-wave |
| **Surface-agnostic substrate (anchored decision #6).** Substrate works across Tauri / WP / Claude Desktop / Cursor / MCP without surface-specific branching. | Reorientation doc + ADR-0130 §1 + ADR-0129 §6 | Producer/renderer split per ADR-0130 §4; per-surface renderer; same `Composition` model |
| **Many blocks, not few (anchored decision #1).** Each Tauri component or pattern gets a Gutenberg block. Composite surfaces compose many small blocks. | Reorientation doc anchored decision #1 | W2/W3/W4 sub-L0 packets list per-block breakdown, not monolithic composite |
| **Inline edit captured as feedback claim (anchored decision #2).** No direct DB writes from WP. Pull/push of feedback flows through `services::claims::record_claim_feedback`. | Reorientation doc anchored decision #2 + CLAUDE.md critical rule | W4 sub-L0 codifies the wire shape; CI grep gate on `wp/dailyos/` for DB write attempts |
| **Tauri UI freeze (anchored decision #4).** No new Tauri React UI from 2026-05-15. Existing surfaces in stasis until W6 parity. | Reorientation doc anchored decision #4 + memory `feedback_tauri_ui_freeze` | Wave packet acceptance: no new `src/pages/` or `src/components/` React UI files outside bug-fix maintenance |
| **Runtime stays side-process (C3).** Tauri (or headless Rust binary) hosts runtime; WP plugin signs HTTP requests. PHP-extension and FFI-bridge runtime models out of scope. | Reorientation doc §C3 + ADR-0129 §7 | W1 substrate items use existing transport (signed loopback HTTP); no new transports introduced |
| **Theme owns no trust/provenance styling (v1.4.3 invariant carried forward).** Plugin owns essential trust/provenance CSS. Stock TwentyTwentyFive fallback supported. | v1.4.3 waves.md architecture invariants | W2/W3/W4 blocks ship trust/provenance via plugin-owned CSS, not theme overrides |
| **Producer commit on cache miss (v1.4.3 carried forward).** No signal-propagation invalidation bus required at v1.4.4 scope. | v1.4.3 W0 invariant | W3/W4 sub-L0 packets reuse v1.4.2 W4-F cache discipline |
| **Render-path authorization mandatory before cache lookup.** `authorize_local_render` skips rate-budget consumption; descriptor/actor/mode/scope checks remain. | v1.4.3 W0 invariant (Packet B §5.5) | All composite blocks invoke through `surface_runtime/project_composition` |
| **Chrome runtime-injection scope (from chrome lane L0).** Runtime DOM-injection permitted ONLY for FolioBar / FloatingNavIsland / AtmosphereLayer / MagazinePageLayout. **Default is Gutenberg blocks for ALL body-content and ALL surfaces other than the 4-module shell set.** Adding a 5th runtime-injection module requires (a) wave-plan amendment with explicit parity-vs-authorability cost analysis, (b) `ce-design-lens-reviewer` + WP-skill-grounded reviewer + `ce-architecture-strategist` re-approval, and (c) a documented invariant violation in the next retro K-out. **Default bias: when in doubt, ship a Gutenberg block; runtime injection is the exception.** [arch F7] | Chrome lane L0 §10 + reorientation doc + arch F7 | W2/W3/W4 sub-L0 packets keep body content as Gutenberg; future runtime-injection requires wave-plan amendment |
| **Canonical source-of-truth for chrome assets + design-system canonicity rule.** `.docs/design/reference/_shared/` is canonical for chrome; one-way sync via `sync-chrome.sh`. **Design-system canonicity extension [arch F8]:** W2/W3/W4 composite blocks translate existing canonical design (`.docs/design/reference/_shared/`, `.docs/design/patterns/*`, `.docs/design/primitives/README.md`) into block.json + render.php. **No new visual patterns are invented inside `wp/dailyos/blocks/`.** If a composite needs a pattern that does not exist canonically, the pattern lands canonical-first (per the chrome lane V1.3 §5.4.1 precedent) with `ce-design-lens-reviewer` approval, then is consumed by the block. | Chrome lane L0 §10 + arch F8 + memory `project_wp_primitives_translation_not_new_design` | W3 briefing surfaces consume FolioBar via existing chrome shell, not re-inject; W2/W3/W4 composites translate canonical design, do not invent |
| **Outer/inner block contract [arch F1 + WP-skill F1].** Entity-detail composites (Account / Project / Person / Meeting) and briefing composites (Daily / Meeting) use Gutenberg `InnerBlocks` with `templateLock: false` and a default `template` array enumerating chapters in canonical order. The outer Gutenberg block is a **renderer-side projection of `AbilityOutput<Composition>` per ADR-0130 §4** — the outer block's `InnerBlocks` correspond 1-to-1 with `Composition.sections[].blocks[]`. The substrate does NOT know about Gutenberg outer-vs-inner; the renderer does (Reading A). Inner blocks register WITHOUT a `parent` field — primitive blocks (`Pill`, `EntityChip`, `TrustBandBadge`, etc.) stay inserter-global so they compose into vanilla posts too (per ADR-0129 §2 "WordPress is the composition layer"). Outer block uses `providesContext` (e.g. `{ "dailyos/accountId": "account_id", "dailyos/projectId": "project_id" }`); inner blocks declare `usesContext: ["dailyos/accountId"]` to receive the entity binding without attribute passthrough. render.php for outer blocks calls `do_blocks( $content )` to render reordered inner blocks; the outer block's render.php is responsible only for chrome (subject binding, section dividers, ability invocation that feeds inner blocks via block context). The v1.4.2 `dailyos/account-overview` monolith is preserved as-is for backward compatibility; v1.4.4 W2 introduces `dailyos/account-detail` as the new outer block. | arch F1 + WP-skill F1 + ADR-0130 §4 + ADR-0129 §2 | W2/W3/W4 sub-L0 packets author outer+inner per this contract; primitives stay inserter-global; block context replaces attribute passthrough for entity binding |
| **Entity list pagination contract [arch F2 + WP-skill F2].** List shapes (account list, person list, project list, action list, history list, touchpoints feed, open-loops feed, activity log) consume substrate via `executeAbility()` (WP 7.0 client-side Abilities API per ADR-0129 §4), **NOT** via `@wordpress/core-data` `useEntityRecords`. List abilities return `AbilityOutput<List<T>>` shaped as `{ items, next_cursor: Option<String>, total_hint: Option<u64> }` — distinct from `AbilityOutput<Composition>`. Opaque cursor is server-encoded (no client parses it). W2 sub-L0 ships a shared `useAbilityCursor()` hook under `wp/dailyos/blocks/_shared/`; list blocks consume it. Composite blocks consuming list shapes invoke the list ability separately from the entity-detail Composition ability. Cursor invalidation on signal-driven refresh follows the v1.4.2 W4-F cache discipline (carried forward) — cursor resets when watermark changes. Per-ADR amendment to ADR-0130 §2 OR a sibling ADR is filed if `Composition` itself needs a cursor primitive (decided at W1 sub-L0 with `/codex consult` + `ce-architecture-strategist` in the panel). | arch F2 + WP-skill F2 + ADR-0129 §4 | W1 envelope contracts (DOS-459 / DOS-460) carry first-page slice + `next_cursor` for each list chapter so initial render is one round-trip; W2 sub-L0 ships shared hook |
| **Per-project tint via CSS custom property [WP-skill F3].** Per-project tint flows as a CSS custom property on the outer-block wrapper (`style="--dailyos-project-tint: <hex>;"` injected by render.php from the substrate's project claim), NOT as a `theme.json` palette slug or block style variation. theme.json palette stays generic; primitives consume the custom property. This is the narrow exception to the `no inline CSS` rule (memory `feedback_no_inline_css`) — runtime-computed values on a wrapper element via CSS custom property are explicitly permitted. CPT decision (`dailyos_project` registration) is orthogonal: default-no per the `account-overview` comparator (account is not a CPT; block has an `account_id` attribute); justify-yes only if a non-substrate reason emerges (likely none). | WP-skill F3 + memory `feedback_no_inline_css` exception clause | W2 sub-L0 for Project Detail follows this pattern; deviation requires `ce-design-lens-reviewer` justification |
| **Refresh model: pull-on-render + user refresh; no push-invalidation bus at v1.4.4 [arch F5].** W2/W3/W4 composite blocks invoke the producing ability on render; staleness is conveyed via `FreshnessIndicator` primitive (v1.4.3 shipped) and trust-band downgrade per ADR-0105. **The Intelligence Loop §3 (signals + invalidation) question is answered uniformly across v1.4.4 blocks: "pull on render + user-initiated refresh; no push invalidation at this version."** Signal-propagation-driven refresh is a v1.4.6 Salience concern (signal correlation drives recommendations) or v1.4.9 Self-Healing concern. | arch F5 + ADR-0105 + v1.4.3 W0 producer-commit-on-cache-miss | Sub-L0 packets answer Intelligence Loop §3 with this uniform answer; do not invent per-surface push channels |
| **Block apiVersion 3 mandatory [WP-skill F6].** All new W2–W5 blocks declare `"apiVersion": 3` (WP 6.9+ enforcement; 7.0 iframed editor compatibility). Sub-L0 packets must surface any apiVersion downgrade as scope-revision. The v1.4.3 starter kit already declares apiVersion 3; no per-block override permitted. | WP-skill F6 + WP 6.9/7.0 enforcement | Every new block.json carries apiVersion 3; CI lint optional |
| **L2 bounded by acceptance criteria (memory `feedback_l2_must_review_against_acceptance_criteria`).** Path-α findings → maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`. **C4 supersedes path-α when the substrate gap blocks a named AC — see AC #W5 tie-break.** | CLAUDE.md + engineering ladder + codex challenge F3 | Every sub-L0 packet sizes ACs tightly; L2 cycle-2 path-α offload per memory; C4-blocking findings reopen W1, not maintenance |

## 11. Reviewer matrix (L0 panel)

Wave-level L0 reviewer panel — 5 reviewers per CLAUDE.md + engineering-ladder.md + memory `feedback_wp_skill_grounded_reviewer_for_wp_l0`. Sub-L0 packets (W1/W2/W3/W4/W5/W6) **need their own reviewer panels** — this packet does not pre-empt them.

| Reviewer | Scope at wave level |
|---|---|
| `/codex challenge` | Adversarial — premise-check the wave structure, stress-test "no façades" rule, challenge "many blocks not few" vs composite reality, verify Tauri freeze interactions, surface scope inflation risk |
| `/codex consult` | Architecture continuity with ADR-0129 / ADR-0130; substrate-consumed inventory completeness; v1.4.3 → v1.4.4 dependency chain |
| `ce-architecture-strategist` | Wave-program shape; sub-L0 packet boundaries; substrate-in-same-wave rule application; coordination with v1.4.5/v1.4.6 parallel substrate work |
| `ce-design-lens-reviewer` | Surface-migration design coherence; visible-QA-state matrix; many-blocks-not-few interpretation; user-can-do-the-thing acceptance shape |
| **WP-skill-grounded reviewer** (per memory `feedback_wp_skill_grounded_reviewer_for_wp_l0`) | WordPress / Gutenberg / theme.json correctness across W2–W5 composite blocks; block.json + render.php discipline; chrome shell composition; loopback HTTP boundary |

**Pass rule:** unanimous APPROVE. Per CLAUDE.md pacing rule, 2 revision cycles without convergence ⇒ L6 escalation. Memory `feedback_review_loop_l6_policy` allows continued looping on architectural/critical/high; class-pattern recurrence triggers system-wide sweep per memory `feedback_systemic_look_for_recurring_issue_classes`.

**Sub-L0 reviewer panels:**
- **W1 (substrate gaps):** add `/cso` or `security-auditor` per Amendment 3 (substrate touches trust-boundary hardening DOS-477, receipt privacy DOS-341, surface authorization).
- **W2 (entity surfaces):** add `accessibility-tester` for user-facing surface; consider `architect-reviewer` for envelope consumption pattern.
- **W3 (briefing surfaces):** same as W2 + WP-skill-grounded reviewer maintained.
- **W4 (action surfaces):** add `/cso` (feedback wire-through trust boundary) + `accessibility-tester`.
- **W5 (system / history surfaces):** add `qa-expert` for adversarial fixtures.
- **W6 (parity proof):** add `architect-reviewer` (integrated state); Suites S / P / E run as L3 wave-close.

## 12. References

- ADR-0083 — Product vocabulary (banned strings; user-facing copy discipline).
- ADR-0102 — Abilities as runtime contract.
- ADR-0105 — Provenance as first-class output.
- ADR-0108 — Provenance rendering and privacy (64KB serialized cap; actor-filtered rendering).
- ADR-0111 — Surface-independent ability invocation (`SurfaceClient` actor).
- ADR-0125 — Claim anatomy: temporal, sensitivity, type registry.
- ADR-0128 — Headless DailyOS: MCP as co-equal product surface.
- ADR-0129 — Composable surfaces: WordPress Studio as primary surface.
- ADR-0130 — Surface-independent Composition contract (`Composition` model, `BlockType` taxonomy, fallback projection).
- ADR-0132 — Pill primitive dual existence (chrome `.Pill_*` vs block `.dailyos-pill*`).
- `.docs/plans/wp-foundation-roadmap-reorientation.md` — canonical reorientation doc; v1.4.4 section at lines 80–106.
- `.docs/plans/v1.4.4-waves.md` — placeholder (superseded by this packet).
- `.docs/plans/v1.4.3-waves.md` — v1.4.3 wave plan; v1.4.4 prerequisite.
- `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W3-chrome-lane-pulled-forward.md` — chrome lane L0 (already shipped substrate).
- `.docs/plans/engineering-ladder.md` — L0–L6 skill matrix + K-channel.
- Linear project: [v1.4.4 — WordPress Surface Migration](https://linear.app/a8c/project/v144-wordpress-surface-migration-877aaa780177) (id `f8b805d9-f3d4-41b4-a446-51bbb7e05f2e`).
- Maintenance project for path-α: [`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`](https://linear.app/a8c/project/codebase-maintenance-production-quality-b8e6aea4d47e) (DailyOS Codebase Maintenance & Production Quality).
- Memory `feedback_wp_skill_grounded_reviewer_for_wp_l0` — 5th-reviewer addition for WP-touching L0 packets.
- Memory `feedback_tauri_ui_freeze` — no new Tauri React UI from 2026-05-15.
- Memory `project_v14x_renumber_2026_05_17` — version sequence lock.
- Memory `feedback_l2_must_review_against_acceptance_criteria` — L2 bounded by AC.
- Memory `feedback_systemic_look_for_recurring_issue_classes` — class-pattern sweep policy.
- Pulled-forward commits on this branch: `52d25db5..26062496` inclusive (DOS-721 at `52d25db5`, DOS-729 at `771a3d5d`, DOS-730, DOS-731, DOS-732, DOS-724, DOS-722, plus chrome lane commit `0342bf7a`); theme.json generator wiring `26062496` (DOS-336, magazine-theme follow-up, landed in same window — not chrome lane proper) [consult F1 + F2].

## 13. Open architectural questions

**Resolutions locked 2026-05-20:**

| # | Question | Decision | Implications |
|---|---|---|---|
| 1 | Block granularity for entity-detail composites | **One outer block, N inner pieces — Reading A per ADR-0130 §4.** Account Detail is a single `dailyos/account-detail` Gutenberg block that is a renderer-side projection of `AbilityOutput<Composition>`; its ~25 chapters are inner blocks corresponding 1-to-1 with `Composition.sections[].blocks[]`. The substrate does NOT know about Gutenberg outer-vs-inner; the renderer does. Same pattern for Project / Person / Meeting Detail and the W3 briefing composites. **Inner list chapters invoke their own paginated ability per decision #2; outer envelope (DOS-459) carries first-page slice + cursor for each list chapter so initial render is one round-trip [codex challenge F2].** Implementation details in §10 invariant "Outer/inner block contract." | W2 sub-L0 designs the outer+inner template per entity. Inner blocks reuse v1.4.3 primitives and stay inserter-global (no `parent` field). Outer uses `providesContext`; inner uses `usesContext`. Consistency wins over freedom. |
| 2 | Entity list pagination model | **Server-side pagination via cursor; WP-side via `executeAbility()`.** List abilities return `{ items, next_cursor: Option<String>, total_hint: Option<u64> }`; block invokes the ability again on scroll/filter change via the WP 7.0 client-side Abilities API per ADR-0129 §4 (NOT `useEntityRecords`). W2 sub-L0 ships shared `useAbilityCursor()` hook under `wp/dailyos/blocks/_shared/`. Implementation details in §10 invariant "Entity list pagination contract." | W1 envelope contract for list shapes (DOS-459 / DOS-460) carries `next_cursor` from day one. Opaque server-encoded cursor; no client-side hide-extras pattern. Outer envelope carries first-page slice for embedded list chapters per #1 sub-clause. |
| 5 | Tauri shell deprecation strategy | **Flag-flip at W6 — UI-only.** Single PR hides Tauri React **end-user** UI behind a build flag (e.g. `VITE_HIDE_LEGACY_UI=true`). Both surfaces visible until W6; one switch at parity gate. Flag-flip hides W0-classified end-user surfaces only [arch F3]; dev/admin surfaces from the W0 5th list stay visible (settings-runtime, keychain debug, ability runtime status, MCP server status). Flag MUST NOT alter `src-tauri/tauri.conf.json` `externalBin` packaging or the `build-mcp.sh` stub-create-before-cargo-build dance [WP-skill F4]. Parity-proof artifact includes runtime-still-running smoke test (see §5.6 acceptance checklist item (iv)). | W6 sub-L0 owns the flip PR. No per-wave Tauri removal coupling. Tauri continues hosting runtime + MCP + keychain + dev-admin per memory `feedback_tauri_ui_freeze`. |
| 6 | DOS-336 candidate extension hook timing | **v1.4.6 lands the hook itself.** W1 leaves claim review queue alone. v1.4.6 designs + lands the candidate hook with its actual consumer. | W1 scope shrinks; v1.4.6 accepts a small migration against post-v1.4.4 review-queue code. Per memory `feedback_check_substrate_before_authoring_primitives`. |

**Deferred to sub-L0 (still open):**

3. **W3 briefing surfaces — FolioBar consumption pattern.** Chrome lane L0 §10 invariant restricts runtime-injection to 4 shell modules. W3 briefing surfaces (Daily Briefing, Meeting Briefing) need to communicate state to FolioBar (meeting prep readiness count, briefing status). Does W3 use chrome.js `chrome_config()` data attributes? A block-level wrapper that bridges to FolioBar? **Resolves at:** W3 sub-L0.

4. **W6 parity-proof artifact format.** Per-surface side-by-side proof bundle — HTML report? Linear-attached markdown with screenshots? Per-surface video walk-through? Affects per-surface acceptance shape and reviewer expectations. **Resolves at:** W6 sub-L0 OR pre-W2 if early surfaces benefit from baseline.

7. **Settings surface migration shape.** Per W0 audit — full migration of all Tauri Settings to WP blocks? Thin admin surface for runtime/keychain settings on Tauri side, with theme/feedback settings as WP? Migration path matters for clean-machine UX (DOS-577 carries to W5 / W6). Partly resolved by audit (user-facing tabs in W5, diagnostics deferred to maintenance). **Resolves at:** W5 sub-L0.

8. **`dailyos_project` tint resolution.** From chrome lane L0 §8 (DOS-725) — Project Detail block CPT registration needs project tint resolved via ADR-0077 amendment OR new ADR. **Mechanism locked at the wave layer [WP-skill F3]:** per-project tint flows as a CSS custom property on the outer-block wrapper (`style="--dailyos-project-tint: <hex>;"` from render.php, sourced from the project claim), NOT as a theme.json palette slug or block style variation. See §10 invariant "Per-project tint via CSS custom property." CPT decision (`dailyos_project` registration) is orthogonal to tint mechanism: default-no per the `account-overview` comparator; justify-yes only if a non-substrate reason emerges. **Resolves at:** W2 sub-L0 with `ce-design-lens-reviewer` + (if new ADR for CPT) `ce-architecture-strategist`.
