#!/usr/bin/env python3
"""
Meeting classification utilities for daily operating system scripts.
Handles meeting type detection, domain mapping, and prep requirements.
"""

import json
import subprocess
import sys
from pathlib import Path
from typing import Dict, List, Optional, Any, Tuple

# Standard paths
VIP_ROOT = Path(__file__).parent.parent.parent
ACCOUNTS_MAPPING_FILE = VIP_ROOT / "_tools/accounts-mapping.json"
BU_CACHE_FILE = VIP_ROOT / "_reference/bu-classification-cache.json"
GOOGLE_API_PATH = VIP_ROOT / ".config/google/google_api.py"
WORKSPACE_CONFIG_PATH = VIP_ROOT / "_config/workspace.json"

# Sheet ID for account data
ACCOUNT_SHEET_ID = "1edLlG0rkPj9QRT5mWQmCh_L-qy4We9fBLJ4haMZ_14g"


def get_internal_domains() -> set:
    """
    Load internal email domains from workspace config.
    Falls back to empty set if config not found.
    """
    if WORKSPACE_CONFIG_PATH.exists():
        try:
            with open(WORKSPACE_CONFIG_PATH) as f:
                config = json.load(f)
            domains = config.get("organization", {}).get("internal_domains", [])
            return set(d.lower() for d in domains)
        except (json.JSONDecodeError, IOError):
            pass
    return set()


# Internal email domains (lazy-loaded from config)
_internal_domains_cache = None


def _get_internal_domains() -> set:
    """Get internal domains, caching the result."""
    global _internal_domains_cache
    if _internal_domains_cache is None:
        _internal_domains_cache = get_internal_domains()
    return _internal_domains_cache

# Known project configurations
KNOWN_PROJECTS = {
    'agentforce': {
        'title_keywords': ['agentforce', 'vip <> salesforce', 'salesforce connector'],
        'partner_domains': {'salesforce.com', 'rtcamp.com'},
        'location': 'Projects/Agentforce'
    }
}

# Multi-BU parent companies
MULTI_BU_DOMAINS = {
    'salesforce.com': {
        'parent': 'Salesforce',
        'bus': ['Digital-Marketing-Technology', 'AppExchange', 'Availability-Infrastructure-Engineering',
                'Engineering', 'Office-of-Accessibility', 'Q-Branch-Team', 'Talent-Experience', 'Ventures'],
        'default': 'Digital-Marketing-Technology'
    },
    'hilton.com': {
        'parent': 'Hilton',
        'bus': ['Domestic', 'B2B-Engagement', 'Careers', 'Corporate', 'Newsroom'],
        'default': 'Domestic'
    },
    'coxautoinc.com': {
        'parent': 'Cox',
        'bus': ['Corporate-Services-B2B', 'Consumer-Brands', 'Diversification', 'Enterprises'],
        'default': 'Corporate-Services-B2B'
    },
    'intuit.com': {
        'parent': 'Intuit',
        'bus': ['Credit-Karma', 'Intuit'],
        'default': 'Credit-Karma'
    }
}


def load_domain_mapping() -> Dict[str, str]:
    """
    Load domain to account mapping from accounts-mapping.json.

    Returns:
        Dictionary mapping email domains to account names
    """
    if ACCOUNTS_MAPPING_FILE.exists():
        try:
            with open(ACCOUNTS_MAPPING_FILE) as f:
                data = json.load(f)
                return data.get('domain_to_account', {})
        except Exception as e:
            print(f"Warning: Failed to load accounts mapping: {e}", file=sys.stderr)

    return {}


def load_bu_cache() -> Dict[str, Any]:
    """
    Load BU classification cache for multi-BU accounts.

    Returns:
        BU cache dictionary
    """
    if BU_CACHE_FILE.exists():
        try:
            with open(BU_CACHE_FILE) as f:
                return json.load(f)
        except Exception:
            pass

    return {'mappings': [], 'default_bus': {}}


def save_bu_cache(cache: Dict[str, Any]) -> None:
    """
    Save BU classification cache.

    Args:
        cache: Cache dictionary to save
    """
    try:
        BU_CACHE_FILE.parent.mkdir(parents=True, exist_ok=True)
        with open(BU_CACHE_FILE, 'w') as f:
            json.dump(cache, f, indent=2)
    except Exception as e:
        print(f"Warning: Failed to save BU cache: {e}", file=sys.stderr)


