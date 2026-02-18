---
name: role-vocabulary
description: "Adapts all output vocabulary based on the active role preset — 8 presets with distinct terminology"
---

# Role Vocabulary

This skill fires based on the active role preset defined in `data/manifest.json`. It shapes the vocabulary used across all skills and commands without changing the underlying architecture. The same entity-intelligence skill reads the same files, but a Customer Success preset describes "churn risk" while a Sales preset describes "deal stall."

## Activation

Always active when a DailyOS workspace is loaded. Read the `role_preset` field from `data/manifest.json` and apply the corresponding vocabulary throughout the session.

## Vocabulary Dimensions

Each preset defines these vocabulary dimensions:

- **entityNoun** — What entities are called in output
- **healthFrame** — How entity health is described and measured
- **riskVocabulary** — Terms used for negative signals, threats, problems
- **winVocabulary** — Terms used for positive signals, successes, progress
- **urgencySignals** — What triggers immediate attention in this role

## The 8 Role Presets

### 1. Customer Success

```
entityNoun: "accounts" / "customers"
healthFrame: Retention health — Green/Yellow/Red based on churn risk
riskVocabulary:
  - "churn risk" — customer may not renew
  - "health declining" — engagement or satisfaction dropping
  - "escalation" — customer raised issue above normal channels
  - "champion loss" — key advocate leaving or disengaging
  - "adoption gap" — product usage below expected levels
winVocabulary:
  - "expansion signal" — customer showing signs of growing usage
  - "advocacy" — customer willing to be reference or case study
  - "value realization" — customer achieving stated goals
  - "health improving" — positive trajectory on key metrics
urgencySignals: Renewal within 90 days + Yellow/Red health, champion departure, executive escalation
```

**How it shapes commands:**
- assess produces health assessments and renewal risk reports
- produce generates QBR narratives, success plans, business reviews
- compose uses customer-appropriate tone, references shared value
- decide frames decisions around retention vs. expansion investment

### 2. Sales

```
entityNoun: "deals" / "opportunities" / "accounts"
healthFrame: Deal velocity — pipeline stage progression and momentum
riskVocabulary:
  - "deal stall" — opportunity stopped progressing
  - "competitor threat" — alternative solution gaining traction
  - "budget risk" — funding uncertain or being reallocated
  - "champion weakening" — internal advocate losing influence
  - "timeline slip" — expected close date moving out
winVocabulary:
  - "deal acceleration" — faster-than-expected progression
  - "multi-thread" — engaged multiple stakeholders
  - "executive alignment" — C-level engaged and supportive
  - "budget confirmed" — funding secured
urgencySignals: Deal in late stage with stall signals, competitor entering, budget cycle ending
```

**How it shapes commands:**
- assess produces deal reviews and pipeline analysis
- produce generates proposals, business cases, competitive positioning
- compose uses persuasion-aware tone, references business outcomes
- decide frames decisions around win probability and deal strategy

### 3. Partnerships

```
entityNoun: "partners" / "alliances"
healthFrame: Partnership health — mutual value delivery and engagement
riskVocabulary:
  - "misalignment" — strategic priorities diverging
  - "underperformance" — partner not delivering on commitments
  - "competing priorities" — partner resources allocated elsewhere
  - "trust erosion" — reliability declining
winVocabulary:
  - "joint win" — successful co-delivery or co-sell
  - "deepening" — expanding scope of partnership
  - "strategic alignment" — priorities converging
  - "executive sponsorship" — senior leader actively championing
urgencySignals: Partner agreement renewal, major joint deliverable approaching, misalignment detected
```

### 4. Agency

```
entityNoun: "clients" / "engagements"
healthFrame: Client satisfaction — delivery quality and relationship health
riskVocabulary:
  - "scope creep" — work expanding beyond agreement
  - "satisfaction drop" — client happiness declining
  - "payment risk" — invoicing issues or delayed payment
  - "burnout signal" — team overextended on this client
  - "relationship erosion" — point of contact disengaging
winVocabulary:
  - "upsell opportunity" — client needs expanding
  - "referral signal" — client recommending your agency
  - "retainer growth" — engagement expanding
  - "showcase work" — deliverable worth featuring
urgencySignals: Client satisfaction issue raised, contract renewal approaching, scope dispute
```

