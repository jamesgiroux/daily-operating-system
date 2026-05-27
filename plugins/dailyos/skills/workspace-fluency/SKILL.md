---
name: workspace-fluency
description: "Foundational knowledge of DailyOS workspace structure, schemas, and conventions"
---

# Workspace Fluency

This skill is always active when operating inside a DailyOS workspace. It provides foundational knowledge of the directory structure, file formats, schemas, and conventions that every other skill and command depends on.

## Workspace Detection

A DailyOS workspace is identified by the presence of these directories at the workspace root:

- `data/` — Structured data files (JSON schemas)
- `Accounts/` and/or `Projects/` — Entity directories
- `People/` — Relationship profiles
- `_archive/` — Historical meeting summaries
- `_inbox/` — Raw inputs awaiting processing

If these directories exist, you are in a DailyOS workspace. Load this skill automatically.

## Directory Structure

```
workspace-root/
  Accounts/
    {entity-name}/
      dashboard.json        # DailyOS-managed export projection
      intelligence.json     # DailyOS-managed intelligence export projection
      stakeholders.md       # Champion, exec sponsor, buyer, influencer roles
      actions.md            # Entity-specific action items
  Projects/
    {entity-name}/
      dashboard.json        # DailyOS-managed export projection
      intelligence.json     # DailyOS-managed intelligence export projection
  People/
    {person-name}/
      person.json           # DailyOS-managed export projection
      person.md             # DailyOS-managed relationship export projection
  _archive/
    YYYY-MM/                # Monthly directories
      {meeting-summary}.md  # Processed meeting outputs
  _inbox/                   # Raw inputs: transcripts, notes, documents
  data/
    schedule.json           # Derived calendar export
    prep.json               # Derived meeting prep export
    actions.json            # Derived action export
    emails.json             # Derived email export
    manifest.json           # Workspace configuration and role preset
```

## Authority Model

DailyOS SQLite, services, and MCP/runtime tools are the source of truth for current intelligence. Generated JSON and markdown files are portability exports. Use them only when runtime tools are unavailable, when performing an explicit import/backfill/recovery workflow, or when the user asks to inspect file artifacts.

User-authored transcripts, notes, documents, and meeting outputs are source material and may be read as evidence. Generated exports should not be read back as authority for runtime answers, prompt inputs, or mutations.

## Entity Model

The term "entity" refers to an Account, Project, Person, Meeting, Action, or another DailyOS subject kind exposed by runtime tools. Account and project workspaces live in their respective top-level directories and may include generated export files. When a command asks for an entity, resolve it through DailyOS runtime/MCP first; use directory search only as a fallback locator.

### Entity Resolution

To find an entity by name:

1. List directories under `Accounts/` and `Projects/`
2. Match by directory name (case-insensitive, partial match acceptable)
3. If ambiguous, list candidates and ask the user to clarify
4. Once resolved, the entity path becomes the working context for all reads

### Entity Files

**dashboard.json** — Generated quantitative export:
- `arr` or `revenue` — Financial value
- `health` — Green/Yellow/Red status
- `renewal_date` or `end_date` — Key date
- `lifecycle` — Stage (onboarding, growth, mature, renewal, at-risk)
- `owner` — Internal owner name
- `tier` — Priority tier

**intelligence.json** — Generated qualitative export:
- `executive_assessment` — Current narrative summary
- `risks` — Array of identified risk factors with evidence
- `wins` — Array of recent wins and positive signals
- `current_state` — What is happening right now
- `stakeholder_insights` — Relationship dynamics and power structures
- `last_updated` — Staleness indicator

**stakeholders.md** — Markdown document or generated export mapping people to roles:
- Champion, Executive Sponsor, Economic Buyer, Technical Buyer
- Influence level, engagement frequency, sentiment signals

**actions.md** — Entity-specific action export in markdown format

These files mirror or project runtime data. They are not the primary authority when runtime tools are available.

## People Files

