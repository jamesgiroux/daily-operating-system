# DailyOS Repo Cleanup — Next Pass Plan

_Date: 2026-04-05_

This document turns the cleanup audit into an executable plan.

## Goal

Reduce repo clutter without losing durable knowledge.

The guiding principle:
- **keep knowledge in git**
- **move execution tracking to Linear**
- **remove or archive residue once migration is complete**

This is not a single cleanup commit. It is a staged cleanup program.

---

## North Star

After cleanup:
- Linear is the system of record for **issues**, **project execution**, and **version/project briefs**.
- The repo remains the system of record for **architecture**, **design intent**, **decisions**, **runbooks**, **audits**, **research**, and **release history**.
- `.docs/` feels like an internal knowledge base, not a second project manager.

---

## Proposed Cleanup Policy

## Keep in repo
These are durable and should stay version-controlled:

- `.docs/decisions/**`
- `.docs/architecture/**`
- `.docs/design/**`
- `.docs/runbooks/**`
- `.docs/research/**`
- `.docs/audits/**`
- `CHANGELOG.md`
- release governance/process docs
- top-level `design/PHILOSOPHY.md`, `design/PRINCIPLES.md`, `design/VISION.md`

## Move out of repo as active trackers
These should stop being maintained in markdown once migration is complete:

- `.docs/issues/**`
- `.docs/BACKLOG.md`
- most of `.docs/plans/**`
- version brief docs used as project execution trackers

## Remove or archive after confirmation
These are likely residue/artifacts:

- `.stash/`
- `notification-demo.html`
- misplaced one-off proposal/audit docs in the wrong homes
- stale tracker docs after their contents are safely represented in Linear

---

## Target `.docs/` Structure

