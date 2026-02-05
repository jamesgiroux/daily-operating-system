#!/usr/bin/env python3
"""
Account Dashboard Generator.

Generates comprehensive account dashboards from CSV data or creates blank templates.
Designed for use with the Daily Operating System.

Usage:
    python3 generate_account_dashboard.py --csv path/to/accounts.csv
    python3 generate_account_dashboard.py --account "Company Name"
    python3 generate_account_dashboard.py --all
    python3 generate_account_dashboard.py --help

The CSV should have these columns (order doesn't matter, headers required):
    Account, Status, Tier, ARR, Contract Value, Last Engagement Date,
    Last Renewal Date, Next Renewal Date, Primary Contact, Primary Email,
    Meeting Cadence, Support Package, Health Score, Churn Risk,
    Expansion Opportunity, Account Owner, Notes

Optional columns will be used if present, otherwise defaults are applied.
"""

import argparse
import csv
import os
import re
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional, Any


# Default CSV path relative to workspace
DEFAULT_CSV_PATH = "_reference/account-tracker.csv"

# Template path
TEMPLATE_PATH = "_templates/account-dashboard-template.md"


def slugify(name: str) -> str:
    """Convert account name to URL-friendly slug."""
    slug = name.lower()
    slug = re.sub(r'[^a-z0-9\s-]', '', slug)
    slug = re.sub(r'[\s_]+', '-', slug)
    slug = re.sub(r'-+', '-', slug)
    return slug.strip('-')


def parse_date(date_str: str) -> Optional[datetime]:
    """Parse date from various formats."""
    if not date_str or date_str.strip() == '':
        return None

    formats = [
        '%Y-%m-%d',
        '%m/%d/%Y',
        '%m/%d/%y',
        '%d/%m/%Y',
        '%B %d, %Y',
        '%b %d, %Y',
    ]

    for fmt in formats:
        try:
            return datetime.strptime(date_str.strip(), fmt)
        except ValueError:
            continue

    return None


def days_until(date: Optional[datetime]) -> str:
    """Calculate days until a future date."""
    if not date:
        return "Unknown"

    delta = date - datetime.now()
    days = delta.days

    if days < 0:
        return f"{abs(days)} days ago"
    elif days == 0:
        return "Today"
    elif days == 1:
        return "Tomorrow"
    elif days < 30:
        return f"{days} days"
    elif days < 365:
        months = days // 30
        return f"~{months} month{'s' if months != 1 else ''}"
    else:
        years = days // 365
        return f"~{years} year{'s' if years != 1 else ''}"


def format_currency(value: str) -> str:
    """Format currency value consistently."""
    if not value:
        return "$0"

    # Remove existing formatting
    clean = re.sub(r'[^\d.]', '', str(value))

    try:
        amount = float(clean)
        if amount >= 1000000:
            return f"${amount/1000000:.1f}M"
        elif amount >= 1000:
            return f"${amount:,.0f}"
        else:
            return f"${amount:.0f}"
    except ValueError:
        return value


def get_status_emoji(status: str, health: str = "", risk: str = "") -> str:
    """Get appropriate status emoji."""
    status_lower = status.lower() if status else ""
    health_lower = health.lower() if health else ""
    risk_lower = risk.lower() if risk else ""

    # Check risk first
    if risk_lower in ['high', 'critical']:
        return "🔴"
    elif risk_lower == 'medium':
        return "🟡"

    # Check health score
    if health:
        try:
            score = int(re.sub(r'[^\d]', '', health))
            if score >= 80:
                return "🟢"
            elif score >= 60:
                return "🟡"
            else:
                return "🔴"
        except ValueError:
            pass

    # Check status
    if status_lower in ['active', 'healthy', 'good']:
        return "🟢"
    elif status_lower in ['at risk', 'warning', 'attention']:
        return "🟡"
    elif status_lower in ['churned', 'lost', 'critical']:
        return "🔴"

    return "⚪"