def fetch_account_data() -> Optional[List[List[str]]]:
    """
    Fetch account data from Google Sheet.

    Returns:
        Sheet data as list of rows, or None if failed
    """
    try:
        result = subprocess.run(
            ["python3", str(GOOGLE_API_PATH), "sheets", "get", ACCOUNT_SHEET_ID, "A1:AB50"],
            capture_output=True,
            text=True,
            timeout=30
        )

        if result.returncode != 0:
            print(f"Warning: Sheet fetch failed: {result.stderr}", file=sys.stderr)
            return None

        data = json.loads(result.stdout)
        return data.get('values', [])

    except Exception as e:
        print(f"Warning: Failed to fetch account data: {e}", file=sys.stderr)
        return None


def build_account_lookup(sheet_data: List[List[str]]) -> Dict[str, Dict[str, Any]]:
    """
    Build account lookup dictionary from sheet data.

    Args:
        sheet_data: Raw sheet data (list of rows)

    Returns:
        Dictionary mapping account names to their data
    """
    if not sheet_data or len(sheet_data) < 2:
        return {}

    lookup = {}
    headers = sheet_data[0] if sheet_data else []

    # Column mappings (0-indexed)
    col_map = {
        'account': 0,      # A
        'ring': 3,         # D
        'last_engagement': 5,  # F
        'arr': 8,          # I
        'renewal': 15,     # P
        'cadence': 23,     # X
        'success_plan': 24,    # Y
        'success_plan_updated': 25,  # Z
        'email_domain': 27,  # AB
    }

    for row in sheet_data[1:]:
        if not row:
            continue

        account = row[0] if len(row) > 0 else None
        if not account:
            continue

        lookup[account] = {
            'account': account,
            'ring': row[col_map['ring']] if len(row) > col_map['ring'] else None,
            'last_engagement': row[col_map['last_engagement']] if len(row) > col_map['last_engagement'] else None,
            'arr': row[col_map['arr']] if len(row) > col_map['arr'] else None,
            'renewal': row[col_map['renewal']] if len(row) > col_map['renewal'] else None,
            'cadence': row[col_map['cadence']] if len(row) > col_map['cadence'] else None,
            'success_plan': row[col_map['success_plan']] if len(row) > col_map['success_plan'] else None,
            'success_plan_updated': row[col_map['success_plan_updated']] if len(row) > col_map['success_plan_updated'] else None,
            'email_domain': row[col_map['email_domain']] if len(row) > col_map['email_domain'] else None,
        }

    return lookup


def extract_domains_from_attendees(attendees: List[str]) -> Tuple[set, set]:
    """
    Extract internal and external domains from attendee list.

    Args:
        attendees: List of email addresses (can be "email@domain.com" or "Name <email@domain.com>")

    Returns:
        Tuple of (internal_domains, external_domains)
    """
    internal = set()
    external = set()

    for email in attendees:
        if '@' not in email:
            continue

        # Handle "Name <email@domain.com>" format
        if '<' in email and '>' in email:
            # Extract just the email part
            email = email.split('<')[1].split('>')[0]

        domain = email.split('@')[1].lower().strip()

        if domain in _get_internal_domains():
            internal.add(domain)
        else:
            external.add(domain)

    return internal, external


def check_project_match(title: str, external_domains: set) -> Optional[Dict[str, Any]]:
    """
    Check if a meeting matches a known project.

    Args:
        title: Meeting title
        external_domains: Set of external domains in attendees

    Returns:
        Project info dictionary if matched, None otherwise
    """
    title_lower = title.lower()

    for project_name, config in KNOWN_PROJECTS.items():
        # Check title keywords
        if any(keyword in title_lower for keyword in config['title_keywords']):
            return {
                'type': 'project',
                'project': project_name.title(),
                'location': config['location']
            }

        # Check partner domains
        if config['partner_domains'] & external_domains:
            # Only match if title also suggests project context
            if any(keyword in title_lower for keyword in config['title_keywords']):
                return {
                    'type': 'project',
                    'project': project_name.title(),
                    'location': config['location']
                }

    return None


