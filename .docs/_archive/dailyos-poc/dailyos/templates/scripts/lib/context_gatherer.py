#!/usr/bin/env python3
"""
Context gathering utilities for directive reference approach.

Instead of embedding full file contents into the directive JSON,
context gatherers produce:
- inline_metrics: Small key metrics that go directly into the directive
- refs: File paths that Claude reads during Phase 2 enrichment

This keeps directive size small while providing rich context.
"""

import os
import sys
from pathlib import Path
from typing import Dict, List, Any, Optional

# Add lib to path for sibling imports
sys.path.insert(0, str(Path(__file__).parent))

from file_utils import (
    find_account_dashboard, find_recent_meeting_summaries,
    find_account_action_file, get_file_age_days, VIP_ROOT
)


class CSMContextGatherer:
    """
    Gathers meeting context for Customer Success Manager profile.

    Produces rich context with account metrics inline and deep
    references to dashboards, stakeholder maps, meeting history,
    and account actions.
    """

    def __init__(self, account_lookup: Dict[str, Dict[str, Any]] = None):
        self.account_lookup = account_lookup or {}

    def gather_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """
        Gather context for a classified meeting.

        Args:
            meeting: Classification result from classify_meeting()

        Returns:
            Dictionary with inline_metrics and refs
        """
        meeting_type = meeting.get('type', 'unknown')

        if meeting_type in ('customer', 'qbr'):
            return self._gather_customer_context(meeting)
        elif meeting_type == 'training':
            return self._gather_training_context(meeting)
        elif meeting_type == 'partnership':
            return self._gather_partnership_context(meeting)
        elif meeting_type in ('internal', 'team_sync'):
            return self._gather_internal_context(meeting)
        elif meeting_type == 'one_on_one':
            return self._gather_one_on_one_context(meeting)
        elif meeting_type == 'external':
            return self._gather_external_context(meeting)
        else:
            return {'inline_metrics': {}, 'refs': {}}

    def _gather_customer_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """Gather full context for customer/QBR meetings."""
        account = meeting.get('account', '')
        account_data = self.account_lookup.get(account, {})

        inline_metrics = {
            'account': account,
            'arr': account_data.get('arr'),
            'tier': account_data.get('tier'),
            'renewal': account_data.get('renewal'),
            'last_engagement': account_data.get('last_engagement'),
            'cadence': account_data.get('cadence'),
        }

        refs = {}

        # Account dashboard
        dashboard = find_account_dashboard(account)
        if dashboard:
            refs['account_dashboard'] = str(dashboard)
            inline_metrics['dashboard_age_days'] = get_file_age_days(dashboard)

        # Meeting history — more lookback for QBR
        lookback = 3 if meeting.get('type') == 'qbr' else 2
        recent = find_recent_meeting_summaries(account, limit=lookback)
        if recent:
            refs['meeting_history'] = [str(p) for p in recent]

        # Stakeholder map
        stakeholder_path = _find_account_file(account, 'stakeholders.md')
        if stakeholder_path:
            refs['stakeholder_map'] = str(stakeholder_path)

        # Account actions
        action_file = find_account_action_file(account)
        if action_file:
            refs['account_actions'] = str(action_file)

        # Strategic programs (for QBR)
        if meeting.get('type') == 'qbr':
            programs_path = _find_account_file(account, 'programs.md')
            strategy_path = _find_account_file(account, 'strategy.md')
            if programs_path:
                refs['strategic_programs'] = str(programs_path)
            elif strategy_path:
                refs['strategic_programs'] = str(strategy_path)

        return {'inline_metrics': inline_metrics, 'refs': refs}

    def _gather_training_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """Gather context for training sessions."""
        account = meeting.get('account', '')

        inline_metrics = {'account': account} if account else {}
        refs = {}

        if account:
            dashboard = find_account_dashboard(account)
            if dashboard:
                refs['account_dashboard'] = str(dashboard)

        # Look for prior training sessions in archive
        title = meeting.get('title', '')
        recent = find_recent_meeting_summaries(title, limit=2)
        if recent:
            refs['prior_training_sessions'] = [str(p) for p in recent]

        return {'inline_metrics': inline_metrics, 'refs': refs}

    def _gather_partnership_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """Gather context for partnership meetings."""
        account = meeting.get('account', '')

        inline_metrics = {'partner': account} if account else {}
        refs = {}

        # Look for partner-related files
        recent = find_recent_meeting_summaries(account, limit=2) if account else []
        if recent:
            refs['meeting_history'] = [str(p) for p in recent]

        return {'inline_metrics': inline_metrics, 'refs': refs}

    def _gather_internal_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """Gather lightweight context for internal meetings."""
        refs = {}

        # Just last meeting with same title
        title = meeting.get('title', '')
        recent = find_recent_meeting_summaries(title, limit=1)
        if recent:
            refs['last_meeting'] = str(recent[0])

        return {'inline_metrics': {}, 'refs': refs}

    def _gather_one_on_one_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """Gather context for 1:1 meetings — kept deliberately light."""
        refs = {}

        title = meeting.get('title', '')
        recent = find_recent_meeting_summaries(title, limit=1)
        if recent:
            refs['last_meeting'] = str(recent[0])

        return {'inline_metrics': {}, 'refs': refs}

    def _gather_external_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """Gather context for external meetings with unknown attendees."""
        inline_metrics = {}
        refs = {}

        unknown_domains = meeting.get('unknown_domains', [])
        if unknown_domains:
            inline_metrics['unknown_domains'] = unknown_domains

        # Search archive for any mentions of these domains
        for domain in unknown_domains:
            archive_mentions = search_local_archive(domain)
            if archive_mentions:
                refs[f'archive_mentions_{domain}'] = archive_mentions

        return {'inline_metrics': inline_metrics, 'refs': refs}