def load_csv(csv_path: Path) -> List[Dict[str, str]]:
    """Load accounts from CSV file."""
    if not csv_path.exists():
        raise FileNotFoundError(f"CSV file not found: {csv_path}")

    accounts = []
    with open(csv_path, 'r', encoding='utf-8-sig') as f:
        reader = csv.DictReader(f)
        for row in reader:
            # Clean up keys (remove BOM, whitespace)
            cleaned = {k.strip(): v.strip() if v else '' for k, v in row.items() if k}
            accounts.append(cleaned)

    return accounts


def load_template(workspace: Path) -> str:
    """Load the dashboard template."""
    template_path = workspace / TEMPLATE_PATH

    if template_path.exists():
        with open(template_path, 'r') as f:
            return f.read()

    # Return embedded template if file doesn't exist
    return get_embedded_template()


def get_embedded_template() -> str:
    """Return embedded template if file template not available."""
    return '''---
area: "Accounts"
account: "{{ACCOUNT_NAME}}"
doc_type: "report"
date: "{{DATE}}"
status: "active"
privacy: "internal"
tags: ["dashboard", "account-health"]
report_type: "account-dashboard"
---

# {{ACCOUNT_NAME}} Dashboard
*Last Updated: {{DATE}}*
*Account Owner: {{ACCOUNT_OWNER}}*
*Status: {{STATUS_EMOJI}} {{STATUS}}*

---

## Quick View
| Metric | Value | Notes |
|--------|-------|-------|
| **ARR/Contract Value** | {{ARR}} | |
| **Health Score** | {{HEALTH_SCORE}} | |
| **Account Status** | {{STATUS}} | |
| **Meeting Cadence** | {{MEETING_CADENCE}} | Last: {{LAST_ENGAGEMENT}} |
| **Renewal Date** | {{RENEWAL_DATE}} | {{DAYS_TO_RENEWAL}} |
| **Support Package** | {{SUPPORT_PACKAGE}} | |
| **Primary Contact** | {{PRIMARY_CONTACT}} | |

---

## Critical Information

### ⚠️ Current Risks
<!-- AI ENRICHMENT NEEDED: Extract risks from transcripts and meeting notes -->
-

### ✅ Recent Wins
<!-- AI ENRICHMENT NEEDED: Extract wins from transcripts and meeting notes -->
-

### 🎯 Next Actions
<!-- AI ENRICHMENT NEEDED: Extract action items from transcripts -->
- [ ]

---

## Account Intelligence

### Business Context
<!-- AI ENRICHMENT NEEDED: Populate from discovery conversations -->
**Industry**:
**Company Overview**:
**Business Model**:
**Strategic Priority**:
**Relationship Type**:

### Current State

**Working Well**:
<!-- AI ENRICHMENT NEEDED: Extract positive signals -->
-

**Challenges**:
<!-- AI ENRICHMENT NEEDED: Extract challenges and friction points -->
-

**Unknown / Discovery Needed**:
<!-- AI ENRICHMENT NEEDED: Identify knowledge gaps -->
-

---

## Stakeholder Map

| Name | Role | Influence | Engagement Level | Last Contact | Notes |
|------|------|-----------|------------------|--------------|-------|
| {{PRIMARY_CONTACT}} | | High | Active | {{LAST_ENGAGEMENT}} | Primary contact |

### Missing Coverage (Discovery Needed)
<!-- AI ENRICHMENT NEEDED: Identify stakeholder gaps -->
-

---

## Value Tracking

### Success Metrics
<!-- AI ENRICHMENT NEEDED: Define success metrics based on customer goals -->

| Metric | Current | Target | Contribution |
|--------|---------|--------|--------------|
| | | | |

### Value Delivered
<!-- AI ENRICHMENT NEEDED: Document value from meeting summaries -->

| Date | Value Statement | Source | Impact |
|------|-----------------|--------|--------|
| | | | |

### Value Gaps
<!-- AI ENRICHMENT NEEDED: Identify opportunities from conversations -->
-

---

## Technical Landscape

### Current Architecture
<!-- AI ENRICHMENT NEEDED: Extract from technical discussions -->
**Platform**:
**Key Integrations**:
**Technical Health**:

### Technical Debt & Issues
<!-- AI ENRICHMENT NEEDED: Track from support and meetings -->

| Priority | Issue | Status | Owner | Due Date |
|----------|-------|--------|-------|----------|
| | | | | |

---

## Engagement Tracking

### Meeting Cadence
**Expected Frequency**: {{MEETING_CADENCE}}
**Actual Frequency (90 days)**:
<!-- AI ENRICHMENT NEEDED: List recent meetings from calendar/transcripts -->

### Engagement Signals

| Signal Type | Assessment | Evidence |
|-------------|------------|----------|
| Support Ticket Volume | | |
| Feature Adoption | | |
| Proactive Engagement | | |
| Executive Engagement | | |

---

## Commercial Information

### Contract Details
- **Current ARR:** {{ARR}}
- **Last Renewal:** {{LAST_RENEWAL}}
- **Next Renewal:** {{RENEWAL_DATE}}
- **Contract Type:** {{SUPPORT_PACKAGE}}

### Growth Opportunities
<!-- AI ENRICHMENT NEEDED: Extract expansion signals -->

| Opportunity | Value | Probability | Timeline | Next Step |
|-------------|-------|-------------|----------|-----------|
| | | | | |

### Renewal Strategy
- **Days Until Renewal:** {{DAYS_TO_RENEWAL}}
- **Renewal Confidence:** <!-- AI ENRICHMENT NEEDED -->
- **Key Risks:** <!-- AI ENRICHMENT NEEDED -->
- **Key Opportunities:** <!-- AI ENRICHMENT NEEDED -->

---

## Success Plan

**Active Success Plan:** Create at:
`Accounts/{{ACCOUNT_NAME}}/01-Customer-Information/{{ACCOUNT_SLUG}}-success-plan.md`

- **Current Objectives:** To be defined
- **Last Updated:** Not yet created
- **Next Review:** TBD

---

## Historical Context

### Key Events Timeline
<!-- AI ENRICHMENT NEEDED: Extract from meeting summaries -->

| Date | Event | Impact | Source |
|------|-------|--------|--------|
| | | | |

### Escalations/Issues History
<!-- AI ENRICHMENT NEEDED: Track incidents and escalations -->

| Date | Issue | Resolution | Impact |
|------|-------|------------|--------|
| | | | |

---

## Resources & Links

### Internal Resources
- **Account Folder**: `Accounts/{{ACCOUNT_NAME}}/`
- **Account Index**: [00-Index.md](../00-Index.md)

### Recent Meetings (Last 90 Days)
<!-- AI ENRICHMENT NEEDED: Link to meeting files -->
-

---

## Account Team Notes

### Open Questions for Discovery
<!-- AI ENRICHMENT NEEDED: Extract questions from meeting notes -->
1.

### Internal Alignment Notes
-

### Personal Relationship Notes
-

---

## Review Checklists

### Monthly Review Checklist
- [ ] Review any touchpoints this month
- [ ] Update dashboard with new insights
- [ ] Check engagement health signals
- [ ] Update CRM/system of record with changes

### Quarterly Review Checklist
- [ ] Conduct strategic touchpoint
- [ ] Review and update success plan objectives
- [ ] Assess account health status
- [ ] Update renewal strategy if needed
- [ ] Document value delivered this quarter
- [ ] Plan next quarter's focus

---

**📝 GENERATION NOTES:**
- Dashboard generated from: {{CSV_SOURCE}}
- Deterministic sections populated: Header, Quick View, Commercial
- Sections needing AI enrichment marked with <!-- AI ENRICHMENT NEEDED -->
- **Next Steps:**
  1. Ask Claude to enrich this dashboard from available transcripts
  2. Create success plan document
  3. Add stakeholder details
  4. Document recent meetings

*Generated: {{TIMESTAMP}} by generate_account_dashboard.py*
'''


