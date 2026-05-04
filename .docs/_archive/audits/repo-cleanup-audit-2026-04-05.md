# DailyOS Repo Cleanup Audit — 2026-04-05

## Executive Summary

This repo does have real documentation value, but it is carrying a lot of planning residue.

The main pattern:
- **Architecture / ADR / design docs are worth preserving** and should remain in-repo.
- **Issue specs and version briefs have sprawled** and are now the biggest cleanup opportunity.
- **A few directories and files are clearly transitional or stale** (`.docs/issues`, large parts of `.docs/plans`, `.docs/BACKLOG.md`, top-level `release-notes.md`, `.stash/`, `notification-demo.html`, some naming drift in plan files).
- **The latest released tag is `v1.1.1` (2026-04-02)**, while the repo still contains many planning docs for older shipped versions plus future briefs that are now partly inconsistent.

If the team is moving execution tracking to **Linear**, the best cleanup move is **not** “delete docs broadly.” It is:
1. keep durable knowledge in git,
2. migrate execution/planning artifacts to Linear,
3. archive or remove repo-local planning residue once migrated,
4. tighten `.docs/` into a clearer information architecture.

---

## Repo State Snapshot

### Git / release state

- Current branch: `dev`
- `dev` is ahead of `origin/dev` by 1 commit
- Latest tag: `v1.1.1`
- Latest release tag commit equals `origin/main` merge base for the `v1.1.1` release line
- Commits after `v1.1.1` on `main`/`dev` are small follow-up changes plus `I660`/`I661`/`I662`

### High-level inventory

Top-level notable directories/files:
- `.docs/` — main internal documentation hub, **574 files / 517 markdown files**
- `docs/` — public website assets/docs
- `design/` — top-level product/design statements referenced by `README.md`
- `_archive/` — older archived app/code/docs snapshot
- `release-notes.md` — top-level product release notes
- `notification-demo.html` — likely one-off artifact / demo page
- `.stash/` — contains a single patch file

### Internal docs density

- `.docs/issues/` — **301 markdown files**
  - active-ish/current: **115**
  - `_done`: **166**
  - `_archive`: **20**
- `.docs/plans/` — **41 markdown files**
- `.docs/decisions/` — **101 ADRs**
- `.docs/architecture/` — **15 docs**
- `.docs/design/` — **18 docs**

This confirms the main cleanup target is not “documentation in general”; it is **planning and issue tracking documentation specifically**.

---

## Findings by Area

## 1) `.docs/decisions/` — preserve as historical record

### Recommendation
**Preserve in repo.** This is durable product/architecture history.

### Why
- The ADR set is large but structurally valuable.
- These files are exactly the kind of docs git is good at preserving.
- They are foundational references for architecture, product vocabulary, and design intent.

### Action
- Keep under version control.
- Add an ADR index/metadata pass later if discoverability becomes a pain.
- Do not migrate ADRs to Linear.

**Bucket:** Preserve as historical/decision record

---

## 2) `.docs/architecture/` — preserve, but refresh selected docs

### Recommendation
**Preserve, but update selectively.**

### Why
- `README.md` in this folder already distinguishes between generated docs, manually maintained docs, and frontend audit docs that may have drifted.
- This is a healthy sign: the folder has intent and structure.
- Some docs explicitly admit drift, which means they should stay but be refreshed.

### Strong keep candidates
- `.docs/architecture/README.md`
- `.docs/architecture/COMMAND-REFERENCE.md`
- `.docs/architecture/DATA-FLOWS.md`
- `.docs/architecture/DATA-MODEL.md`
- `.docs/architecture/LIFECYCLES.md`
- `.docs/architecture/MODULE-MAP.md`
- `.docs/architecture/PIPELINES.md`
- `.docs/architecture/SELF-HEALING.md`

### Needs-refresh candidates
Per the folder README, likely first refresh targets:
- `.docs/architecture/FRONTEND-COMPONENTS.md`
- `.docs/architecture/FRONTEND-STYLES.md`
- `.docs/architecture/FRONTEND-TYPES.md`

### Archive handling
- `.docs/architecture/_archive/` looks appropriate for superseded analysis/proposals.
- Keep archived docs, but make sure archive items are clearly marked as superseded.

