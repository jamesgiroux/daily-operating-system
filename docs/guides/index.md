# How It Works

Technical documentation for the Daily Operating System.

## Quick Reference

| Command | When | What it Does | Documentation |
|---------|------|--------------|---------------|
| `/today` | Every morning | Prep meetings, surface actions, triage email | [Daily Workflow](daily-workflow.md) |
| `/wrap` | End of day | Reconcile actions, capture impacts, archive | [Daily Workflow](daily-workflow.md) |
| `/email-scan` | Inbox zero blocks | Triage email, draft responses, archive noise | [Daily Workflow](daily-workflow.md) |
| `/week` | Monday morning | Week overview, hygiene alerts, impact template | [Weekly Workflow](weekly-workflow.md) |
| `/month` | First Monday | Aggregate weekly impacts, generate report | [Monthly/Quarterly](monthly-quarterly-workflow.md) |
| `/quarter` | Last week of Q | Pre-populate review with evidence | [Monthly/Quarterly](monthly-quarterly-workflow.md) |

---

## System Architecture Overview

```mermaid
flowchart TB
    subgraph Daily["Daily Rhythm"]
        TODAY["/today<br/>Morning prep"]
        WRAP["/wrap<br/>End of day"]
        EMAIL["/email-scan<br/>Inbox triage"]
    end

    subgraph Weekly["Weekly Rhythm"]
        WEEK["/week<br/>Monday planning"]
        IMPACT[Weekly Impact<br/>Capture]
    end

    subgraph Monthly["Monthly Rhythm"]
        MONTH["/month<br/>Report generation"]
    end

    subgraph Quarterly["Quarterly Rhythm"]
        QUARTER["/quarter<br/>Review prep"]
    end

    subgraph Supporting["Supporting Systems"]
        INBOX["/inbox<br/>Document triage"]
    end

    TODAY --> WRAP
    WRAP --> |"Daily impacts"| IMPACT
    WEEK --> IMPACT
    IMPACT --> MONTH
    MONTH --> QUARTER

    TODAY --> |"Inbox auto-processed"| INBOX
    WRAP --> |"Transcripts"| INBOX
```

---

## Command Cascade

The commands form a cascade where outputs flow upward:

```mermaid
flowchart BT
    subgraph L1["Level 1: Daily"]
        T1["/today outputs"]
        T2["/wrap captures"]
    end

    subgraph L2["Level 2: Weekly"]
        W1["Weekly impact template"]
        W2["Week overview"]
    end

    subgraph L3["Level 3: Monthly"]
        M1["Monthly report draft"]
    end

    subgraph L4["Level 4: Quarterly"]
        Q1["Quarterly review with evidence"]
    end

    T1 --> |"Actions, meetings"| W2
    T2 --> |"Daily impacts"| W1
    W1 --> |"Aggregated"| M1
    M1 --> |"3 months"| Q1
```

---

## Documentation Map

### Core Systems

| Document | Purpose | Key Sections |
|----------|---------|--------------|
| [Daily Workflow](daily-workflow.md) | /today, /wrap, /email-scan workflows | Three-phase execution, meeting prep, action reconciliation |
| [Weekly Workflow](weekly-workflow.md) | /week command and impact capture | Monday planning, hygiene alerts, time blocking |
| [Monthly/Quarterly](monthly-quarterly-workflow.md) | /month and /quarter workflows | Report aggregation, evidence mapping |
| [Inbox Processing](inbox.md) | Three-phase document workflow | Preparation, enrichment, delivery |

### Architecture

| Document | Purpose |
|----------|---------|
| [Skill-Agent-Tool Layers](skill-agent-tool-layers.md) | How skills, agents, and Python tools relate |
| [Three-Phase Pattern](three-phase-pattern.md) | The prepare → enrich → deliver architecture |

### Reference

| Document | Purpose |
|----------|---------|
| [Tools Reference](tools-reference.md) | Quick reference for all Python tools |

---

## Three-Phase Pattern

All major commands follow this pattern:

```mermaid
sequenceDiagram
    participant U as User
    participant P1 as Phase 1 (Python)
    participant C as Phase 2 (Claude)
    participant P3 as Phase 3 (Python)

    U->>P1: Run command
    Note over P1: Fetch data<br/>Validate state<br/>Write directive

    P1->>C: Directive ready
    Note over C: Read directive<br/>Generate content<br/>Apply intelligence

    C->>P3: AI tasks complete
    Note over P3: Write files<br/>Update state<br/>Clean up

    P3->>U: Command complete
```

**Why three phases?**

1. **Reliability**: Python handles flaky APIs with retries and timeouts
2. **Speed**: Deterministic ops complete quickly, AI focuses on judgment
3. **Debuggability**: Directive files show exactly what data was gathered
4. **Resilience**: Commands can resume from Phase 2 if Claude interrupts

---

## Getting Started

### New User Checklist

1. **Set up Google API**: Configure `.config/google/` credentials
2. **Run `/week`**: Start with Monday overview for full context
3. **Run `/today`**: Daily morning routine
4. **Run `/wrap`**: End each day to maintain data quality
5. **Process inbox**: Documents flow through system

### Recommended Daily Flow

```
Morning:
1. Run /today
2. Review 00-overview.md
3. Work through prep files
4. Address high-priority emails

Throughout day:
5. Attend meetings
6. Capture notes in _inbox/ or directly

End of day:
7. Run /wrap
8. Answer prompts for task status, impacts
9. System archives and prepares for tomorrow
```

---

## Troubleshooting

| Symptom | Likely Cause | Resolution |
|---------|--------------|------------|
| No calendar events | Google API issue | Check credentials, run API test |
| Empty directive | Script error | Check prepare script output |
| Stale task data | Missed /wrap | Run /wrap to reconcile |
| Inbox stuck | Enrichment incomplete | Complete Phase 2 manually |

### Common Debug Commands

```bash
# Check Google API
python3 .config/google/google_api.py calendar list 1

# View directive
cat _today/.today-directive.json | jq .

# Check inbox state
cat _inbox/.processing-state.json
```

---

## Related Documentation

- [Getting Started](/docs/getting-started.md) - Setup and installation
- [CLI Reference](/docs/cli-reference.md) - All available commands

---

*Documentation version: 1.0*
