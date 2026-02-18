---
name: "Project Update"
content_type: status-report
best_for: "Multi-week project tracking, cross-functional initiative updates"
word_count: "400-700"
sections: 5
---

# Project Update Template

Structured update for ongoing projects or initiatives that span multiple weeks and involve cross-functional coordination.

## When to Use

- Weekly or biweekly project status updates
- Cross-functional initiative tracking
- Stakeholder alignment on multi-phase work
- Any project with milestones and dependencies

## Core Principle

**Status should inform decisions.**

A good project update doesn't just report—it helps stakeholders know if they need to act, adjust, or escalate.

## Traffic Light System

Use consistent status indicators:

| Status | Meaning | Action |
|--------|---------|--------|
| 🟢 **On Track** | Proceeding as planned | No intervention needed |
| 🟡 **At Risk** | Challenges emerging | Attention needed, not urgent |
| 🔴 **Blocked** | Cannot proceed | Immediate action required |
| ⬜ **Not Started** | Future milestone | No update yet |
| ✅ **Complete** | Milestone achieved | Done |

## Structure

### Status Header (1-2 sentences)

**Purpose**: Overall status at a glance.

**Guidance**:
- Overall traffic light status
- One sentence summary of current state
- Any urgent callouts

**Format**: "[Overall Status]: [One sentence current state]"

**Questions to ask**:
- Would a busy stakeholder know if they need to read further?
- Is the overall status accurate?
- Are urgent issues immediately visible?

### Progress Since Last Update (3-4 bullets)

**Purpose**: What was accomplished.

**Guidance**:
- Completed milestones and deliverables
- Key decisions made
- Blockers resolved
- Forward progress demonstrated

**Questions to ask**:
- Is progress concrete, not vague?
- Did we do what we said we'd do?
- Are completions clearly marked?

### Current Blockers or Risks (2-3 bullets if any)

**Purpose**: What's impeding progress.

**Guidance**:
- Be specific about the blocker
- Identify who/what can unblock it
- Quantify impact if possible
- Don't hide problems

**Blocker format**:
- **Blocker**: [What's stuck]
- **Impact**: [What this affects]
- **Resolution path**: [How to fix it]
- **Owner**: [Who needs to act]

**Questions to ask**:
- Am I being honest about impediments?
- Is the path to resolution clear?
- Have I identified the right owner?

### Next Steps (3-4 bullets)

**Purpose**: What's coming in the next cycle.

**Guidance**:
- Specific deliverables or milestones
- Named owners
- Target dates
- Clear enough to track

**Questions to ask**:
- Are next steps specific and actionable?
- Are owners named?
- Are dates realistic?

### Timeline Check (brief)

**Purpose**: Are we on track to overall deadline?

**Guidance**:
- Original timeline vs. current projection
- Any adjustments needed
- Key upcoming milestones

**Questions to ask**:
- Is timeline reality reflected?
- Are stakeholders aware of any shifts?
- Are key dates visible?

---

## Anti-Patterns

- **Vague progress** - "Making good progress" vs. specific completions
- **Hidden blockers** - Problems buried or softened
- **Missing owners** - Next steps without accountability
- **Status drift** - 🟢 when it should be 🟡
- **No timeline view** - Losing sight of end goal

## Template

```markdown
# [Project Name] Update - [Date]

## Status: [🟢/🟡/🔴] [One sentence summary]

---

### Progress Since Last Update
- ✅ [Completed item 1]
- ✅ [Completed item 2]
- 🔄 [In progress item with percentage or status]
- ⬜ [Not yet started, as expected]

### Blockers / Risks
| Issue | Impact | Resolution | Owner |
|-------|--------|------------|-------|
| [Blocker 1] | [What it affects] | [How to fix] | [Name] |
| [Risk 1] | [Potential impact] | [Mitigation] | [Name] |

*[None currently]* if no blockers

### Next Steps (Next [Timeframe])
- [ ] [Action] - **[Owner]** - [Target date]
- [ ] [Action] - **[Owner]** - [Target date]
- [ ] [Action] - **[Owner]** - [Target date]

### Timeline
- **Original target**: [Date]
- **Current projection**: [Date] [On track / Adjusted because X]
- **Key upcoming milestone**: [Milestone] by [Date]

---

*Next update: [Date]*
```

---

## Example

```markdown
# Agentforce WordPress Connector - Update - Jan 10, 2026

## Status: 🟢 On Track
MVP feature-complete. Now in testing phase with Cox POC environment validation underway.

---

### Progress Since Last Update
- ✅ Core connector MVP deployed to staging
- ✅ Cox POC environment configured and validated
- ✅ Documentation draft completed (technical + user guides)
- 🔄 Integration testing with Salesforce team (60% complete)

### Blockers / Risks
| Issue | Impact | Resolution | Owner |
|-------|--------|------------|-------|
| Salesforce API rate limit question | May affect scale scenarios | Meeting scheduled Jan 12 | Abdul |

### Next Steps (Next 2 Weeks)
- [ ] Complete integration testing - **Abdul** - Jan 15
- [ ] Cox pilot dealer selection (50 dealers) - **James** - Jan 17
- [ ] Partner documentation review with Salesforce - **Riley** - Jan 20
- [ ] Production environment setup - **Abdul** - Jan 22

### Timeline
- **Original target**: Feb 1 pilot launch
- **Current projection**: Feb 1 (On track)
- **Key upcoming milestone**: Pilot launch with Cox by Feb 1

---

*Next update: Jan 17, 2026*
```