**Bucket:** Preserve but update

---

## 3) `.docs/design/` — preserve, but reconcile with top-level `design/`

### Recommendation
**Preserve core design system docs, but reduce duplication/confusion between `.docs/design/` and top-level `design/`.**

### Why
There are currently two design doc homes:
- `.docs/design/` — implementation/design system/internal UX reference
- `design/` — top-level philosophy/principles/vision statements

This split is not inherently wrong, but it is easy to confuse:
- `README.md` points to top-level `design/PHILOSOPHY.md`, `PRINCIPLES.md`, `VISION.md`
- internal design system guidance lives under `.docs/design/`

### Keep in repo
- `.docs/design/*` should stay; it is operational design documentation.
- `design/PHILOSOPHY.md`, `design/PRINCIPLES.md`, `design/VISION.md` should likely stay as canonical high-level product/design framing.

### Cleanup need
- Decide whether `design/` remains the public-facing/high-level layer while `.docs/design/` remains internal implementation guidance.
- If yes, document that split clearly in both READMEs.
- If no, merge one into the other.

### Possible stale candidate
- `design/CSS-DESIGN-SYSTEM-AUDIT-2026-03-21.md` may be useful as an audit artifact, but probably belongs under `.docs/audits/` rather than top-level `design/`.

**Bucket:** Preserve but update

---

## 4) `.docs/issues/` — migrate to Linear, then remove from active repo surface

### Recommendation
**This is the single biggest migration/removal target.**

### Why
- 301 issue markdown files is a full secondary issue tracker living beside git and now beside Linear.
- The current setup includes active issues, done issues, archived issues, and references from backlog/plans.
- This is likely the largest source of clutter, duplication, and staleness.

### What to preserve
- Not every issue spec needs permanent in-repo retention.
- Preserve only exceptional cases as durable technical records, e.g. if an issue doc contains:
  - architectural rationale not captured elsewhere,
  - migration/operational procedure,
  - unusually important acceptance criteria that became system behavior,
  - postmortem-grade context.

### Default policy
- **Move issue tracking to Linear.**
- For each markdown issue doc, do one of:
  1. migrate summary/ACs/links into Linear issue,
  2. preserve as ADR/research/runbook if it contains durable knowledge,
  3. otherwise archive temporarily and remove from repo later.

### Specific repo observation
Many issue files are still current and recently touched (`I660`, `I661`, `I662`, etc.), which suggests the team is still using the markdown tracker operationally. That means cleanup should be staged, not abrupt.

**Bucket:** Migrate to Linear and remove from repo later

---

## 5) `.docs/plans/` — mostly migrate version briefs to Linear project descriptions

### Recommendation
**Future planning should move to Linear project descriptions; shipped version briefs should be archived or reduced.**

### Why
This folder mixes:
- shipped version plans,
- dissolved versions,
- active future briefs,
- historical execution trackers,
- at least two naming/content mismatches.

### Important inconsistencies found
- `.docs/plans/v2.1.0.md` has heading `# v1.1.0 Version Brief`
- `.docs/plans/v2.2.0.md` has heading `# v1.2.0 Version Brief`

That is a strong signal of planning drift / copied-forward docs without cleanup.

### Current released reality vs plans
Latest shipped tag is `v1.1.1`, so these are effectively historical planning artifacts now:
- `.docs/plans/v1.0.1.md`
- `.docs/plans/v1.0.2.md`
- `.docs/plans/v1.0.3.md`
- `.docs/plans/v1.0.4.md`
- `.docs/plans/v1.1.0.md`
- `.docs/plans/v1.1.1.md`

These can likely be:
- archived as historical planning records, or
- condensed into release notes / changelog provenance if no longer actively needed.

### Future-facing files to migrate
Likely migrate to Linear project descriptions / initiatives:
- `.docs/plans/v1.1.2.md`
- `.docs/plans/v1.1.3.md`
- `.docs/plans/v1.2.0.md`
- `.docs/plans/v1.3.0.md`
- `.docs/plans/v1.4.0.md`
- `.docs/plans/v2.1.0.md`
- `.docs/plans/v2.2.0.md`

