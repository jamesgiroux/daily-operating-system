# DailyOS — "If we started over" architecture examination

**Date:** 2026-05-29
**Status:** thinking artifact under active iteration — NOT a plan, NOT approved
**Frame:** The 2026-05-28 production DB loss is being used as a *forcing function*, not an emergency. The question is explicitly: *if we started over, with everything we've learned, what would we do differently?* This is decoupled from "rewrite now." There is no urgency.

This doc exists to be **red-teamed** by independent reviewers. It deliberately states both the converged design and the strongest objections to it, so reviewers adjudicate rather than pile on.

---

## Context the reviewers need

DailyOS is a "personal intelligence" app (category peers: GBrain by Garry Tan, OpenClaw, Hermes; adjacent: Claude Cowork, Gemini Personal Intelligence). Mission: *make intelligence personal — memory + judgment you can trust.* Canonical product framing in `.docs/design/product/{MISSION,VISION,PRODUCT-THESIS}.md`.

**Competitive read (mid-2026):** the *memory/retrieval* layer is commoditizing fast (GBrain = markdown-in-git + pgvector + typed entity graph + 30 MCP tools, MIT-licensed, no app, engineer-shaped). Every 2026 memory survey names the same *unsolved* problem: **judgment / salience** — knowing what matters, what to trust, what *not* to remember, temporal trust decay, trust as a testable contract. That unsolved layer is exactly DailyOS's existing claim + abilities + trust substrate (ADR-0102, 0123, 0125, 0126).

**Current state (verified against code 2026-05-28):**
- SQLCipher (encrypted SQLite) is both primary store AND foreground read path; ~387 MB at ~20 user entities; 195 tables.
- `db_backup::rebuild_from_filesystem` rebuilds **only** accounts/projects/people from workspace JSON. Claims, signals, embeddings, trust scores, meeting history, enrichment state are SQLite-only and NOT rebuildable today.
- Raw sources mostly retained/re-fetchable: 276 transcript files on disk, Gmail re-fetchable, Glean remote. So most of the 387 MB is *derived* intelligence over recoverable sources.
- A process-wide `WRITE_TRANSACTION_GATE` serializes all writes; ~190–205 `ActionDb::open()` bypass sites open their own handles (ADR-0133 names this anti-pattern; W1-C is the closure).
- Yesterday's corruption root causes — `pkill` on a hot WAL writer, a backup using `step(-1)`, an over-aggressive recovery predicate — are ALL patched in PR #416. None is "SQLite is the wrong store."

---

## James's assessment — what he'd do differently (his words, lightly annotated)

1. **Wouldn't use SQLCipher.** *(Ambiguous: drop encryption — clearly right, FileVault already encrypts disk, AES-per-page is the latency driver and the `.recover`-OOM cause — vs. drop SQLite-the-engine, a separate and open question. Plain SQLite is still not human-readable, so it doesn't satisfy inspectability either.)*
2. **Composable surface components** (add/remove/customize) **instead of locked-in full-page layouts.** *(= ADR-0130 surface-independent composition; WP block model fits.)*
3. **Start from claims rather than entities.** *(Claim as the primitive; entities become derived views. Deepest idea here.)*
4. **Tighter connection to the workspace and local file output.**
5. **Make enrichment less foreground.** *(Fixes 1.5–8s foreground read latency; amortizes embedding recompute.)*
6. **Feedback/learning more freeform, less click-a-button.** *(More knowledge-worker-shaped; precision risk for a trust product.)*
7. **Think about the core, not the CS-shaped role preset.** *(Role-agnostic substrate; CS is a seat, not the architecture.)*

---

## The converged architecture (what the conversation drifted toward)

```
SURFACES (knowledge-worker-shaped — no git, no CLI)
  • WordPress Studio — visual render + proactive briefing + optional in-WP chat   ← primary
  • Proactive push — briefing into email / calendar / notification
  • Claude desktop / any MCP client — "ask anything" + agent interop              ← optional
        │  MCP tools + composed render fragments (ADR-0130)
DAILYOS SUBSTRATE  (one local daemon, invisible)
  • Judgment runtime — abilities, trust, salience, belief revision, corrections   ← THE MOAT (already exists, works)
  • Claim-native memory (memory + judgment unified in the claim)
  • Truth: human-readable files in an owned folder; auto-versioned underneath (never user-facing)
  • Cache: SQLite + vector index — rebuildable from files
        │  abilities normalize sources → claims
SOURCES  • email • transcripts on disk • calendar • Glean • Slack
```

