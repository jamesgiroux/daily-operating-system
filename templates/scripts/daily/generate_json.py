#!/usr/bin/env python3
"""
Generate JSON data files from markdown for Daybreak consumption.

This script reads the markdown files in _today/ and generates structured JSON
in _today/data/ for the Daybreak app to consume.

Usage:
    python generate_json.py [workspace_path]

If workspace_path is not provided, uses current directory.
"""

import json
import os
import re
import sys
from datetime import datetime
from pathlib import Path
from typing import Any, Optional


def parse_overview_md(content: str) -> dict:
    """Parse 00-overview.md to extract greeting, date, summary."""
    result = {
        "greeting": "Good morning",
        "date": datetime.now().strftime("%A, %B %d"),
        "summary": "",
        "focus": None,
    }

    # Extract date from header like "# Wednesday, February 5"
    date_match = re.search(r"^#\s+(\w+,\s+\w+\s+\d+)", content, re.MULTILINE)
    if date_match:
        result["date"] = date_match.group(1)

    # Determine greeting based on current time
    hour = datetime.now().hour
    if hour < 12:
        result["greeting"] = "Good morning"
    elif hour < 17:
        result["greeting"] = "Good afternoon"
    else:
        result["greeting"] = "Good evening"

    # Extract summary - first paragraph after the date heading
    summary_match = re.search(r"^#[^\n]+\n\n([^\n#]+)", content, re.MULTILINE)
    if summary_match:
        result["summary"] = summary_match.group(1).strip()

    # Extract focus if present
    focus_match = re.search(r"\*\*Focus[:\s]*\*\*\s*(.+)", content)
    if focus_match:
        result["focus"] = focus_match.group(1).strip()

    return result


def parse_schedule_table(content: str) -> list[dict]:
    """Parse schedule table from overview to get basic meeting info."""
    meetings = []

    # Find schedule table
    table_match = re.search(
        r"## Today's Schedule.*?\n\|(.*?)\n\|[-\s|]+\n((?:\|.*\n)*)",
        content,
        re.DOTALL,
    )
    if not table_match:
        return meetings

    rows = table_match.group(2).strip().split("\n")
    for row in rows:
        cols = [c.strip() for c in row.split("|")[1:-1]]  # Skip empty first/last
        if len(cols) >= 3:
            time_str = cols[0].strip()
            title = cols[1].strip()
            meeting_type = cols[2].strip().lower() if len(cols) > 2 else "internal"

            # Clean up type
            if "customer" in meeting_type or "external" in meeting_type:
                meeting_type = "customer"
            elif "personal" in meeting_type:
                meeting_type = "personal"
            else:
                meeting_type = "internal"

            # Generate ID from time and title
            time_clean = re.sub(r"[^0-9]", "", time_str.split("-")[0])[:4]
            title_slug = re.sub(r"[^a-z0-9]+", "-", title.lower())[:30].strip("-")
            meeting_id = f"{time_clean}-{title_slug}"

            meeting = {
                "id": meeting_id,
                "time": time_str.split("-")[0].strip() if "-" in time_str else time_str,
                "title": title,
                "type": meeting_type,
                "has_prep": False,
                "prep_file": None,
                "prep_summary": None,
            }

            # Check for end time
            if "-" in time_str:
                parts = time_str.split("-")
                if len(parts) == 2:
                    meeting["end_time"] = parts[1].strip()

            meetings.append(meeting)

    return meetings