### `_archive/` under plans
This is already doing useful work. Keep it, but only for historical snapshots worth retaining.

**Bucket:** Mostly migrate to Linear and remove from repo later; shipped plans can be archived as historical record

---

## 6) `.docs/BACKLOG.md` — likely stale as a working system, probably remove after Linear migration

### Recommendation
**Treat as migration source, not future canonical tracker.**

### Why
- It is huge and blends active roadmap, closed history, dissolved plans, implemented items, and references to version briefs.
- It says closed issues live in `CHANGELOG.md`, while `.docs/issues/` and `.docs/plans/` are also carrying overlapping planning state.
- This is exactly the kind of “repo as project manager” artifact that becomes stale fastest.

### Best use now
- Mine it for migration into Linear.
- Preserve a final frozen snapshot if desired.
- Then remove it from active use.

**Bucket:** Migrate to Linear and remove from repo later

---

## 7) Release docs: `CHANGELOG.md`, `release-notes.md`, `.docs/RELEASE-*`

### `CHANGELOG.md`
**Keep.** This is canonical release history.

**Bucket:** Preserve as historical record

### `release-notes.md`
Likely still useful, but overlaps with changelog and shipped-version briefs.

Recommendation:
- Keep if this is the human-facing/product-marketing release narrative.
- Otherwise fold it into website/docs or generated release notes.

**Bucket:** Preserve but update, or simplify if redundant

### `.docs/RELEASE-CHECKLIST.md`
**Keep.** Operationally useful.

### `.docs/RELEASE-POLICY.md`
**Keep.** Governance/process doc.

**Bucket for both:** Preserve but update

---

## 8) `.docs/research/`, `.docs/runbooks/`, `.docs/audits/` — mostly preserve

### Recommendation
**Preserve, but curate.**

### Why
These are knowledge artifacts rather than tracker artifacts.

### Notes
- `.docs/research/` appears valuable, though some files are probably exploratory and could later move to archive.
- `.docs/runbooks/` is low-volume and useful.
- `.docs/audits/` is the right home for dated audit outputs.

### Cleanup opportunity
- Add frontmatter or naming conventions so dated research/audits are easier to distinguish from canonical docs.

**Bucket:** Mostly preserve as historical/working knowledge; some may later move to archive

---

## 9) `.docs/mockups/` and `.docs/fixtures/` — preserve selectively, archive aggressively

### Recommendation
**Keep only what still supports current product/design work; archive the rest.**

### Why
- `.docs/mockups/_archive/` already signals good hygiene.
- The large number of HTML explorations is normal for design iteration, but they should not clutter the active doc surface.
- `meeting-record.html` may still be useful if tied to active UI reference.

### Action
- Decide whether any active mockups are canonical references.
- Everything else should live under archive with clear “historical exploration” labeling.

**Bucket:** Mixed — preserve selectively; much is likely stale / archival

---

## 10) Top-level `docs/` — preserve; this is public-site material, not cleanup priority

### Recommendation
**Preserve.**

### Why
- This looks like the static website/docs site, not internal planning clutter.
- Only one markdown file lives there; most files are web assets.

### Note
- `docs/ai-poller-token-reduction-proposal-2026-03-29.md` stands out because it is internal/proposal-like material in a public-site directory.
- It may belong in `.docs/research/` or `.docs/audits/` instead.

**Bucket:** Preserve, but inspect `docs/ai-poller-token-reduction-proposal-2026-03-29.md`

---

## 11) Top-level `design/` — preserve, but clarify purpose

### Recommendation
**Preserve, but define it as canonical high-level product/design framing or move it under `.docs/` later.**

### Why
It is small, intentional, and referenced by `README.md`.

### Cleanup candidate
- `design/CSS-DESIGN-SYSTEM-AUDIT-2026-03-21.md` probably belongs in `.docs/audits/`.

**Bucket:** Preserve but update

---

## 12) `_archive/` (top-level) — probably preserve, but decide whether it belongs in the main repo long-term

### Recommendation
**Needs human decision.**

### Why
- `_archive/dailyos/` looks like a snapshot of an older codebase/app iteration.
- It may be historically useful, but it also makes the repo feel heavier and more chaotic.
- If it is no longer used for reference, it could move to a separate archive branch/repo/tarball.

