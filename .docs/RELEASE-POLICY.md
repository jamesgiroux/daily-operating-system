# Release Policy

DailyOS ships on a weekly release train. This policy governs when and how releases reach users.

---

## Release Train

**Schedule:** Weekly, Tuesday late morning ET.

| Day | Activity |
|-----|----------|
| Monday noon ET | Feature cutoff — no new work merges to `main` after this |
| Monday afternoon | Acceptance testing, regression check, release notes draft, final triage |
| Tuesday morning | Release candidate validation — full checklist pass |
| Tuesday late morning | Publish if all gates green |
| Tuesday afternoon | Monitor, collect hotfix-worthy fallout |

Work completed after Monday noon cutoff rides the next train.

---

## Hotfix Policy

Off-cycle releases are reserved for:

- Broken core workflow (briefing won't load, meetings page crashes)
- Data corruption or data loss risk
- Auth/sync failure (Google, Glean disconnected with no recovery)
- Security vulnerability
- Severe customer-facing regression from the current train

**Everything else waits for Tuesday.** A cosmetic bug, a missing badge, a wrong label — these are annoying but not urgent. They ride the next train.

Hotfix process:
1. Fix on `dev`, cherry-pick to `main`
2. Bump patch version
3. Tag, push, monitor
4. Reconcile `dev` with `main` before next train

---

## Versioning

Semver. The train doesn't change the numbering — it changes the cadence.

| Level | When |
|-------|------|
| **Major** (2.0.0) | Breaking changes, major architecture shifts |
| **Minor** (1.1.0) | Meaningful user-facing feature set or theme shift |
| **Patch** (1.0.5) | Weekly train — improvements, fixes, polish batched together |
| **Patch hotfix** | Urgent fix outside the train (same patch bump) |

A Tuesday train is a patch release. A train that includes a significant new feature (like the Meeting Record) is a minor release. The decision is made at cutoff time based on what's in the train.

---

## Branch Model

- **`dev`** — active development. Merge continuously. All work lands here first.
- **`main`** — stable releases only. Merged from `dev` at train time or for hotfixes.
- **Tags** — every release gets a semver tag (`v1.0.5`). Tags trigger the CI release workflow.

No feature branches required for solo work. Feature branches recommended when parallel sessions modify the same files.

---

## Train Planning

Version briefs (`.docs/plans/vX.Y.Z.md`) serve as train manifests. Each brief defines:

- Issues in scope
- Acceptance criteria
- Dependency graph
- What's explicitly out of scope

Issues are assigned to a target train, not a standalone version. If an issue isn't ready by Monday cutoff, it moves to the next train — no partial releases.

---

## Release Notes

Two artifacts, both updated before tagging:

| File | Audience | Style |
|------|----------|-------|
| `CHANGELOG.md` | Developers | Keep a Changelog format (Added/Changed/Fixed) |
| `release-notes.md` | Users | Product marketing language, no jargon, grouped by value area |

Release notes group changes by what users care about:
- Meetings
- Accounts & Health
- Actions & Tasks
- Email & Correspondence
- Reliability & Polish

Not by engineering slices or issue numbers.

---

## Checklist

The full pre-release checklist lives in `.docs/RELEASE-CHECKLIST.md`. The train conductor (you) runs it on Monday afternoon / Tuesday morning. The checklist gates the release — if any required item fails, the train doesn't ship.

---

## What This Replaces

Before this policy, releases shipped whenever code was ready — sometimes 3-4 per day. That worked for a solo builder iterating fast. With 6-9 external users receiving update prompts, batched weekly releases are more respectful of their attention and more reliable for everyone.
