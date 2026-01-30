# Daily Operating System

A productivity framework built on Claude Code for managing your daily work, strategic thinking, and professional development.

## Philosophy

**Consuming, not producing.** You shouldn't have to maintain your productivity tools. They should just be productive.

**Works when you work.** Tuesday and Thursday this week. Wednesday and Friday next week. The system adapts to your rhythm.

**Everything changeable or removable.** If it's not working for you, change it or remove it. No sacred cows.

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
| `/today` | Your morning—dashboard, meeting prep, inbox processing |
| `/wrap` | Your evening—close loops, capture wins, archive |
| `/week` | Monday—plan the week ahead, surface what's coming |
| `/month` | Roll up your monthly impacts from weekly captures |
| `/quarter` | Pre-fill your quarterly review with tracked evidence |
| `/email-scan` | Triage inbox—surface important, draft responses, archive noise |
| `/git-commit` | Save your work—atomic commits with clear messages |

### Skills (3)

| Skill | Purpose |
|-------|---------|
| **inbox** | Drop any file—it gets renamed, summarized, tagged, and filed automatically |
| **strategy-consulting** | McKinsey in your terminal—SCQA framing, issue trees, pyramid principle |
| **editorial** | Multi-pass writing review—catch AI-tells, check voice, challenge premises |

### Agents (16)

**Strategic** (from strategy-consulting):
- problem-framer: Problem framing (SCQA + Day 1 Hypothesis)
- framework-strategist: Issue tree construction (MECE)
- red-team: Quality control and red team
- evidence-analyst: Evidence gathering and validation
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

**Inbox** (from inbox):
- file-organizer: PARA routing for documents
- integration-linker: External system links

**Commands**:
- agenda-generator: Draft agendas for upcoming meetings

## Directory Structure

Setup creates this folder structure for you:

```
workspace/
├── _today/             # Your daily command center
│   ├── tasks/          # Persistent task tracking
│   └── archive/        # Previous days (auto-managed)
├── _inbox/             # Drop zone—files get processed and filed
├── Accounts/           # Per customer: meetings, transcripts, actions
├── Projects/           # Active initiatives with deadlines
├── Areas/              # Ongoing responsibilities (leadership, development)
├── Resources/          # Templates, reference docs, standards
├── .config/google/     # Google API credentials (optional)
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
daily-operating-system/
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
├── ui/                   # HTML companion guide
└── docs/                 # Additional documentation
```

## Contributing

Contributions welcome! Open an issue or submit a PR:
- **New skills or agents** — Share workflows that work for you
- **Bug fixes** — Found something broken? Let us know
- **Documentation** — Help make setup clearer for others

Fork and customize for your needs.

## License

GPL-3.0

---

*Built on Claude Code by Anthropic*