def generate_dashboard(
    account: Dict[str, str],
    template: str,
    workspace: Path,
    csv_source: str = "manual"
) -> str:
    """Generate dashboard content for an account."""
    today = datetime.now()

    # Extract values with fallbacks
    account_name = account.get('Account', 'Unknown Account')
    account_slug = slugify(account_name)

    # Parse dates
    renewal_date = parse_date(account.get('Next Renewal Date', ''))
    last_engagement = parse_date(account.get('Last Engagement Date', ''))
    last_renewal = parse_date(account.get('Last Renewal Date', ''))

    # Build replacements
    replacements = {
        '{{ACCOUNT_NAME}}': account_name,
        '{{ACCOUNT_SLUG}}': account_slug,
        '{{DATE}}': today.strftime('%Y-%m-%d'),
        '{{TIMESTAMP}}': today.strftime('%Y-%m-%d %H:%M:%S'),
        '{{ACCOUNT_OWNER}}': account.get('Account Owner', 'Not assigned'),
        '{{STATUS}}': account.get('Status', 'Active'),
        '{{STATUS_EMOJI}}': get_status_emoji(
            account.get('Status', ''),
            account.get('Health Score', ''),
            account.get('Churn Risk', '')
        ),
        '{{ARR}}': format_currency(account.get('ARR', '') or account.get('Contract Value', '')),
        '{{HEALTH_SCORE}}': account.get('Health Score', 'Not tracked'),
        '{{MEETING_CADENCE}}': account.get('Meeting Cadence', 'Not defined'),
        '{{SUPPORT_PACKAGE}}': account.get('Support Package', '') or account.get('Tier', 'Standard'),
        '{{PRIMARY_CONTACT}}': account.get('Primary Contact', 'Not identified'),
        '{{PRIMARY_EMAIL}}': account.get('Primary Email', ''),
        '{{RENEWAL_DATE}}': renewal_date.strftime('%Y-%m-%d') if renewal_date else 'Not set',
        '{{DAYS_TO_RENEWAL}}': days_until(renewal_date),
        '{{LAST_ENGAGEMENT}}': last_engagement.strftime('%Y-%m-%d') if last_engagement else 'Not recorded',
        '{{LAST_RENEWAL}}': last_renewal.strftime('%Y-%m-%d') if last_renewal else 'Not recorded',
        '{{CSV_SOURCE}}': csv_source,
        '{{NOTES}}': account.get('Notes', ''),
    }

    # Apply replacements
    content = template
    for placeholder, value in replacements.items():
        content = content.replace(placeholder, value)

    return content