class GeneralContextGatherer:
    """
    Gathers meeting context for General (non-CSM) profile.

    Lighter touch — no account metrics, no stakeholder maps.
    Focuses on meeting history and attendee context.
    """

    def gather_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """
        Gather context for a classified meeting.

        Args:
            meeting: Classification result from classify_meeting()

        Returns:
            Dictionary with inline_metrics and refs
        """
        meeting_type = meeting.get('type', 'unknown')

        if meeting_type in ('external', 'partnership'):
            return self._gather_external_context(meeting)
        elif meeting_type in ('internal', 'team_sync'):
            return self._gather_internal_context(meeting)
        elif meeting_type == 'one_on_one':
            return self._gather_one_on_one_context(meeting)
        else:
            return {'inline_metrics': {}, 'refs': {}}

    def _gather_external_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """Gather context for external meetings."""
        refs = {}

        title = meeting.get('title', '')
        recent = find_recent_meeting_summaries(title, limit=2)
        if recent:
            refs['meeting_history'] = [str(p) for p in recent]

        unknown_domains = meeting.get('unknown_domains', [])
        if unknown_domains:
            for domain in unknown_domains:
                archive_mentions = search_local_archive(domain)
                if archive_mentions:
                    refs[f'archive_mentions_{domain}'] = archive_mentions

        return {
            'inline_metrics': {'unknown_domains': unknown_domains} if unknown_domains else {},
            'refs': refs,
        }

    def _gather_internal_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """Gather context for internal meetings."""
        refs = {}

        title = meeting.get('title', '')
        recent = find_recent_meeting_summaries(title, limit=1)
        if recent:
            refs['last_meeting'] = str(recent[0])

        return {'inline_metrics': {}, 'refs': refs}

    def _gather_one_on_one_context(self, meeting: Dict[str, Any]) -> Dict[str, Any]:
        """Gather context for 1:1 meetings."""
        refs = {}

        title = meeting.get('title', '')
        recent = find_recent_meeting_summaries(title, limit=1)
        if recent:
            refs['last_meeting'] = str(recent[0])

        return {'inline_metrics': {}, 'refs': refs}


def get_context_gatherer(profile: str, account_lookup: Dict = None):
    """
    Factory function to get the right context gatherer for the profile.

    Args:
        profile: "customer-success" or "general"
        account_lookup: Account data (only needed for CS profile)

    Returns:
        Context gatherer instance
    """
    if profile == "customer-success":
        return CSMContextGatherer(account_lookup=account_lookup)
    return GeneralContextGatherer()


