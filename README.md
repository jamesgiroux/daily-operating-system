# Daily Operating System

A productivity framework built on Claude Code for managing your daily work, strategic thinking, and professional development.

## Philosophy

**Value shows up without asking.** The system does work before you arrive.

**Skip a day, nothing breaks.** Each run rebuilds fresh—no accumulated guilt from missed days.

**Incremental improvement.** Small, compounding gains over time.

## Quick Start

```bash
# Run the setup wizard
python3 setup.py

# Or with options
python3 setup.py --workspace ~/Documents/productivity
python3 setup.py --quick  # Use defaults
python3 setup.py --verify  # Check existing installation
```

## What You Get

### Commands (7)

| Command | Purpose |
|---------|---------|
| `/today` | Morning dashboard—meeting prep, actions, email triage, look-ahead agendas |
| `/wrap` | End-of-day closure—reconcile actions, capture impacts, archive |
| `/week` | Monday review—overview, hygiene alerts, time blocking, impact template |
| `/month` | Monthly roll-up—aggregate weekly impacts into monthly report |
| `/quarter` | Quarterly prep—pre-fill review templates with evidence |
| `/email-scan` | Email triage—surface important, draft responses, archive noise |
| `/git-commit` | Atomic commits—stage, commit, push with meaningful messages |

### Skills (3)

| Skill | Purpose |
|-------|---------|
| **inbox-processing** | Three-phase document flow (preparation, enrichment, delivery) |
| **strategy-consulting** | McKinsey-style strategic analysis with multi-agent workflow |
| **editorial** | Writing review standards with multi-stage review process |

### Agents (16)

**Strategic** (from strategy-consulting):
- engagement-manager: Problem framing (SCQA + Day 1 Hypothesis)
- framework-strategist: Issue tree construction (MECE)
- partner-critic: Quality control and red team
- analyst-research-logic: Evidence gathering and validation
- executive-storyteller: Output generation (Pyramid Principle)

**Content** (from editorial):
- writer-research: Evidence gathering for content
- writer-mechanical-review: Typography, terminology linting
- writer-structural-review: Logic, flow, evidence integration
- writer-voice-review: Voice fidelity to content type
- writer-craft-review: Soul + mechanics evaluation
- writer-authenticity-review: AI-tell detection
- writer-challenger: Red-team premises and claims
- writer-scrutiny: Executive specificity check

**Inbox** (from inbox-processing):
- file-organizer: PARA routing for documents
- integration-linker: External system links

**Commands**:
- agenda-generator: Draft agendas for upcoming meetings

## Directory Structure

After setup, your workspace will look like:

```
workspace/
├── Projects/           # Active initiatives with deadlines
├── Areas/              # Ongoing responsibilities
├── Resources/          # Reference materials
├── Archive/            # Completed/inactive items
├── _inbox/             # Unprocessed documents
├── _today/             # Daily working files
│   ├── tasks/          # Persistent task tracking
│   ├── archive/        # Previous days
│   └── 90-agenda-needed/
├── _templates/         # Document templates
├── _tools/             # Python automation scripts
├── _reference/         # Standards and guidelines
├── .config/google/     # Google API credentials
├── .claude/
│   ├── commands/       # Slash commands
│   ├── skills/         # Skill packages
│   └── agents/         # Agent definitions
└── CLAUDE.md           # Claude Code configuration
```

## Google API Integration (Optional)

The system works best with Google API access for:
- **Calendar**: View meetings, create time blocks
- **Gmail**: Read emails, create drafts, manage labels
- **Sheets**: Read account data, update tracking
- **Docs**: Create and edit shared documents

Setup is guided by the wizard. You'll need a Google Cloud project with OAuth credentials.

## Daily Workflow

### Morning (`/today`)

1. Archives yesterday's files
2. Fetches today's calendar
3. Classifies meetings (customer, project, internal, personal)
4. Generates prep for customer meetings
5. Scans email for important items
6. Surfaces due/overdue actions
7. Looks ahead 3-4 days for agenda needs
8. Creates daily overview

### End of Day (`/wrap`)

1. Checks if meeting notes/transcripts processed
2. Reconciles action items (status updates)
3. Captures daily impacts (customer + personal)
4. Updates master task list
5. Archives today's files
6. Prepares for tomorrow

### Monday (`/week`)

1. Prompts for weekly priorities
2. Shows all meetings this week
3. Aggregates action items
4. Checks account hygiene
5. Pre-populates impact template
6. Suggests time blocks
7. Creates calendar events (with approval)

## Customization

### CLAUDE.md

Your `CLAUDE.md` file configures Claude Code. Key sections:

- **About You**: Working style, strengths, preferences
- **Directory Structure**: Your workspace organization
- **Available Commands**: What's installed
- **Google API**: What's configured

### Account/Project Configuration

If you manage accounts or projects, configure domain-to-account mappings:

```yaml
# In your CLAUDE.md or config file
account_domains:
  "clienta.com": "Client A"
  "clientb.org": "Client B"
```

### Email Classification

Configure domain patterns for email triage:

```yaml
email_domains:
  customers: [clienta.com, clientb.org]
  leadership: [mycompany.com]
  newsletters: [substack.com, mailchimp.com]
```

## HTML Documentation

Open `ui/index.html` in a browser for visual documentation including:
- Setup guide with screenshots
- Skills and commands reference
- Account structure explorer
- Google API setup walkthrough
- Troubleshooting FAQ

## Requirements

- Python 3.8+
- Claude Code CLI (recommended)
- Git (recommended)
- Google Cloud project (optional, for API integration)

## Development

The project structure:

```
Projects/Daily-Operating-System/
├── setup.py              # Main entry point
├── requirements.txt      # Python dependencies
├── src/
│   ├── wizard.py         # Main orchestrator
│   ├── steps/            # Setup step modules
│   ├── ui/               # Terminal UI helpers
│   └── utils/            # File operations, validators
├── templates/
│   ├── commands/         # Command definitions
│   ├── skills/           # Skill packages
│   ├── agents/           # Agent definitions
│   └── scripts/          # Python tools
├── ui/                   # HTML documentation
└── docs/                 # Additional documentation
```

## Contributing

This is a personal productivity system. Fork and customize for your needs.

## License

MIT

---

*Built on Claude Code by Anthropic*