def parse_prep_file(path: Path) -> Optional[dict]:
    """Parse an individual prep markdown file."""
    if not path.exists():
        return None

    content = path.read_text()
    prep = {
        "title": "",
        "time_range": "",
        "meeting_context": None,
        "quick_context": {},
        "attendees": [],
        "since_last": [],
        "strategic_programs": [],
        "risks": [],
        "talking_points": [],
        "open_items": [],
        "questions": [],
        "key_principles": [],
        "references": [],
    }

    # Extract title from first heading
    title_match = re.search(r"^#\s+(.+)", content, re.MULTILINE)
    if title_match:
        prep["title"] = title_match.group(1).strip()

    # Extract time range
    time_match = re.search(r"\*\*Time[:\s]*\*\*\s*(.+)", content)
    if time_match:
        prep["time_range"] = time_match.group(1).strip()

    # Extract Quick Context table
    qc_match = re.search(
        r"## Quick Context.*?\n\|(.*?)\n\|[-\s|]+\n((?:\|.*\n)*)",
        content,
        re.DOTALL,
    )
    if qc_match:
        rows = qc_match.group(2).strip().split("\n")
        for row in rows:
            cols = [c.strip() for c in row.split("|")[1:-1]]
            if len(cols) >= 2:
                prep["quick_context"][cols[0]] = cols[1]

    # Extract sections with bullet points
    def extract_bullets(section_name: str) -> list[str]:
        pattern = rf"##\s+{section_name}.*?\n((?:[-*]\s+.+\n?)+)"
        match = re.search(pattern, content, re.IGNORECASE)
        if match:
            bullets = re.findall(r"[-*]\s+(.+)", match.group(1))
            return [b.strip() for b in bullets]
        return []

    prep["since_last"] = extract_bullets("Since Last Meeting")
    prep["risks"] = extract_bullets("Current Risks|Risks to Monitor")
    prep["talking_points"] = extract_bullets("Suggested Talking Points|Talking Points")
    prep["questions"] = extract_bullets("Questions to Surface|Questions")
    prep["key_principles"] = extract_bullets("Key Principles")

    # Extract strategic programs (may have checkmarks)
    programs_match = re.search(
        r"##\s+(?:Current )?Strategic Programs.*?\n((?:[-*✓○]\s+.+\n?)+)",
        content,
        re.IGNORECASE,
    )
    if programs_match:
        for line in programs_match.group(1).strip().split("\n"):
            line = line.strip()
            if line.startswith("✓") or line.startswith("[x]"):
                name = re.sub(r"^[✓\[\]x]+\s*", "", line)
                prep["strategic_programs"].append({"name": name, "status": "completed"})
            elif line.startswith("-") or line.startswith("*") or line.startswith("○"):
                name = re.sub(r"^[-*○]\s*", "", line)
                prep["strategic_programs"].append({"name": name, "status": "in_progress"})

    # Extract attendees
    attendees_match = re.search(
        r"##\s+(?:Key )?Attendees.*?\n((?:[-*]\s+.+\n?)+)",
        content,
        re.IGNORECASE,
    )
    if attendees_match:
        for line in attendees_match.group(1).strip().split("\n"):
            line = re.sub(r"^[-*]\s*", "", line).strip()
            # Parse "Name (Role) - Focus" or "Name - Role"
            name_match = re.match(r"([^(-]+)(?:\(([^)]+)\))?(?:\s*[-–]\s*(.+))?", line)
            if name_match:
                prep["attendees"].append({
                    "name": name_match.group(1).strip(),
                    "role": name_match.group(2).strip() if name_match.group(2) else None,
                    "focus": name_match.group(3).strip() if name_match.group(3) else None,
                })

    # Extract open items (action items)
    items_match = re.search(
        r"##\s+Open Items.*?\n((?:[-*]\s+.+\n?)+)",
        content,
        re.IGNORECASE,
    )
    if items_match:
        for line in items_match.group(1).strip().split("\n"):
            line = re.sub(r"^[-*]\s*", "", line).strip()
            item = {"title": line, "is_overdue": False}
            # Check for due date
            due_match = re.search(r"\((?:due:?\s*)?(\d{4}-\d{2}-\d{2})\)", line)
            if due_match:
                item["due_date"] = due_match.group(1)
                item["title"] = re.sub(r"\s*\([^)]+\)\s*", "", line).strip()
                # Check if overdue
                try:
                    due_date = datetime.strptime(due_match.group(1), "%Y-%m-%d")
                    if due_date.date() < datetime.now().date():
                        item["is_overdue"] = True
                except ValueError:
                    pass
            prep["open_items"].append(item)

    return prep


