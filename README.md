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

### Google OAuth Secret Management

- Production builds require `DAILYOS_GOOGLE_SECRET` at compile time.
- Local development does not require that env var if `~/.dailyos/google/credentials.json` is present; file credentials override embedded defaults.
- Release workflow (`.github/workflows/release.yml`) fails fast if `DAILYOS_GOOGLE_SECRET` is missing.

Rotation procedure:

1. Create a new Google OAuth Desktop client in Google Cloud Console.
2. Set the new client secret in GitHub repo secret `DAILYOS_GOOGLE_SECRET`.
3. Build and verify OAuth end-to-end in a release artifact.
4. Revoke/delete the previous OAuth client.
5. Rewrite git history to remove exposed secrets and force-push rewritten refs.

## Architecture

Tauri v2 app with a Rust backend and React/TypeScript frontend. Data flows through three tiers: filesystem (durable markdown + JSON), SQLite (working store), and app memory (ephemeral). AI enrichment runs through Claude Code CLI spawned as a PTY subprocess.

## Documentation

- [PHILOSOPHY.md](design/PHILOSOPHY.md) -- Why we exist
- [PRINCIPLES.md](design/PRINCIPLES.md) -- Design principles
- [VISION.md](design/VISION.md) -- Product vision
- [oauth-secret-rotation.md](docs/oauth-secret-rotation.md) -- OAuth rotation + history rewrite runbook

Product website: [daily-os.com](https://daily-os.com)

## License

[GPL-3.0](LICENSE)

## Links

- [Website](https://daily-os.com)
- [Releases](https://github.com/jamesgiroux/daily-operating-system/releases)
- [Issues](https://github.com/jamesgiroux/daily-operating-system/issues)
