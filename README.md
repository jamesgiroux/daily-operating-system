# Daily Operating System

A productivity framework built on Claude Code for managing your daily work, strategic thinking, and professional development.

> **⚠️ Pre-Release Software (v0.8.0)**
>
> This project is under active development heading toward a stable 1.0 release. While functional, you may encounter bugs, breaking changes, or incomplete features. We appreciate early adopters and welcome [bug reports and feedback](https://github.com/jamesgiroux/daily-operating-system/issues).
>
> **Current status:** Alpha → working toward 1.0 stable release
>
> See [CHANGELOG.md](CHANGELOG.md) for version history.

## Philosophy

**Consuming, not producing.** You shouldn't have to maintain your productivity tools. They should just be productive.

**Works when you work.** Tuesday and Thursday this week. Wednesday and Friday next week. The system adapts to your rhythm.

**Everything changeable or removable.** If it's not working for you, change it or remove it. No sacred cows.

## Quick Start

### Step 1: Download the project

**Option A: Download ZIP (easiest)**
1. Click the green **Code** button above
2. Click **Download ZIP**
3. Unzip to a folder on your computer (e.g., `Documents/daily-operating-system`)

**Option B: Clone with Git**
```bash
git clone https://github.com/jamesgiroux/daily-operating-system.git
```

### Step 2: Launch the setup wizard

**Web-based wizard (recommended for beginners):**
Double-click `easy-start.command` in the downloaded folder. A browser opens with an interactive setup wizard.

**Terminal wizard (more options):**
```bash
cd ~/Documents/daily-operating-system
python3 advanced-start.py
```

Both wizards walk you through 10 steps:

1. **Prerequisites** — Checks Python, Claude Code, Git
2. **Workspace Location** — Where to create your productivity folder
3. **Directory Structure** — Creates PARA folders based on your role
4. **Git Setup** — Initializes version control
5. **Google API** — Optional calendar/email/sheets integration
6. **CLAUDE.md** — Generates your Claude Code configuration
7. **Skills & Commands** — Installs slash commands and skill packages
8. **Web Dashboard** — Optional browser-based UI for navigation
9. **Python Tools** — Installs automation scripts
10. **Verification** — Confirms everything works

### Optional flags (advanced-start.py only)

```bash
python3 advanced-start.py --workspace ~/Documents/productivity  # Custom location
python3 advanced-start.py --quick   # Use defaults, fewer prompts
python3 advanced-start.py --verify  # Check existing installation
python3 advanced-start.py --google  # Configure Google API only
```

## Role-Based Setup

The wizard asks how you manage your work and configures the folder structure accordingly:

| Role | Description | Primary Folders |
|------|-------------|-----------------|
| **Customer Success** | TAMs, RMs, CSMs, AOs with dedicated portfolios | Accounts/ (12 subfolders each) |
| **Sales** | AEs, BDRs, SEs with pipeline stages | Accounts/Active, Qualified, Future |
| **Project Management** | PMs, Program Managers | Projects/Active, Planning, Completed |
| **Product Management** | Product Managers | Features/Discovery, In-Progress, Shipped |
| **Marketing** | Campaign and content managers | Campaigns/Active, Planned, Completed |
| **Engineering** | Engineers, Tech Leads | Projects/Active, Backlog, Completed |
| **Consulting** | Consultants, Analysts | Engagements/Active, Completed |
| **General** | Flexible knowledge work | Standard PARA structure |

## What You Get

### Commands (8)

| Command | Purpose |
|---------|---------|
| `/today` | Your morning—dashboard, meeting prep, inbox processing |
| `/wrap` | Your evening—close loops, capture wins, archive |
| `/week` | Monday—plan the week ahead, surface what's coming |
| `/month` | Roll up your monthly impacts from weekly captures |
| `/quarter` | Pre-fill your quarterly review with tracked evidence |
| `/email-scan` | Triage inbox—surface important, draft responses, archive noise |
| `/git-commit` | Save your work—atomic commits with clear messages |
| `/setup` | Re-run setup or configure additional components |

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

## Web Dashboard (Optional)

The setup wizard can install a browser-based dashboard for visual navigation of your workspace.

### Features

- **Visual sidebar** — Navigate accounts, projects, and daily files
- **Markdown rendering** — View your documents formatted in the browser
- **Search** — Find content across all your documents
- **Health indicators** — See account status at a glance (Customer Success roles)
- **Ring badges** — Visual lifecycle positioning (Summit, Influence, Evolution, Foundation)

### Requirements

- Node.js (for running the local server)

### Manual Start

If you installed the dashboard during setup:

```bash
cd ~/Documents/productivity/_ui
npm start
```

Then open http://localhost:5050 in your browser.

### Configuration

The dashboard reads from `_ui/config/config.json`, which is auto-generated based on your role selection. You can customize:

- **Sections** — Which folders appear in the sidebar
- **Subsections** — Folder icons and labels
- **Features** — Enable/disable health status, ring badges, etc.
- **Today links** — Quick access to daily files

Role-specific templates are in `_ui/config/roles/`.

## Directory Structure

Setup creates this folder structure for you:

```
workspace/
├── _today/             # Your daily command center
│   ├── tasks/          # Persistent task tracking
│   └── archive/        # Previous days (auto-managed)
├── _inbox/             # Drop zone—files get processed and filed
├── _ui/                # Web dashboard (if installed)
│   ├── config/         # Dashboard configuration
│   ├── public/         # Frontend assets
│   └── server.js       # Express server
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

## Requirements

- Python 3.8+
- Claude Code CLI (recommended)
- Git (recommended)
- Node.js (optional, for web dashboard)
- Google Cloud project (optional, for API integration)

## Development

The project structure:

```
daily-operating-system/
├── easy-start.command    # Web-based setup wizard (beginners)
├── advanced-start.py     # CLI setup wizard (more options)
├── requirements.txt      # Python dependencies
├── src/
│   ├── wizard.py         # Main orchestrator (10 steps)
│   ├── steps/            # Setup step modules
│   │   ├── directories.py    # Role-based folder creation
│   │   ├── ui_setup.py       # Web dashboard installation
│   │   └── ...
│   ├── ui/               # Terminal UI helpers
│   └── utils/            # File operations, validators
├── templates/
│   ├── commands/         # Command definitions
│   ├── skills/           # Skill packages
│   ├── agents/           # Agent definitions
│   ├── scripts/          # Python tools
│   └── ui/               # Web dashboard template
│       ├── config/
│       │   └── roles/    # Role-specific configs
│       ├── public/       # Frontend (HTML, CSS, JS)
│       └── server.js     # Express server
├── docs/                 # HTML companion guide
└── tests/                # Test suite
```

### Running Tests

```bash
python3 -m pytest tests/ -v
```

## Contributing

Contributions welcome! Open an issue or submit a PR:
- **Bug reports** — Found something broken? [Open an issue](https://github.com/jamesgiroux/daily-operating-system/issues)
- **New skills or agents** — Share workflows that work for you
- **Documentation** — Help make setup clearer for others
- **Role templates** — Add configurations for new work styles

### Development Workflow

We use a simple branching model:
- `main` — Stable releases only (tagged versions)
- `dev` — Active development (PRs merge here)

To contribute:
1. Fork the repo
2. Create a feature branch from `dev`
3. Submit a PR back to `dev`

See [CHANGELOG.md](CHANGELOG.md) for version history.

## License

GPL-3.0

---

*Built on Claude Code by Anthropic*