def parse_actions_md(content: str) -> list[dict]:
    """Parse 80-actions-due.md to extract action items."""
    actions = []
    action_id = 0

    # Find action items - support various formats
    # Format 1: "- [ ] Action text (Account) - Due: date"
    # Format 2: "- **P1** Action text"
    # Format 3: "### Account Name\n- Action text"

    current_account = None
    current_priority = "P2"

    for line in content.split("\n"):
        line = line.strip()

        # Check for account header
        account_match = re.match(r"^###?\s+(.+)", line)
        if account_match:
            current_account = account_match.group(1).strip()
            continue

        # Check for priority marker
        priority_match = re.search(r"\*\*(P[123])\*\*", line)
        if priority_match:
            current_priority = priority_match.group(1)

        # Check for action item
        if line.startswith("-") or line.startswith("*"):
            action_text = re.sub(r"^[-*]\s*(\[.\]\s*)?", "", line).strip()
            action_text = re.sub(r"\*\*P[123]\*\*\s*", "", action_text)

            if not action_text:
                continue

            action_id += 1
            action = {
                "id": f"action-{action_id:03d}",
                "title": action_text,
                "priority": current_priority,
                "status": "pending",
                "is_overdue": False,
            }

            # Extract account from parentheses
            account_match = re.search(r"\(([^)]+(?:Corp|Inc|LLC|Co\.?)?)\)", action_text)
            if account_match:
                action["account"] = account_match.group(1)
                action["title"] = re.sub(r"\s*\([^)]+\)\s*", " ", action["title"]).strip()
            elif current_account:
                action["account"] = current_account

            # Extract due date
            due_match = re.search(r"(?:due|by)[:\s]*(\d{4}-\d{2}-\d{2}|\w+\s+\d+)", action_text, re.IGNORECASE)
            if due_match:
                action["due_date"] = due_match.group(1)
                action["title"] = re.sub(r"\s*[-–]\s*(?:due|by)[:\s]*[^\s]+", "", action["title"], flags=re.IGNORECASE).strip()

                # Check if overdue
                try:
                    due_date = datetime.strptime(due_match.group(1), "%Y-%m-%d")
                    if due_date.date() < datetime.now().date():
                        action["is_overdue"] = True
                        action["days_overdue"] = (datetime.now().date() - due_date.date()).days
                except ValueError:
                    pass

            # Clean up title
            action["title"] = re.sub(r"\s+", " ", action["title"]).strip()

            if action["title"]:
                actions.append(action)

    return actions


def parse_emails_md(content: str) -> list[dict]:
    """Parse email summary or overview email table."""
    emails = []
    email_id = 0

    # Try table format first (from overview)
    table_match = re.search(
        r"## Email.*?\n\|(.*?)\n\|[-\s|]+\n((?:\|.*\n)*)",
        content,
        re.DOTALL | re.IGNORECASE,
    )
    if table_match:
        rows = table_match.group(2).strip().split("\n")
        for row in rows:
            cols = [c.strip() for c in row.split("|")[1:-1]]
            if len(cols) >= 2:
                email_id += 1
                sender = cols[0].strip()
                subject = cols[1].strip()
                priority = "high" if "🔴" in row or "urgent" in row.lower() else "normal"

                emails.append({
                    "id": f"email-{email_id:03d}",
                    "sender": sender,
                    "sender_email": "",
                    "subject": subject,
                    "priority": priority,
                })
        return emails

    # Try bullet list format
    for line in content.split("\n"):
        if line.strip().startswith("-") or line.strip().startswith("*"):
            email_id += 1
            text = re.sub(r"^[-*]\s*", "", line).strip()

            # Try to parse "From: Subject" or "Sender - Subject"
            parts = re.split(r"[-–:]", text, 1)
            sender = parts[0].strip() if parts else "Unknown"
            subject = parts[1].strip() if len(parts) > 1 else text

            emails.append({
                "id": f"email-{email_id:03d}",
                "sender": sender,
                "sender_email": "",
                "subject": subject,
                "priority": "normal",
            })

    return emails


