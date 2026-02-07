"""Shared atomic operations for DailyOS workflows (ADR-0030).

Each module is an independently callable operation:
    config          Workspace, config, profile, Google auth
    id_gen          Stable content-hash IDs
    calendar_fetch  Calendar fetch + classify (date range → classified events)
    gap_analysis    Calendar gap computation + focus blocks
    email_fetch     Email fetch + classify (Gmail → prioritised emails)
    meeting_prep    Meeting context gathering (single meeting → rich context)
    action_parse    Action parsing + SQLite pre-check
"""