**person.json** — Structured profile:
- `name`, `role`, `title`, `organization`
- `classification` — Internal, External, Partner
- `relationship_type` — Champion, Stakeholder, Executive, Peer, Report

**person.md** — Narrative relationship intelligence:
- Meeting history signals (frequency, topics, tone)
- Temperature model: warming, stable, cooling (with evidence)
- Last seen date and context
- Communication preferences (format, frequency, formality level)
- Relationship arc narrative

## The 5 DailyOS Schemas

### schedule.schema.json
Today's calendar events. Key fields:
- `id`, `title`, `start_time`, `end_time`
- `attendees` — Array of participant objects with name and email
- `meeting_type` — Maps to one of 8 meeting templates
- `entity` — Associated entity name (if detected)
- `prep_status` — Whether meeting prep has been generated

### prep.schema.json
Meeting preparation data. Key fields:
- `meeting_id` — Links to schedule entry
- `template_type` — One of 8 meeting templates
- `attendee_context` — Loaded people profiles for attendees
- `entity_context` — Entity dashboard and intelligence snapshot
- `open_actions` — Relevant pending actions
- `talking_points` — Generated discussion items
- `risks_to_address` — Items requiring attention in meeting

### actions.schema.json
Tracked action items. Key fields:
- `id` — Unique identifier
- `text` — Action description
- `entity` — Associated entity name
- `person` — Associated person (owner or assignee)
- `due_date` — When it is due (ISO date)
- `status` — open, in_progress, completed, overdue
- `source_meeting` — Meeting where action was created
- `created_at` — When action was captured

### emails.schema.json
Email signals from Gmail. Key fields:
- `id`, `subject`, `from`, `to`, `date`
- `entity` — Associated entity (if detected)
- `signal_type` — categorization (escalation, request, update, positive, negative)
- `summary` — AI-generated summary
- `requires_action` — Boolean flag

### manifest.schema.json
Workspace configuration. Key fields:
- `workspace_name` — Display name
- `role_preset` — Active role preset identifier
- `entity_mode` — "accounts", "projects", or "both"
- `owner` — Workspace owner name
- `created_at`, `last_sync` — Timestamps

## Role Preset System

The `manifest.json` file specifies the active role preset. The preset is a vocabulary layer that shapes how all skills and commands produce output. It does not change architecture — it changes language.

A role preset defines:
- **entityNoun** — What entities are called (accounts, clients, deals, projects)
- **healthFrame** — How health is described (retention risk, deal velocity, project status)
- **riskVocabulary** — Terms for negative signals
- **winVocabulary** — Terms for positive signals
- **urgencySignals** — What triggers immediate attention

Read the active preset from `manifest.json` and apply its vocabulary to all output. The role-vocabulary skill provides detailed mappings for each preset.

## Loop-Back Convention

After producing any deliverable, always offer to write results back to the workspace. This is the loop-back convention:

- **Offer, never force.** Present what could be saved and where, then let the user confirm.
- **Be specific.** Not "Want me to save this?" but "Would you like me to save this risk report as a markdown artifact and create 3 DailyOS actions?"
- **Route correctly.** Durable intelligence updates go through DailyOS runtime/services. Entity reports may be saved as artifacts. People insights go through person intelligence tools when available. Meeting outputs go to `_archive/YYYY-MM/`. Actions go through DailyOS action tools when available.

The loop-back skill provides detailed routing rules. Every command includes loop-back instructions in its workflow.

## Conventions

- **Workspace, not project folder.** Always refer to the DailyOS directory as a "workspace."
- **Entity, not account.** Use "entity" as the generic term. Use "account" or "project" only when the distinction matters.
- **Evidence-backed.** Every assertion in output should trace to a specific file, meeting, signal, or data point in the workspace.
- **Staleness awareness.** Check `last_updated` fields. If intelligence is older than 2 weeks, note it. If dashboard data is older than a month, flag it as potentially stale.