def generate_json_data(workspace: Path) -> None:
    """Generate all JSON data files from markdown sources."""
    today_dir = workspace / "_today"
    data_dir = today_dir / "data"
    preps_dir = data_dir / "preps"

    # Create directories
    data_dir.mkdir(parents=True, exist_ok=True)
    preps_dir.mkdir(exist_ok=True)

    # Read overview
    overview_path = today_dir / "00-overview.md"
    overview_content = ""
    if overview_path.exists():
        overview_content = overview_path.read_text()

    # Generate schedule.json
    overview_data = parse_overview_md(overview_content)
    meetings = parse_schedule_table(overview_content)

    # Find and parse prep files
    for prep_file in sorted(today_dir.glob("*-prep.md")):
        prep_data = parse_prep_file(prep_file)
        if prep_data:
            # Find matching meeting
            prep_name = prep_file.stem
            for meeting in meetings:
                if meeting["title"].lower() in prep_data["title"].lower() or \
                   prep_data["title"].lower() in meeting["title"].lower():
                    meeting["has_prep"] = True
                    meeting["prep_file"] = f"preps/{prep_name}.json"

                    # Create prep summary for schedule
                    meeting["prep_summary"] = {
                        "at_a_glance": [f"{k}: {v}" for k, v in list(prep_data["quick_context"].items())[:4]],
                        "discuss": prep_data["talking_points"][:4] or prep_data["questions"][:4],
                        "watch": prep_data["risks"][:3],
                        "wins": [p["name"] for p in prep_data["strategic_programs"] if p["status"] == "completed"][:3],
                    }
                    break

            # Write individual prep JSON
            prep_json_path = preps_dir / f"{prep_name}.json"
            with open(prep_json_path, "w") as f:
                json.dump({
                    "meeting_id": prep_name,
                    **prep_data,
                }, f, indent=2)
            print(f"  Created {prep_json_path.relative_to(workspace)}")

    # Write schedule.json
    schedule_data = {
        "date": datetime.now().strftime("%Y-%m-%d"),
        "greeting": overview_data["greeting"],
        "summary": overview_data["summary"],
        "focus": overview_data["focus"],
        "meetings": meetings,
    }
    schedule_path = data_dir / "schedule.json"
    with open(schedule_path, "w") as f:
        json.dump(schedule_data, f, indent=2)
    print(f"  Created {schedule_path.relative_to(workspace)}")

    # Generate actions.json
    actions_path = today_dir / "80-actions-due.md"
    if actions_path.exists():
        actions = parse_actions_md(actions_path.read_text())
        actions_data = {
            "date": datetime.now().strftime("%Y-%m-%d"),
            "summary": {
                "overdue": len([a for a in actions if a.get("is_overdue")]),
                "due_today": len([a for a in actions if a.get("due_date") == datetime.now().strftime("%Y-%m-%d")]),
                "due_this_week": len(actions),
            },
            "actions": actions,
        }
        actions_json_path = data_dir / "actions.json"
        with open(actions_json_path, "w") as f:
            json.dump(actions_data, f, indent=2)
        print(f"  Created {actions_json_path.relative_to(workspace)}")

    # Generate emails.json
    emails = parse_emails_md(overview_content)
    email_summary_path = today_dir / "83-email-summary.md"
    if email_summary_path.exists():
        emails = parse_emails_md(email_summary_path.read_text()) or emails

    if emails:
        emails_data = {
            "date": datetime.now().strftime("%Y-%m-%d"),
            "stats": {
                "high_priority": len([e for e in emails if e["priority"] == "high"]),
                "normal_priority": len([e for e in emails if e["priority"] == "normal"]),
                "needs_action": len(emails),
            },
            "emails": emails,
        }
        emails_json_path = data_dir / "emails.json"
        with open(emails_json_path, "w") as f:
            json.dump(emails_data, f, indent=2)
        print(f"  Created {emails_json_path.relative_to(workspace)}")

    # Generate manifest.json
    manifest = {
        "schema_version": "1.0.0",
        "date": datetime.now().strftime("%Y-%m-%d"),
        "generated_at": datetime.now().isoformat(),
        "partial": False,
        "files": {
            "schedule": "schedule.json",
            "actions": "actions.json" if (data_dir / "actions.json").exists() else None,
            "emails": "emails.json" if (data_dir / "emails.json").exists() else None,
            "preps": [f.name for f in preps_dir.glob("*.json")],
        },
        "stats": {
            "total_meetings": len(meetings),
            "customer_meetings": len([m for m in meetings if m["type"] == "customer"]),
            "actions_due": len(actions) if actions_path.exists() else 0,
            "emails_flagged": len(emails),
        },
    }
    manifest_path = data_dir / "manifest.json"
    with open(manifest_path, "w") as f:
        json.dump(manifest, f, indent=2)
    print(f"  Created {manifest_path.relative_to(workspace)}")


def main():
    workspace = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd()

    if not workspace.exists():
        print(f"Error: Workspace not found: {workspace}")
        sys.exit(1)

    today_dir = workspace / "_today"
    if not today_dir.exists():
        print(f"Error: No _today directory found in {workspace}")
        print("Run /today first to generate your daily briefing.")
        sys.exit(1)

    print(f"Generating JSON data from {today_dir}...")
    generate_json_data(workspace)
    print("Done!")


if __name__ == "__main__":
    main()