**Bucket:** Unclear / needs human decision

---

## 13) Clear clutter candidates outside `.docs/`

### Likely stale / removable after confirmation
- `.stash/bob-phase4a-changes.patch` — clearly a one-off stash artifact
- `notification-demo.html` — likely a standalone exploration/demo artifact
- `docs/ai-poller-token-reduction-proposal-2026-03-29.md` — likely misplaced rather than canonical public doc
- `design/CSS-DESIGN-SYSTEM-AUDIT-2026-03-21.md` — likely misplaced at top-level `design/`

### Unclear / needs quick check
- `index.html` at repo root is normal for the app frontend, so not a cleanup target
- `plugins/` is small and intentional, not random clutter
- `tests/` is small but worth confirming whether these Python tests still run / matter

---

## Classification Summary

## Preserve as historical / decision record
- `.docs/decisions/**`
- `CHANGELOG.md`
- selected `.docs/research/**`
- selected `.docs/audits/**`
- selected `.docs/runbooks/**`
- selected archived docs already under `.docs/_archive/**`, `.docs/architecture/_archive/**`, `.docs/mockups/_archive/**`, `.docs/plans/_archive/**`

## Preserve but update
- `.docs/architecture/**`
- `.docs/design/**`
- `.docs/RELEASE-CHECKLIST.md`
- `.docs/RELEASE-POLICY.md`
- `release-notes.md`
- top-level `design/**`
- `README.md` doc pointers / internal doc map

## Migrate to Linear and remove from repo later
- `.docs/issues/**`
- `.docs/BACKLOG.md`
- future-facing `.docs/plans/*.md`
- version-brief style planning docs currently acting as project trackers

## Likely stale / removable
- `.stash/`
- `notification-demo.html`
- misplaced proposal/audit docs in top-level/public-facing directories
- plan files with heading/name mismatch after migration/reconciliation

## Unclear / needs human decision
- top-level `_archive/`
- whether top-level `design/` should remain separate from `.docs/design/`
- whether `release-notes.md` remains a standalone artifact or gets folded elsewhere

---

## Recommended Cleanup Policy

Use this as the repo rule set going forward.

### Keep in repo if the doc is one of these
1. **Architecture / ADR / design system / runbook / audit / research**
2. **Release governance** (`CHANGELOG`, release checklist/policy)
3. **Canonical product philosophy / principles / vision**
4. **Generated or semi-generated technical reference docs**

### Do not keep in repo as the primary system of record if the doc is one of these
1. **Issue tracking**
2. **Version planning / project execution tracking**
3. **Status dashboards / backlog management**
4. **Temporary execution notes once migrated to Linear**

### Archive policy
- Historical docs that explain *why* should stay.
- Historical docs that only tracked *work in progress* should be archived temporarily, then removed once safely represented in Linear/changelog/ADRs.

### Placement policy
- **Public/high-level**: `README.md`, top-level `design/`, `docs/` website
- **Internal durable docs**: `.docs/architecture`, `.docs/design`, `.docs/decisions`, `.docs/research`, `.docs/runbooks`, `.docs/audits`
- **No new tracker docs** in `.docs/issues`, `.docs/plans`, or giant backlog markdown files once Linear migration is complete.

---

## Prioritized Action List

## Priority 1 — stop adding new planning debt
1. Declare **Linear** the canonical home for:
   - issues
   - project/version execution tracking
   - version briefs / project descriptions
2. Freeze creation of new markdown issue specs except for rare cases that should really be ADRs/research/runbooks.

## Priority 2 — migrate the biggest clutter sources
3. Migrate active `.docs/issues/*.md` into Linear.
4. Migrate active future `.docs/plans/*.md` into Linear project descriptions.
5. Freeze `.docs/BACKLOG.md`, use it only as migration source, then retire it.

## Priority 3 — clean obvious inconsistencies
6. Fix plan naming/content mismatches:
   - `.docs/plans/v2.1.0.md`
   - `.docs/plans/v2.2.0.md`
7. Split shipped plans from future plans more clearly during migration.
8. Move misplaced audit/proposal docs into better homes.

