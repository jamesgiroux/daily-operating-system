---
description: "Deepen entity intelligence with research and web sources"
---

# /enrich

Deepen an entity's intelligence by identifying data gaps, conducting web research, and updating workspace artifacts with fresh insights. This command bridges the gap between what the workspace knows and what is publicly available.

## Arguments

- `$ARGUMENTS[0]` — Entity name (required). Resolved against Accounts/ and Projects/.

## Workflow

### Step 1: Read Current Intelligence

Resolve the entity and load all existing workspace data (entity-intelligence skill auto-fires):
- `dashboard.json` — Current vitals
- `intelligence.json` — Current assessment, risks, wins
- `stakeholders.md` — Current stakeholder map
- Recent `_archive/` meeting summaries
- People/ profiles for known stakeholders

### Step 2: Identify Gaps

Compare what the workspace has against what a complete intelligence picture requires:

**Staleness check:**
- When was `intelligence.json` last updated? Flag if older than 14 days.
- When was `dashboard.json` last refreshed? Flag if metrics are from last quarter.
- When was the last meeting with this entity? Flag if more than 30 days.

**Completeness check:**
- Are all stakeholder roles filled in `stakeholders.md`? (Champion, exec sponsor, economic buyer, technical buyer)
- Do all listed stakeholders have People/ profiles?
- Is there competitive intelligence? Is it current?
- Are there gaps in the risk or win arrays that recent events might fill?
- Is the executive assessment narrative still accurate given recent developments?

**Currency check:**
- Have there been recent news events (earnings, leadership changes, acquisitions, product launches) that the workspace does not reflect?
- Has the competitive landscape shifted?
- Are there regulatory or market changes relevant to this entity?

Present the gap assessment:

```
Intelligence gaps for Nielsen:

- Last intelligence update: January 3 (45 days ago)
- Missing stakeholder: no Technical Buyer identified
- David Park (CTO) has no People/ profile
- No competitive intelligence on record
- Q4 earnings not reflected in dashboard
- No recent news check since December
```

### Step 3: Conduct Research

Use web search to fill identified gaps. Research targets:

**Company news:**
- Recent earnings reports, financial performance
- Leadership changes (new hires, departures, promotions)
- Strategic announcements (acquisitions, partnerships, product launches)
- Press coverage and analyst commentary

**Executive/stakeholder updates:**
- Role changes for known stakeholders
- New executives who should be on the stakeholder map
- LinkedIn-level professional context for key contacts

**Competitive landscape:**
- Recent moves by competitors mentioned in workspace data
- New entrants or alternative solutions gaining traction
- Market positioning shifts

**Industry context:**
- Regulatory changes affecting this entity
- Market trends relevant to their business
- Technology shifts impacting their strategy

### Step 4: Synthesize Findings

Organize research findings into workspace-relevant categories:

```markdown
## Enrichment Report: Nielsen

### Company Updates
- Q4 2025 earnings: Revenue up 8% YoY, beat analyst estimates by 3%
- New Chief Data Officer hired in January (Maya Patel, ex-Snowflake)
- Announced strategic pivot toward AI-powered analytics in January investor call

### Stakeholder Updates
- David Park (CTO) spoke at DataConf in February — his keynote focused on "reducing vendor sprawl"
  - Signal: This aligns with our integration concerns. He may be building a case for consolidation.
- New CDO Maya Patel — likely becomes relevant stakeholder for our data platform work

### Competitive Intelligence
- Datadog launched new analytics module in January (relevant to the competitive evaluation mentioned in Feb 14 meeting)
- Snowflake announced partnership with a consulting firm Nielsen uses — could create alternate path

### Strategic Context
- Nielsen's AI analytics pivot aligns with our platform capabilities — potential expansion vector
- "Vendor sprawl reduction" theme from CTO keynote is a risk if we are seen as an additional vendor rather than a consolidation play

### Recommended Intelligence Updates
1. Update intelligence.json executive_assessment to reflect Q4 earnings and AI pivot
2. Add competitive risk: Datadog analytics module + Snowflake consulting partnership
3. Add opportunity: AI analytics pivot alignment with our platform
4. Create People/ profile for Maya Patel (new CDO)
5. Update David Park's profile with vendor sprawl concerns
6. Update dashboard.json with Q4 revenue figures if available
```

### Step 5: Present for Confirmation

Present the enrichment report and proposed workspace updates:

```
Ready to update Nielsen workspace:

1. Update Accounts/Nielsen/intelligence.json:
   - Revised executive assessment reflecting Q4 performance and AI pivot
   - New risk: competitive evaluation (Datadog, Snowflake)
   - New opportunity: AI analytics alignment
   - Updated last_updated timestamp

2. Update Accounts/Nielsen/stakeholders.md:
   - Add Maya Patel (CDO) as emerging stakeholder
   - Note David Park's vendor consolidation stance

3. Create People/Maya-Patel/:
   - person.json with role, organization, classification
   - person.md with initial context from research

4. Update People/David-Park/person.md:
   - Add vendor sprawl keynote signal
   - Note cooling engagement pattern with new strategic context

Proceed with all, or adjust first?
```

### Step 6: Execute Updates

After confirmation:
- Read each target file before modifying
- Merge new intelligence with existing (append to arrays, update narratives)
- Create new People/ profiles if needed
- Update all `last_updated` timestamps
- Report completion: "Enrichment complete. Intelligence refreshed, 1 new stakeholder profiled, competitive landscape updated."

## Loop-Back

This command's primary output is workspace updates. The confirmation step (Step 5) is the loop-back gate. Additionally, if enrichment reveals actionable items, offer to create actions:

```
The vendor sprawl concern and competitive evaluation suggest you should proactively address Nielsen's consolidation narrative before renewal. Want me to create an action: "Prepare consolidation value narrative for Nielsen renewal conversation" — due by March 1?
```

## Skills That Contribute

- **entity-intelligence** — Provides the baseline data to compare against
- **relationship-context** — Informs stakeholder research targets
- **workspace-fluency** — Guides where enrichment data should be stored
- **action-awareness** — Connects enrichment findings to actionable next steps
- **role-vocabulary** — Shapes how enrichment findings are framed
- **loop-back** — Manages the write-back confirmation workflow
