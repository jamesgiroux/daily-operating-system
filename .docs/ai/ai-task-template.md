# DailyOS AI Task Template

Use this template when handing work to Copilot, Claude, Jay, or another coding agent.

The goal is simple:
- use stronger models for **thinking and edge cases**
- use cheaper/faster agents for **implementation volume**
- avoid review loops where the smart model only plays cleanup crew

---

## How to use this

### For Claude / Jay / stronger model
Ask for:
- solution shape
- tradeoffs
- edge cases
- acceptance criteria
- traps / what not to do

### For Copilot / implementation-first agent
Ask for:
- implementation of the chosen approach
- explicit assumptions
- exact files changed
- exact validation commands run
- anything still uncertain

### Escalation rule
- first miss: clarify
- second miss: reassign or redesign
- do not keep burning cycles on review ping-pong

---

## Copy/paste template

```md
# Task
[What needs to be built or changed?]

# Why
[What user/product/problem does this solve?]

# Repo context
- Area(s): [frontend/backend/schema/tests/docs]
- Relevant files:
  - [path]
  - [path]
- Related issue/spec/PR:
  - [link or ID]

# Constraints
- Preserve existing behavior for: [x]
- Do not change: [x]
- Must work with: [x]
- Keep scope limited to: [x]

# Acceptance criteria
- [ ] [criterion 1]
- [ ] [criterion 2]
- [ ] [criterion 3]

# Non-goals
- [thing explicitly out of scope]
- [thing explicitly out of scope]

# Risks / edge cases to handle
- [edge case]
- [silent failure mode]
- [data migration / backward compatibility concern]

# Preferred approach
[If known, state the desired implementation shape. If unknown, ask the agent to propose 1-2 options first.]

# Validation
Run and report the results of the relevant checks:
- [command]
- [command]
- [command]

# Output format
Return:
1. short summary of approach
2. files changed
3. assumptions made
4. validation run + results
5. open questions / residual risk
```

---

## Variants by agent

### 1) Design brief prompt
Use this with Claude / Jay when the problem is ambiguous, cross-cutting, or architectural.

```md
Use the task below to produce a design brief, not code.

I want:
- 1-2 viable implementation approaches
- recommended approach with reasoning
- likely failure modes
- exact acceptance criteria
- what to avoid
- whether this should be treated as derived state, persisted state, or event/history state

Do not implement yet unless explicitly asked.
```

### 2) Implementation prompt
Use this with Copilot after the design shape is clear.

```md
Implement the task below using the preferred approach.

Rules:
- do not redesign the solution unless a constraint forces it
- if something is ambiguous, state the assumption clearly
- preserve existing structure and fields where possible
- avoid unrelated cleanup in this PR
- run the requested validation commands
- if validation cannot be run, say exactly why

At the end, report:
- files changed
- assumptions
- validation results
- remaining uncertainty
```

### 3) Targeted verification prompt
Use this with Claude / Jay after implementation.

```md
Review this change against the task below.

Do not do a broad stylistic review.
Focus only on:
- whether the implementation matches the acceptance criteria
- silent failure modes
- schema / persistence / migration correctness
- frontend/backend contract mismatches
- whether there is a simpler or safer approach if this one is flawed

Return:
- confirmed correct
- concerns
- severity
- exact fixes needed before merge
```

---

## When to use which agent

### Use Copilot first when:
- the work is mechanical
- the pattern already exists in the repo
- it is mostly repetitive plumbing
- the architecture is already decided

### Use Claude / Jay first when:
- the problem spans backend + frontend + schema
- semantics matter more than code volume
- persistence or merge logic is involved
- scoring, ranking, or domain meaning is involved
- there are multiple valid approaches

### Switch away from Copilot when:
- it says criteria are met but obvious gaps remain
- it keeps fixing symptoms instead of the model
- the same review feedback repeats twice
- it adds unrelated cleanup or noise to avoid the hard part

---

## DailyOS house style for AI coordination

Default workflow:
1. **Think** — Claude / Jay
2. **Build** — Copilot
3. **Verify** — Jay or Claude, targeted not broad
4. **Escalate after two misses** — redesign or reassign

Short version:
- strong model for shape
- cheap model for volume
- strong model for leverage-point verification
- no endless goblin tennis