## Priority 4 — tighten internal doc IA
9. Clarify the purpose of:
   - `.docs/design/` vs `design/`
   - `release-notes.md` vs `CHANGELOG.md`
   - `_archive/` vs `.docs/_archive/`
10. Refresh drift-prone architecture/design reference docs against the current `v1.1.1+` codebase.

## Priority 5 — final residue cleanup
11. Remove one-off artifacts after confirmation:
   - `.stash/`
   - `notification-demo.html`
   - any migrated issue/plan markdown no longer needed

---

## Files / Directories to Inspect First

Start here for maximum cleanup payoff:

1. `.docs/issues/`
   - biggest clutter source
   - direct migration target to Linear

2. `.docs/plans/`
   - contains future project briefs plus shipped history plus naming drift

3. `.docs/BACKLOG.md`
   - likely the most stale and highest-maintenance planning artifact

4. `.docs/architecture/README.md` + drift-prone architecture docs
   - preserve, but refresh where needed to match current release

5. `.docs/design/` and top-level `design/`
   - decide information architecture split

6. `.stash/`
   - trivial cleanup candidate

7. `notification-demo.html`
   - likely trivial cleanup candidate

8. `docs/ai-poller-token-reduction-proposal-2026-03-29.md`
   - likely misplaced

9. top-level `_archive/`
   - confirm whether it still earns its keep in the repo

---

## Suggested Migration Rules for Linear

## Rule 1: Issues
For each `.docs/issues/iXXX.md`:
- create/update Linear issue with:
  - title
  - problem statement
  - scope
  - acceptance criteria
  - relevant technical references
  - repo links / ADR links
- then classify the markdown source:
  - if purely execution-tracking: remove later
  - if it contains durable design/architecture reasoning: convert that part into ADR/research/runbook first

## Rule 2: Version briefs / project briefs
For each active `.docs/plans/*.md`:
- create/update a Linear project with:
  - thesis / objective
  - scope in/out
  - milestones or phases
  - dependencies
  - ship criteria
- keep only a lightweight repo artifact if needed, e.g. a tiny index that points to Linear

## Rule 3: Shipped versions
For shipped versions:
- canonical historical record should be:
  - git tags
  - `CHANGELOG.md`
  - optionally `release-notes.md`
- full working briefs do not need to remain active docs unless they capture uniquely valuable rationale

## Rule 4: Durable knowledge extraction
Before deleting any issue/plan doc, ask:
- does this contain an architectural decision?
- does it contain a runbook?
- does it explain a production incident or important tradeoff?
- does it document design principles not recorded elsewhere?

If yes, extract that knowledge first.

## Rule 5: Linkback strategy
If desired, preserve traceability by adding one short footer/note in Linear or changelog entries linking back to the commit/tag/ADR instead of keeping full duplicate markdown trackers.

---

## Proposed `.docs/` Structure After Cleanup

A cleaner target shape:

```text
.docs/
  audits/
  architecture/
    _archive/
  decisions/
  design/
  research/
    _archive/
  runbooks/
  references/
    contracts/
    fixtures/
    generators/
  mockups/
    _archive/
  _archive/
```

### Notes on this structure
- `issues/` disappears after migration to Linear
- `plans/` disappears or becomes a very small index file pointing to Linear projects
- `contracts/`, `fixtures/`, and `generators/` could either stay top-level under `.docs/` or be grouped under `references/`
- keep `_archive/` only for genuinely useful historical material, not as a graveyard for everything

---

## Practical Next Pass

If doing this in the next implementation pass, I would tackle it in this order:

1. inventory active `.docs/issues/*.md` and map each to Linear status
2. inventory `.docs/plans/*.md` and split into shipped vs future
3. retire `.docs/BACKLOG.md` as active tracker
4. normalize misplaced docs (`design` audit, public `docs` proposal, etc.)
5. refresh architecture/design docs that are intended to remain canonical
6. remove obvious one-off artifacts after human signoff

---

## Bottom Line

The repo is not suffering from “too many docs.”
It is suffering from **too many docs acting as a project tracker**.

The winning move is:
- keep **knowledge** in the repo,
- move **execution tracking** to Linear,
- simplify `.docs/` around durable internal documentation,
- then do a small residue cleanup of one-off artifacts and misplaced files.