def classify_meeting(event: Dict[str, Any], domain_mapping: Dict[str, str] = None,
                     bu_cache: Dict[str, Any] = None) -> Dict[str, Any]:
    """
    Classify a meeting by type and determine prep requirements.

    Args:
        event: Calendar event dictionary
        domain_mapping: Domain to account mapping (optional)
        bu_cache: BU classification cache (optional)

    Returns:
        Classification result with type, account, prep_status, etc.
    """
    if domain_mapping is None:
        domain_mapping = load_domain_mapping()
    if bu_cache is None:
        bu_cache = load_bu_cache()

    title = event.get('summary', '')
    attendees = event.get('attendees', [])

    result = {
        'event_id': event.get('id'),
        'title': title,
        'start': event.get('start'),
        'end': event.get('end'),
        'type': 'unknown',
        'account': None,
        'project': None,
        'prep_status': None,
        'agenda_owner': None,
        'needs_bu_prompt': False,
        'bu_options': None,
    }

    # No attendees or only owner = personal
    if not attendees or len(attendees) <= 1:
        result['type'] = 'personal'
        result['prep_status'] = None
        return result

    # Extract domains
    internal_domains, external_domains = extract_domains_from_attendees(attendees)

    # Check for project match first
    project_match = check_project_match(title, external_domains)
    if project_match:
        result.update(project_match)
        result['prep_status'] = '🔄 Bring updates'
        return result

    # All internal = internal meeting
    if not external_domains:
        result['type'] = 'internal'
        result['prep_status'] = '👥 Context needed'
        return result

    # Has external attendees - classify as customer or external
    matched_accounts = set()

    for domain in external_domains:
        # Check multi-BU domains first
        if domain in MULTI_BU_DOMAINS:
            multi_bu = MULTI_BU_DOMAINS[domain]

            # Try to resolve from cache
            resolved_bu = resolve_multi_bu(domain, attendees, title, bu_cache)

            if resolved_bu:
                matched_accounts.add(f"{multi_bu['parent']} / {resolved_bu}")
            else:
                # Need user prompt
                result['needs_bu_prompt'] = True
                result['bu_options'] = {
                    'domain': domain,
                    'parent': multi_bu['parent'],
                    'bus': multi_bu['bus'],
                    'default': multi_bu['default']
                }
                # Use default for now
                matched_accounts.add(f"{multi_bu['parent']} / {multi_bu['default']}")

        # Check direct domain mapping
        elif domain in domain_mapping:
            matched_accounts.add(domain_mapping[domain])

    if len(matched_accounts) == 1:
        account = list(matched_accounts)[0]
        result['type'] = 'customer'
        result['account'] = account
        result['prep_status'], result['agenda_owner'] = determine_prep_status(title, account)

    elif len(matched_accounts) > 1:
        # Multiple accounts matched - unusual, use first
        account = list(matched_accounts)[0]
        result['type'] = 'customer'
        result['account'] = account
        result['prep_status'], result['agenda_owner'] = determine_prep_status(title, account)

    else:
        # Unknown external domain
        result['type'] = 'external'
        result['prep_status'] = '👥 Context needed'
        result['unknown_domains'] = list(external_domains)

    return result


def resolve_multi_bu(domain: str, attendees: List[str], title: str,
                     cache: Dict[str, Any]) -> Optional[str]:
    """
    Try to resolve a multi-BU domain to a specific BU.

    Args:
        domain: Email domain
        attendees: List of attendee emails
        title: Meeting title
        cache: BU classification cache

    Returns:
        BU name if resolved, None if needs prompt
    """
    # Check attendee patterns
    for mapping in cache.get('mappings', []):
        if mapping.get('domain') == domain:
            # Check attendee match
            if mapping.get('attendee_pattern'):
                for attendee in attendees:
                    if mapping['attendee_pattern'] in attendee:
                        return mapping['bu']

            # Check title match
            if mapping.get('title_pattern'):
                if mapping['title_pattern'].lower() in title.lower():
                    return mapping['bu']

    # Try default
    defaults = cache.get('default_bus', {})
    if domain in defaults:
        return defaults[domain]

    return None


def determine_prep_status(title: str, account: str = None) -> Tuple[str, Optional[str]]:
    """
    Determine prep status and agenda owner for a customer meeting.

    Args:
        title: Meeting title
        account: Account name (for ring lookup)

    Returns:
        Tuple of (prep_status, agenda_owner)
    """
    title_lower = title.lower()

    # Strategic meeting signals - always need agenda
    strategic_signals = ['renewal', 'ebr', 'qbr', 'strategic review', 'quarterly', 'executive']
    if any(signal in title_lower for signal in strategic_signals):
        return '📅 Agenda needed', 'you'

    # Default: assume customer drives unless Foundation ring
    # (Ring lookup would require account data - simplified here)
    return '📋 Prep needed', 'customer'


def format_classification_for_directive(classification: Dict[str, Any]) -> Dict[str, Any]:
    """
    Format a meeting classification for JSON directive output.

    Args:
        classification: Classification result dictionary

    Returns:
        Serializable dictionary for JSON output
    """
    return {
        'event_id': classification.get('event_id'),
        'title': classification.get('title'),
        'start': classification.get('start'),
        'end': classification.get('end'),
        'type': classification.get('type'),
        'account': classification.get('account'),
        'project': classification.get('project'),
        'prep_status': classification.get('prep_status'),
        'agenda_owner': classification.get('agenda_owner'),
        'needs_bu_prompt': classification.get('needs_bu_prompt', False),
        'bu_options': classification.get('bu_options'),
        'unknown_domains': classification.get('unknown_domains'),
    }