def get_account_path(account_name: str, workspace: Path) -> Path:
    """Get the path for an account's dashboard file."""
    slug = slugify(account_name)
    # Standard structure: Accounts/AccountName/01-Customer-Information/slug-account-dashboard.md
    return workspace / 'Accounts' / account_name / '01-Customer-Information' / f'{slug}-account-dashboard.md'


def create_account_structure(account_name: str, workspace: Path, role: str = 'key_accounts') -> List[Path]:
    """Create the directory structure for an account."""
    from steps.directories import get_account_subdirectories

    account_path = workspace / 'Accounts' / account_name
    created = []

    # Get subdirectories for the role
    try:
        subdirs = get_account_subdirectories(role)
    except ImportError:
        # Fallback if running standalone
        subdirs = [
            '00-Index.md',
            '01-Customer-Information',
            '02-Meetings',
            '03-Call-Transcripts',
            '04-Action-Items',
            '_attachments',
        ]

    for subdir in subdirs:
        if subdir.endswith('.md'):
            # It's a file
            file_path = account_path / subdir
            if not file_path.exists():
                file_path.parent.mkdir(parents=True, exist_ok=True)
                if subdir == '00-Index.md':
                    with open(file_path, 'w') as f:
                        f.write(f'# {account_name}\n\nAccount overview and navigation.\n')
                else:
                    with open(file_path, 'w') as f:
                        f.write(f'# {subdir.replace(".md", "")}\n\n')
                created.append(file_path)
        else:
            # It's a directory
            dir_path = account_path / subdir
            if not dir_path.exists():
                dir_path.mkdir(parents=True, exist_ok=True)
                created.append(dir_path)

    return created