def search_local_archive(
    query: str,
    lookback_days: int = 90,
    max_results: int = 10,
) -> List[str]:
    """
    Search the local archive for mentions of a query string.

    Searches both _archive/ and _inbox/ directories for files
    containing the query. Used for unknown meeting research (2.0f)
    and external context gathering.

    Args:
        query: Search term (domain name, person name, company name)
        lookback_days: How many days back to search
        max_results: Maximum number of file paths to return

    Returns:
        List of file paths containing the query
    """
    matches = []
    archive_dir = VIP_ROOT / "_archive"
    inbox_dir = VIP_ROOT / "_inbox"

    search_dirs = []
    if archive_dir.exists():
        search_dirs.append(archive_dir)
    if inbox_dir.exists():
        search_dirs.append(inbox_dir)

    query_lower = query.lower()

    for search_dir in search_dirs:
        try:
            for root, _dirs, files in os.walk(search_dir):
                for fname in files:
                    if len(matches) >= max_results:
                        return matches

                    fpath = Path(root) / fname

                    # Only search text files
                    if fpath.suffix not in ('.md', '.txt', '.json', '.csv'):
                        continue

                    # Check file age
                    try:
                        age = get_file_age_days(fpath)
                        if age > lookback_days:
                            continue
                    except (OSError, ValueError):
                        continue

                    # Search file content
                    try:
                        content = fpath.read_text(errors='ignore').lower()
                        if query_lower in content:
                            matches.append(str(fpath))
                    except (OSError, UnicodeDecodeError):
                        continue
        except OSError:
            continue

    return matches


def build_research_context(
    meeting: Dict[str, Any],
    attendees: List[str] = None,
) -> Dict[str, Any]:
    """
    Build research context for unknown external meetings (2.0f).

    When a meeting has external attendees that don't match any known
    account or partner, this builds a research brief for Claude.

    Args:
        meeting: Classification result with unknown_domains
        attendees: Full attendee list with email addresses

    Returns:
        Research context dictionary
    """
    unknown_domains = meeting.get('unknown_domains', [])
    attendees = attendees or []

    research = {
        'local_search_performed': True,
        'archive_mentions': [],
        'inbox_threads': [],
        'company_domains': unknown_domains,
        'attendee_names': [],
        'attendee_emails': [],
    }

    # Extract attendee info for unknown domains
    for email in attendees:
        # Handle "Name <email@domain.com>" format
        if '<' in email and '>' in email:
            name_part = email.split('<')[0].strip()
            email_part = email.split('<')[1].split('>')[0]
        elif '@' in email:
            name_part = ''
            email_part = email
        else:
            continue

        domain = email_part.split('@')[1].lower() if '@' in email_part else ''

        if domain in unknown_domains:
            if name_part:
                research['attendee_names'].append(name_part)
            research['attendee_emails'].append(email_part)

    # Search archive for each domain
    for domain in unknown_domains:
        mentions = search_local_archive(domain, lookback_days=90, max_results=5)
        research['archive_mentions'].extend(mentions)

    # Search archive for each attendee name
    for name in research['attendee_names']:
        if len(name) > 2:  # Skip very short names
            mentions = search_local_archive(name, lookback_days=90, max_results=3)
            research['archive_mentions'].extend(mentions)

    # Deduplicate
    research['archive_mentions'] = list(set(research['archive_mentions']))

    # Search inbox for related files
    inbox_dir = VIP_ROOT / "_inbox"
    if inbox_dir.exists():
        for domain in unknown_domains:
            inbox_matches = search_local_archive(
                domain,
                lookback_days=30,
                max_results=5,
            )
            # Filter to only inbox paths
            inbox_matches = [m for m in inbox_matches if '_inbox' in m]
            research['inbox_threads'].extend(inbox_matches)

    research['inbox_threads'] = list(set(research['inbox_threads']))

    return research


def _find_account_file(account: str, filename: str) -> Optional[Path]:
    """
    Find a specific file in an account's directory.

    Searches Accounts/{account}/ for the given filename.

    Args:
        account: Account name
        filename: File to find (e.g. 'stakeholders.md')

    Returns:
        Path if found, None otherwise
    """
    accounts_dir = VIP_ROOT / "Accounts"
    if not accounts_dir.exists():
        return None

    # Try exact match first
    direct = accounts_dir / account / filename
    if direct.exists():
        return direct

    # Try case-insensitive search
    try:
        for item in accounts_dir.iterdir():
            if item.is_dir() and item.name.lower() == account.lower():
                target = item / filename
                if target.exists():
                    return target
    except OSError:
        pass

    return None