```text
.docs/
  ai/
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

## Notes

- `issues/` should disappear after migration or collapse to a tiny historical index.
- `plans/` should disappear after migration or collapse to a small index pointing to Linear projects.
- `references/` is optional, but gives us a cleaner home if `fixtures/`, `contracts/`, or generator/reference material grows.
- `_archive/` should be used intentionally, not as a dumping ground.

---

## Phase Plan

## Phase 1 — Stop adding new planning debt

### Objective
Ensure we do not keep generating markdown execution clutter while cleanup is underway.

### Actions
1. Declare Linear canonical for:
   - issues
   - project execution tracking
   - version/project briefs
2. Stop creating new markdown issue specs unless the content is actually:
   - an ADR
   - a runbook
   - research
   - a durable technical note
3. Freeze `.docs/BACKLOG.md` as migration source only.
4. Freeze `.docs/issues/**` and `.docs/plans/**` as legacy sources during migration.

### Deliverables
- short documented policy in repo
- optional README note in `.docs/issues/` and `.docs/plans/`

---

## Phase 2 — Migrate tracker docs into Linear

### Objective
Move live execution state out of markdown and into Linear.

### 2A. Issues migration
For each active `.docs/issues/*.md` file:
- map to existing Linear issue if present
- otherwise create Linear issue
- migrate:
  - title
  - problem statement
  - scope
  - acceptance criteria
  - key implementation notes
  - dependencies/links
- classify the markdown file as one of:
  - **delete later** (execution-only)
  - **extract durable knowledge first**
  - **archive temporarily**

### 2B. Plans/version-brief migration
For each active `.docs/plans/*.md` file:
- map to Linear project
- move:
  - thesis/objective
  - scope / out-of-scope
  - milestones/phases
  - dependencies
  - shipping criteria
- keep repo copy only if it adds durable historical context beyond execution

### Deliverables
- migration table: markdown file → Linear object
- list of docs safe to retire after migration

---

## Phase 3 — Preserve durable knowledge before deletion

### Objective
Do not lose important context while removing tracker docs.

Before deleting any issue/plan doc, ask:
- Does it contain an architectural decision not captured in `.docs/decisions/`?
- Does it contain a runbook or operational procedure?
- Does it capture a design/system rationale that should live in `.docs/design/` or `.docs/architecture/`?
- Does it document a postmortem, failure mode, or important tradeoff?

### If yes
Extract and relocate the durable part into one of:
- `.docs/decisions/`
- `.docs/architecture/`
- `.docs/design/`
- `.docs/runbooks/`
- `.docs/research/`
- `.docs/audits/`

### If no
Retire the markdown tracker doc once the Linear migration is confirmed.

---

## Phase 4 — Tighten `.docs/` information architecture

### Objective
Make the remaining documentation easier to trust and navigate.

### Priority areas

#### A. `.docs/architecture/`
Preserve and refresh selective docs against the current shipped reality.

Priority refresh targets:
- `FRONTEND-COMPONENTS.md`
- `FRONTEND-STYLES.md`
- `FRONTEND-TYPES.md`
- any inventory/map docs that drift fastest

#### B. `.docs/design/` vs `design/`
Clarify split:
- top-level `design/` = canonical product/design philosophy
- `.docs/design/` = internal design system and implementation guidance

If that split is correct, document it explicitly.
If not, consolidate.

#### C. `release-notes.md` vs `CHANGELOG.md`
Define roles clearly:
- `CHANGELOG.md` = canonical release history
- `release-notes.md` = optional narrative/curated release communication

If `release-notes.md` adds little, simplify or retire it.

#### D. `plans/`
After migration, either:
- remove entirely, or
- replace with a tiny index doc that points to Linear projects and explains that plans now live there

#### E. `issues/`
After migration, either:
- remove entirely, or
- replace with a tiny README explaining the move to Linear

---

## Phase 5 — Low-risk residue cleanup

### Objective
Remove obvious clutter once the structural cleanup is underway.

### First candidates
- `.stash/bob-phase4a-changes.patch`
- `notification-demo.html`
- `docs/ai-poller-token-reduction-proposal-2026-03-29.md` → likely move to `.docs/research/` or `.docs/audits/`
- `design/CSS-DESIGN-SYSTEM-AUDIT-2026-03-21.md` → likely move to `.docs/audits/`

### Human-decision items
- top-level `_archive/`
- any active need for historical code snapshots in main repo

---

## Immediate Recommended Work Order

If we start now, this is the order I recommend:

1. **Write and commit migration policy**
   - Linear is canonical for issues/plans/project briefs
   - repo is canonical for durable knowledge

2. **Inventory `.docs/issues/` into a migration table**
   - active vs done vs archive
   - mapped vs unmapped to Linear

3. **Inventory `.docs/plans/` into a migration table**
   - shipped vs future
   - active vs stale
   - detect naming/content mismatches

4. **Create a retirement candidate list**
   - docs safe to remove after migration
   - docs needing knowledge extraction first

5. **Refresh trust-critical architecture docs**
   - especially drift-prone inventory/reference docs

6. **Do a small first cleanup commit**
   - low-risk moves/removals only
   - no giant repo-wide churn yet

---

## Concrete Decisions I Recommend Now

### Decision 1
**Do not try to “clean the whole repo” in one pass.**
That would create too much risk and too much diff noise.

### Decision 2
**Treat `.docs/issues/` and `.docs/plans/` as migration projects, not simple delete folders.**
They are the largest clutter source, but also the biggest knowledge-loss risk.

### Decision 3
**Preserve ADRs and architecture docs aggressively.**
Those are assets, not clutter.

### Decision 4
**Use small, category-based cleanup commits.**
Examples:
- policy + README notes
- misplaced doc moves
- issue migration batch 1
- plan migration batch 1
- architecture doc refresh batch 1
- residue cleanup batch 1

---

## Suggested Next Implementation Passes

### Pass A — Policy + indexes
Create small repo docs that state:
- issues now live in Linear
- project/version briefs now live in Linear
- what stays in `.docs/`

### Pass B — Issues migration inventory
Generate an inventory file for `.docs/issues/**` with status and migration destination.

### Pass C — Plans migration inventory
Generate an inventory file for `.docs/plans/**` with shipped/future/migrate/archive classification.

### Pass D — Misplaced file normalization
Move obvious docs into the right homes.

### Pass E — Architecture/design refresh
Update trust-critical docs against the current shipped codebase.

---

## What “Current” Should Mean

When we say `.docs/` should be current, that should mean:

- architecture and design docs reflect the latest shipped reality or explicitly say where they drift
- no active issue tracker exists in markdown
- no active version brief system exists in markdown
- historical docs are intentionally archived rather than ambiguously lingering
- internal docs are organized by knowledge type, not by project chaos

---

## Bottom Line

The repo does not need less thinking.
It needs less **markdown pretending to be a project management system**.

The right next pass is:
1. set policy,
2. migrate trackers to Linear,
3. preserve durable knowledge,
4. clean residue,
5. refresh the docs that remain canonical.
