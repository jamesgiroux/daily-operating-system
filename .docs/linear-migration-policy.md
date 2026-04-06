# Linear Migration Policy

_Date: 2026-04-05_

## Purpose

This repo is moving operational tracking out of markdown and into Linear.

The goal is to keep git focused on durable knowledge while using Linear for live execution tracking.

## Linear is canonical for

- issues
- project execution tracking
- version / release / project briefs
- backlog management
- status tracking

## Git is canonical for

- architecture docs
- ADRs / decisions
- design system and implementation guidance
- research
- audits
- runbooks
- release history (`CHANGELOG.md`)
- high-level product/design framing

## Default rule

If a markdown file is primarily tracking work rather than preserving knowledge, it should probably live in Linear instead.

## Issue docs

- `.docs/issues/**` is now considered legacy migration material.
- New issue tracking should happen in Linear.
- Before deleting an issue doc, extract any durable knowledge into the right permanent home.

## Plan / version brief docs

- `.docs/plans/**` should no longer be the primary home for live project/version tracking.
- Active version briefs should move to Linear project descriptions.
- Shipped plans may be archived if they add historical value.

## Backlog docs

- `.docs/BACKLOG.md` is migration source only, not the future canonical backlog.

## Durable knowledge extraction rule

Before removing any tracker-style markdown doc, check whether it contains:
- an architectural decision
- a runbook or operational procedure
- an important product/design rationale
- a postmortem or notable tradeoff

If yes, extract that material before retiring the tracker doc.

## Practical outcome

Over time:
- `.docs/issues/` should disappear or shrink to a tiny migration note
- `.docs/plans/` should disappear or shrink to an index that points to Linear
- `.docs/` should become a cleaner internal knowledge base rather than a second project manager