Decisions bundled into this: (a) storage model SQLite-primary → files-primary; (b) surface model app → WP + MCP, deprecate macOS app; (c) security right-sizing to local single-user OS boundary; (d) substrate as a single local daemon; (e) LLM lives in the surface, substrate is a tools+judgment engine (don't compete with Cowork).

---

## Red-team (self-administered; reviewers should extend or refute)

1. **The corruption doesn't justify a rewrite.** All three root causes are patched (PR #416). The trigger was a *dev* workflow (`pkill`, 16 restart cycles), not production reality. Steady-state corruption rate in real use may be ~0. "Permission slip" is a post-trauma narrative.
2. **Rewrite bets the moat to remodel the basement.** Judgment (claims/trust/abilities) is the differentiator, it already exists and works; storage/surface are the *least* differentiated parts. A rewrite risks the crown jewels to fix plumbing while competitors ship.
3. **Files-as-truth discards ACID.** SQLite gives atomic multi-record commits; multi-file writes can partially fail silently — worse than loud DB corruption for a *trust* product. *Partial mitigation:* James's claims-first + append-mostly model makes belief revision atomic-per-append, largely defusing this.
4. **"Disposable cache" is false for embeddings.** Vector recompute costs real money + hours and grows with entities; the vector layer is expensive derived state you must protect → back where we started. *Partial mitigation:* background/incremental enrichment (point 5) amortizes it.
5. **Deployment may get WORSE.** The app's real virtue was *packaging* (one local, private, non-technical, always-launchable thing). Replacing it with daemon + WordPress Studio (a local-dev tool) + LLM client = three processes a non-technical user must keep alive; silent daemon death = briefing just doesn't arrive (bad for a trust product). "Claude renders better" critiques *rendering*, not *packaging*.
6. **The mission's proactive requirement fights the surface choices.** "Know before you ask" needs push (notifications → native presence we're deprecating; or email/calendar send → a security surface we're shrinking). Local-first rules out hosted WP; local WP is the technical-to-run option. The app was the cleanest answer to "local + private + non-technical + can push."

---

## What survives the red-team

- **Security right-sizing** — clearly right, independently justified, *cheap* (mostly deletion). Stands alone.
- **Drop SQLCipher *encryption*** — clearly right (cost, recovery, redundant vs FileVault). Engine choice separate.
- **Judgment-as-moat positioning** — right as strategy regardless of architecture.
- **Claims-first substrate (point 3)** — strongest idea; improves the design and defuses the ACID objection.
- **Composable surfaces (2), background enrichment (5), tighter file output (4), role-agnostic core (7)** — well-grounded, incremental, low-risk.
- **MCP as integration surface** — low-risk, additive, already ADR'd.
- **Readable-file projection** for "inspect/move/leave-with" — but as a projection *out of* the store, not necessarily as the primary write path.

## What is contested / unresolved

- Files-as-**primary-truth** vs. transactional store with readable **projection out** (ACID vs inspectability — claims-first changes the calculus).
- Deprecate the app vs. keep it as the *packaging* answer to local+private+non-technical+push.
- WordPress Studio as primary surface for genuinely non-technical knowledge workers (is "run local WordPress" any less engineer-shaped than the app we'd reject?).
- Daemon lifecycle/observability for non-technical users (silent failure of a trust product).
- Embedding/vector recompute cost as a function of entity scale.

## Recommendation under examination

Decouple the three bundled decisions. **Security right-sizing + dropping the cipher** are cheap and clearly right — do them. **Storage-model and surface-model changes** are expensive and contested — treat as hypotheses to *validate cheaply before betting the moat* (handoff open-question #4: smallest experiment = route ONE entity type through the new model for a sprint behind the current store; measure rebuild cost, write integrity, and whether the daemon+surface constellation survives a week without silent death).

---

## Questions for reviewers

1. Is claims-first (entities as derived views) the right substrate primitive, or does it create query/perf/consistency problems that entity-first avoids?
2. Does files-as-primary-truth survive the ACID/partial-write objection even under append-mostly claims? Or is "transactional store + readable projection out" strictly better?
3. Is deprecating the app a category error (throwing away packaging), or is the daemon+WP+MCP constellation genuinely more fit for a non-technical knowledge worker?
4. Is the judgment moat actually *validated*, or are we rearchitecting around an unproven differentiator ("flashes," not "systematic")?
5. What's the single cheapest experiment that would most reduce uncertainty before any commitment?
6. What load-bearing assumption here is most likely to be fatally wrong?

---

## Adjudication — 4-reviewer synthesis (2026-05-29)

Four independent adversaries (architecture, feasibility, product-lens — all code-grounded — plus a non-Claude codex pass). Strong convergence.

**The one finding that reframes everything (unanimous): the cache/truth model is INVERTED.** 192 of 195 tables — claims, contradictions, corroborations, trust ledger, embeddings, the entire ADR-0126 invariant surface, i.e. *the moat* — live ONLY in SQLite, with NO filesystem representation and NO rebuild path (`db_backup.rs:478-500` rebuilds 3 entity tables). So "files = truth, SQLite = disposable cache" is backwards. The correct model is **transactional store = truth, human-readable files = projection OUT** (for the mission's inspect/move/leave-with property). This dissolves the files-as-truth, disposable-cache, and rebuild-from-files objections at once. Promote from "contested" to **decided: projection-out, not files-primary.**

**Show-stoppers for the bundled cutover (do NOT do as bundled):**
- **Files-as-primary-truth breaks the trust contract (codex P0, architecture HIGH).** Belief revision is 4–6 rows across 4 tables (ADR-0126 §1/§2/§6); claims-first *multiplies* rows per change, it does NOT make revision atomic-per-append. Multi-file writes have no cross-file transaction barrier → silent inconsistent belief state on a half-finished write. Fatal unless truth stays transactional.
- **Deprecating the app is a packaging regression (codex P0, feasibility, product-lens).** The app's virtue was never rendering — it's the *control plane*: lifecycle, updates, observability, and the only native push primitive (`tauri-plugin-notification`, which the proactive mission requires). Daemon + WordPress Studio (a local *dev* tool, MORE technical than a signed .app) + LLM client = three processes a non-technical user keeps alive; silent daemon death = the briefing just never arrives. Keep the app as control plane; WP/MCP are optional surfaces behind it, not the replacement.
- **Migration is a one-way door (feasibility, code-verified).** The live DB is opaque SQLCipher keyed to the macOS keychain held *by the app being deprecated*; the me-shape (`claim_feedback`, `claim_corroborations`, `claim_contradictions`, `entity_resolution_feedback`, `*_dismissals`) has no file projection and can't be re-derived from sources. So "drop cipher" and "kill app" are sequenced dependencies — decrypt-and-export the corrections THROUGH the running app FIRST.

**Corrections to this doc's own red-team:**
- Embeddings are **local `fastembed` CPU inference, zero $** (`embeddings.rs:21,79`). Red-team #4's "costs real money" is FALSE. The real constraint is wall-clock CPU recompute that janks the UI — already visible at 20 entities (8s stalls). So the vector layer is expensive *protected* derived state (versioned artifact), not disposable scratch — true, but for the CPU-jank reason, not cost.
- Claims-first as THE primitive is contradicted by code: `subject_ref` is non-canonical JSON; migration `263_claim_subject_lookup_index.sql` already *un-inverted* this with expression indexes. The shipped model is entity-anchored claims, which works. Claims-first-with-entities-derived is a query-cost regression, not a clear win.

**Meta-warning (unanimous): the corruption is being overfit.** All three root causes are patched (PR #416) and were dev-workflow-induced. Hard rule adopted: **no rewrite commitment without evidence the *patched* current architecture still fails under realistic production use.** The differentiated layer (judgment/trust) deserves the iteration, not a storage/surface rewrite that also orphans the in-flight ADR-0133 (178 bypass sites) and ADR-0130 work.

**What survives as worth doing (decoupled, individually justified):**
- Tier 1 (cheap, clearly right, no experiment needed): (A) drop SQLCipher *encryption* → plain SQLite, rely on FileVault; (B) right-size security to OS boundary; (C) project readable claim/feedback files OUT of the store — gives the mission property AND creates the me-shape backup that doesn't exist today (fixes the migration show-stopper as a side effect).
- Tier 2 (directionally right, incremental): (D) background enrichment (already shipped — lean in); (E) composable surface components (ADR-0130) as render surfaces *behind the app shell*; (F) freeform feedback as hybrid (freeform in → system proposes structured interpretation → user confirms).
- Tier 3 (contested — decide with evidence first): files-primary / daemon / app-deprecation only AFTER a crash-injection shadow spike survives a week unattended. Default: don't.

**The actual highest-value move (product-lens): prove the moat first.** Before any architecture, run 2 weeks of instrumentation on the SHIPPED app: does the judgment layer change what the user does vs. a recency-sorted dump? Would the user notice if it were off? If the moat isn't yet felt, no storage/surface decision matters — make the differentiator demonstrably real before building a vault around it.
