# DailyOS

> Open the app. Your day is ready.

Operational intelligence for your accounts, projects, and people.

## What is DailyOS?

DailyOS is a native desktop app that connects to your Google Calendar and Gmail, builds persistent intelligence about your accounts, projects, and people, and prepares your day every morning. It runs locally on your machine -- your data stays in markdown and JSON files you own. AI enrichment is powered by Claude Code (your existing subscription, no API keys).

## Install

Download the latest `.dmg` from [GitHub Releases](https://github.com/jamesgiroux/daily-operating-system/releases).

On first launch, macOS Gatekeeper may block the app. Right-click the app, select Open, then confirm.

**Prerequisites:**

- macOS (Apple Silicon)
- [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code) installed and authenticated
- Google account (Calendar + Gmail access, configured during onboarding)

## Features

- Daily briefing with AI-enriched meeting prep
- Account and project intelligence (executive assessments, risks, wins, stakeholder insights)
- People tracking with relationship history and meeting patterns
- Email triage with three-tier AI priority classification
- Action tracking across briefings, transcripts, and inbox
- Transcript processing with outcome extraction (actions, decisions, captures)
- Weekly narrative with priority synthesis and gap analysis
- Background scheduling (daily briefing, archive, intelligence refresh)
- Local-first: markdown + JSON on your filesystem, SQLite working store
- Open source (GPL-3.0)

## Development

```bash
pnpm install
pnpm tauri dev
```

**Prerequisites:** Rust 1.70+, Node.js 18+, pnpm 8+

**Run backend tests:**

```bash
cd src-tauri && cargo test
```

There are approximately 500 Rust tests covering the backend.

## Architecture

Tauri v2 app with a Rust backend and React/TypeScript frontend. Data flows through three tiers: filesystem (durable markdown + JSON), SQLite (working store), and app memory (ephemeral). AI enrichment runs through Claude Code CLI spawned as a PTY subprocess.

See [ARCHITECTURE.md](daybreak/docs/ARCHITECTURE.md) for full details and 59 [Architecture Decision Records](daybreak/docs/decisions/).

## Documentation

- [PHILOSOPHY.md](design/PHILOSOPHY.md) -- Why we exist
- [PRINCIPLES.md](design/PRINCIPLES.md) -- Design principles
- [VISION.md](design/VISION.md) -- Product vision
- [ARCHITECTURE.md](daybreak/docs/ARCHITECTURE.md) -- Technical architecture
- [decisions/](daybreak/docs/decisions/) -- Architecture Decision Records

Product website: [daily-os.com](https://daily-os.com)

## License

[GPL-3.0](LICENSE)

## Links

- [Website](https://daily-os.com)
- [Releases](https://github.com/jamesgiroux/daily-operating-system/releases)
- [Issues](https://github.com/jamesgiroux/daily-operating-system/issues)
