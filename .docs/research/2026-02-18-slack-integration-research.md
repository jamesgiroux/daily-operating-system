# Slack Integration Research — Distribution Surface for DailyOS Intelligence

**Date:** 2026-02-18
**Purpose:** Map out options for sharing DailyOS intelligence via Slack. Archive for future implementation.
**Status:** Research complete, not scheduled.

---

## Key Insight

Slack is a **collaboration surface**, not a signal source for DailyOS. The primary use case is pushing DailyOS output into Slack conversations — not pulling Slack messages into DailyOS. Users are already in Slack chatting with colleagues about accounts and meetings; they want to share intelligence without leaving the conversation.

## Use Cases (Priority Order)

### 1. Share intelligence from Slack (primary)
User is in a Slack channel discussing an upcoming call. A colleague asks about the account. User invokes a slash command and DailyOS posts the intelligence directly in the channel.

- `/dailyos prep <account>` — next meeting prep/briefing
- `/dailyos summary <account>` — last meeting outcome/summary
- `/dailyos health <account>` — account health/risk card
- `/dailyos actions <account>` — open action items

### 2. Automated posting (secondary)
DailyOS posts meeting summaries to account-specific Slack channels after transcript processing completes. Requires account-to-channel mapping.

### 3. Freeform queries (future, needs LLM)
"@DailyOS what's the status of the Acme renewal?" — requires piping query through Claude API with local context. Out of scope for initial implementation.

## Recommended Architecture: Bolt Sidecar + Socket Mode

### Why Socket Mode
DailyOS is a local desktop app with no public URL. Slack's Socket Mode uses WebSocket connections initiated from the client, eliminating the need for a public endpoint. The app connects outbound to Slack's servers — works behind firewalls, no tunneling needed.

### Components

**Slack App (created at api.slack.com):**
- Socket Mode enabled (requires app-level token, `xapp-`)
- Bot token (`xoxb-`) with scopes: `commands`, `chat:write`
- Slash command registered: `/dailyos`
- Workspace admin approval required

**Bolt Sidecar (Node.js):**
- Small Node.js app using `@slack/bolt` framework
- Spawned by DailyOS as a sidecar process (same pattern as Quill MCP bridge)
- Receives slash commands via Socket Mode WebSocket
- Queries local SQLite DB directly (or via IPC to Tauri backend)
- Formats responses using Slack Block Kit
- Posts back to the channel

**DailyOS Integration:**
- Slack section in Settings page (bot token, app token, enable/disable)
- Tokens stored in macOS Keychain
- Sidecar lifecycle managed by DailyOS (spawn on enable, kill on disable)
- Optional: account → Slack channel mapping field on account detail page

### Data Flow
```
User types /dailyos prep Acme in Slack
  -> Slack sends payload via WebSocket to Bolt sidecar
  -> Sidecar queries local DailyOS SQLite
  -> Formats response as Slack Block Kit message
  -> Posts back to channel via chat.postMessage
  -> Colleagues see the intelligence inline
```

### Constraint
Only works when DailyOS is running on the user's machine. Since the user is the one typing the command, their machine is necessarily on. Teammates cannot invoke it independently.

## Why Not MCP Server

The Claude Slack app can use MCP tools, which would enable "@Claude what's the health of Acme?" with Claude querying DailyOS MCP tools. However, Claude in Slack runs in Anthropic's cloud and cannot reach a localhost MCP server. This breaks the local-only data architecture.

The existing Claude Desktop MCP integration already supports this workflow via cmd-tab (ask Claude Desktop, copy result to Slack), but it's not as seamless as slash commands.

## Why Not Slack as Signal Source

Considered and rejected. Slack messages are collaborative, not intelligence signals. Passively monitoring would require broad read permissions and create noise. User-triggered "send to DailyOS" (message shortcuts) would work mechanically but the use case is weak — meeting transcripts and email already capture the meaningful signal.

## Slack MCP Server (Reference)

Slack launched their official MCP server on 2026-02-17. Provides: search messages/files/channels, read channel history, send messages, canvas management. Requires `SLACK_BOT_TOKEN` and `SLACK_TEAM_ID`. Could be used for background context enrichment in the future but doesn't solve the primary "share from Slack" use case.

## Implementation Estimate

- Slack app setup: manual (api.slack.com, one-time)
- Bolt sidecar: ~200 lines of Node.js
- Settings UI: follows existing Quill/Granola pattern
- Slash command handlers: one per intelligence type (prep, summary, health, actions)
- Block Kit formatting: Slack's rich message format for readable output
- Total: small feature, similar scope to Granola integration

## Dependencies

- Slack workspace admin approval for Socket Mode app
- `@slack/bolt` npm package
- Slack Block Kit for message formatting
- Existing DailyOS DB queries (account health, meeting prep, etc.)
