# ADR-0027: MCP integration: dual-mode server + client

**Date:** 2026-02
**Status:** Accepted

## Context

DailyOS has data (workspace, briefings, actions) that external AI tools want to access. It also needs to consume external services (Clay, potentially Slack, Linear).

## Decision

The app is both an MCP server (exposes workspace tools/resources to Claude Desktop, agents, automation) and an MCP client (consumes external MCP services like Clay).

**Server mode exposes:** workspace structure, today's briefing, account dashboards, action lists, meeting schedule, processing queue status.

**Client mode consumes:** Clay (contact lookup), and future services (Slack, Linear, Notion).

**Interface parity (extends ADR-0025):** App, CLI, and MCP share the same registries (`~/.dailyos/`) and workspace.

## Consequences

- Positions DailyOS as an AI-native integration hub
- Users in Claude Desktop can query DailyOS data without opening the app
- MCP server binds to localhost only — no remote access
- Sensitive data respects privacy levels from frontmatter
- Phase 4 feature — but IPC commands (ADR-0025) should be designed to be MCP-exposable from the start
