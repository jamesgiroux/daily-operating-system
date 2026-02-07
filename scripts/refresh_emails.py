#!/usr/bin/env python3
"""Standalone email refresh for DailyOS (I20).

Re-fetches and classifies emails without re-running the full /today pipeline.
Writes email-refresh-directive.json to _today/data/ for Rust to consume.

Uses customer domains from the morning's schedule.json (if available)
and account domain hints from the workspace Accounts/ directory.

Usage:
    python3 refresh_emails.py [workspace_path]
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Add scripts/ to path for ops imports
sys.path.insert(0, str(Path(__file__).resolve().parent))

from ops.config import (
    build_account_domain_hints,
    get_user_domain,
    load_config,
    resolve_workspace,
    write_json,
    _info,
    _warn,
)
from ops.email_fetch import fetch_and_classify_emails


def main() -> None:
    workspace = resolve_workspace()
    config = load_config()
    user_domain = get_user_domain(config)
    account_hints = build_account_domain_hints(workspace)

    # Extract customer domains from morning's schedule.json if available
    customer_domains: set[str] = set()
    schedule_path = workspace / "_today" / "data" / "schedule.json"
    if schedule_path.exists():
        try:
            schedule = json.loads(schedule_path.read_text(encoding="utf-8"))
            for meeting in schedule.get("meetings", []):
                for attendee in meeting.get("attendees", []):
                    if "@" in attendee:
                        domain = attendee.split("@")[1].lower()
                        customer_domains.add(domain)
        except (json.JSONDecodeError, OSError):
            _warn("Could not read schedule.json for customer domains")

    _info(f"Email refresh: user_domain={user_domain}, "
          f"customer_domains={len(customer_domains)}, "
          f"account_hints={len(account_hints)}")

    result = fetch_and_classify_emails(
        customer_domains=customer_domains,
        user_domain=user_domain,
        account_hints=account_hints,
    )

    # Build refresh directive matching the shape Rust expects
    high_priority = []
    classified = []
    for email in result.all_emails:
        entry = {
            "id": email.get("id"),
            "from": email.get("from"),
            "from_email": email.get("from_email"),
            "subject": email.get("subject"),
            "snippet": email.get("snippet"),
            "priority": email.get("priority"),
        }
        if email.get("priority") == "high":
            high_priority.append(entry)
        else:
            classified.append(entry)

    directive = {
        "source": "email-refresh",
        "emails": {
            "highPriority": high_priority,
            "classified": classified,
            "mediumCount": result.medium_count,
            "lowCount": result.low_count,
        },
    }

    data_dir = workspace / "_today" / "data"
    data_dir.mkdir(parents=True, exist_ok=True)
    output_path = data_dir / "email-refresh-directive.json"
    write_json(output_path, directive)

    _info(f"Email refresh complete: {len(result.all_emails)} emails "
          f"({len(result.high)} high, {result.medium_count} medium, "
          f"{result.low_count} low)")

    # Output JSON to stdout for Rust to read
    print(json.dumps({"status": "ok", "path": str(output_path)}))


if __name__ == "__main__":
    main()