def main():
    parser = argparse.ArgumentParser(
        description='Generate account dashboards from CSV data.',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )

    parser.add_argument(
        '--csv',
        type=str,
        help='Path to CSV file with account data'
    )
    parser.add_argument(
        '--account',
        type=str,
        help='Generate dashboard for a specific account (by name)'
    )
    parser.add_argument(
        '--all',
        action='store_true',
        help='Generate dashboards for all accounts in CSV'
    )
    parser.add_argument(
        '--workspace',
        type=str,
        default='.',
        help='Workspace root directory (default: current directory)'
    )
    parser.add_argument(
        '--dry-run',
        action='store_true',
        help='Show what would be created without writing files'
    )
    parser.add_argument(
        '--force',
        action='store_true',
        help='Overwrite existing dashboards'
    )
    parser.add_argument(
        '--create-structure',
        action='store_true',
        help='Also create account directory structure'
    )

    args = parser.parse_args()

    workspace = Path(args.workspace).resolve()

    # Determine CSV path
    if args.csv:
        csv_path = Path(args.csv)
        if not csv_path.is_absolute():
            csv_path = workspace / csv_path
    else:
        csv_path = workspace / DEFAULT_CSV_PATH

    # Load template
    template = load_template(workspace)

    # Load accounts from CSV
    try:
        accounts = load_csv(csv_path)
        print(f"Loaded {len(accounts)} accounts from {csv_path}")
    except FileNotFoundError:
        if args.account:
            # Create a blank account entry
            accounts = [{'Account': args.account}]
            print(f"CSV not found, creating blank dashboard for: {args.account}")
        else:
            print(f"Error: CSV file not found: {csv_path}")
            print(f"Create one at {DEFAULT_CSV_PATH} or specify --csv path")
            sys.exit(1)

    # Filter accounts if specific one requested
    if args.account:
        accounts = [a for a in accounts if a.get('Account', '').lower() == args.account.lower()]
        if not accounts:
            # Create blank entry
            accounts = [{'Account': args.account}]
            print(f"Account not found in CSV, creating blank dashboard")

    if not accounts:
        print("No accounts to process")
        sys.exit(1)

    # Process each account
    created_count = 0
    skipped_count = 0

    for account in accounts:
        account_name = account.get('Account', 'Unknown')
        dashboard_path = get_account_path(account_name, workspace)

        # Check if exists
        if dashboard_path.exists() and not args.force:
            print(f"  Skipping {account_name} - dashboard exists (use --force to overwrite)")
            skipped_count += 1
            continue

        # Create directory structure if requested
        if args.create_structure:
            create_account_structure(account_name, workspace)

        # Generate content
        content = generate_dashboard(
            account,
            template,
            workspace,
            csv_source=str(csv_path)
        )

        if args.dry_run:
            print(f"  Would create: {dashboard_path}")
            print(f"  Content preview (first 500 chars):")
            print(content[:500])
            print("---")
        else:
            # Ensure directory exists
            dashboard_path.parent.mkdir(parents=True, exist_ok=True)

            # Write file
            with open(dashboard_path, 'w') as f:
                f.write(content)

            print(f"  Created: {dashboard_path}")
            created_count += 1

    # Summary
    print()
    print(f"Summary: {created_count} created, {skipped_count} skipped")

    if created_count > 0:
        print()
        print("Next steps:")
        print("1. Ask Claude to enrich dashboards from available transcripts")
        print("2. Create success plans for each account")
        print("3. Add stakeholder details and relationship notes")


if __name__ == '__main__':
    main()