### 5. Consulting

```
entityNoun: "engagements" / "clients"
healthFrame: Engagement health — delivery milestones and stakeholder satisfaction
riskVocabulary:
  - "scope risk" — deliverables unclear or expanding
  - "stakeholder misalignment" — different stakeholders want different things
  - "delivery risk" — timeline or quality at risk
  - "political headwind" — internal resistance to recommendations
winVocabulary:
  - "impact delivered" — measurable outcome achieved
  - "follow-on signal" — client interested in additional work
  - "executive buy-in" — recommendations accepted at senior level
  - "framework adoption" — client using your methodology
urgencySignals: Major deliverable due, stakeholder conflict surfacing, engagement extension decision
```

### 6. Product

```
entityNoun: "products" / "features" / "initiatives"
healthFrame: Product health — user adoption, satisfaction, and strategic fit
riskVocabulary:
  - "adoption stall" — users not engaging with feature
  - "churn driver" — feature gap causing customer loss
  - "technical debt" — accumulated shortcuts affecting velocity
  - "market shift" — competitive landscape changing
winVocabulary:
  - "adoption surge" — rapid user engagement
  - "retention driver" — feature keeping users on platform
  - "market fit signal" — product resonating with target segment
  - "velocity gain" — team shipping faster
urgencySignals: Major release approaching, adoption metrics declining, competitive launch detected
```

### 7. Leadership

```
entityNoun: "teams" / "initiatives" / "business units"
healthFrame: Organizational health — team performance, strategic execution, morale
riskVocabulary:
  - "execution risk" — strategy not translating to results
  - "talent risk" — key people at risk of leaving
  - "alignment gap" — teams pulling in different directions
  - "culture signal" — behaviors diverging from values
winVocabulary:
  - "momentum" — positive trajectory across metrics
  - "alignment" — teams executing in coordination
  - "talent win" — successful hire or retention of key person
  - "strategic progress" — measurable advancement of key initiative
urgencySignals: Board meeting approaching, quarterly results, key hire/departure, strategic pivot decision
```

### 8. The Desk

```
entityNoun: "entities" (generic — adapts to context)
healthFrame: Flexible — adapts to the entity type being discussed
riskVocabulary: Draws from all presets based on context
winVocabulary: Draws from all presets based on context
urgencySignals: Deadline proximity, relationship temperature drops, pattern breaks
```

The Desk is the generalist preset. It does not impose a specific vocabulary but reads context clues to select appropriate framing. When an entity looks like a customer account, it uses CS vocabulary. When it looks like a deal, it shifts to Sales vocabulary. This preset is for users whose role spans multiple functions.

## How Vocabulary Shapes Output

### assess command
Uses `healthFrame` and `riskVocabulary` to frame the assessment. A CS assess says "churn risk factors." A Sales assess says "deal stall indicators." Same underlying data, different framing.

### produce command
Uses `entityNoun` and deliverable types natural to the role. CS produces QBR narratives. Sales produces deal memos. Agency produces client reports.

### compose command
Uses tone calibration appropriate to the role's relationships. CS writes with partnership tone. Sales writes with value-proposition tone. Leadership writes with strategic clarity.

### decide command
Uses decision types natural to the role. CS decides on intervention strategies. Sales decides on deal tactics. Product decides on build/buy/partner.

### plan command
Uses planning horizons natural to the role. CS plans around renewal cycles. Sales plans around deal stages. Consulting plans around engagement milestones.

## Interaction with Other Skills

- **workspace-fluency** reads the manifest to determine which preset is active
- **entity-intelligence** provides the raw data that vocabulary shapes
- **relationship-context** provides people data that vocabulary adapts for role context
- **political-intelligence** uses role-specific power structures
- **analytical-frameworks** frames analysis using role-relevant decision types
- **action-awareness** uses urgencySignals to determine what surfaces as high-priority
