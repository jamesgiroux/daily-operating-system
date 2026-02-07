"""Email fetch and classification for DailyOS workflows.

Extracted from prepare_today.py per ADR-0030.
Only used by the /today orchestrator (week doesn't fetch emails).
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from .config import (
    BULK_SENDER_DOMAINS,
    HIGH_PRIORITY_SUBJECT_KEYWORDS,
    LOW_PRIORITY_SIGNALS,
    NOREPLY_LOCAL_PARTS,
    build_gmail_service,
    _warn,
)


@dataclass
class EmailResult:
    """Result of fetching and classifying emails."""
    all_emails: list[dict[str, Any]] = field(default_factory=list)
    high: list[dict[str, Any]] = field(default_factory=list)
    medium_count: int = 0
    low_count: int = 0


def fetch_and_classify_emails(
    customer_domains: set[str],
    user_domain: str,
    account_hints: set[str] | None = None,
    max_results: int = 30,
) -> EmailResult:
    """Fetch unread emails from Gmail, classify by priority tier.

    Args:
        customer_domains: Domains of today's customer meeting attendees.
        user_domain: User's own email domain.
        account_hints: Lowercased slugs of known customer accounts.
        max_results: Maximum emails to fetch.

    Returns:
        EmailResult with classified emails and counts.
    """
    result = EmailResult()
    raw_emails = _fetch_unread_emails(max_results)

    for email in raw_emails:
        priority = _classify_email_priority(
            email, customer_domains, user_domain, account_hints,
        )
        from_raw = email.get("from", "")
        email_obj: dict[str, Any] = {
            "id": email.get("id"),
            "thread_id": email.get("thread_id"),
            "from": from_raw,
            "from_email": _extract_email_address(from_raw),
            "subject": email.get("subject"),
            "snippet": email.get("snippet"),
            "date": email.get("date"),
            "priority": priority,
        }
        result.all_emails.append(email_obj)
        if priority == "high":
            result.high.append(email_obj)
        elif priority == "medium":
            result.medium_count += 1
        else:
            result.low_count += 1

    return result


# ---------------------------------------------------------------------------
# Gmail API
# ---------------------------------------------------------------------------

def _fetch_unread_emails(max_results: int = 30) -> list[dict[str, Any]]:
    """Fetch unread emails from the last 24 hours."""
    service = build_gmail_service()
    if service is None:
        return []

    try:
        results = (
            service.users()
            .messages()
            .list(
                userId="me",
                q="is:unread newer_than:1d",
                maxResults=max_results,
            )
            .execute()
        )

        messages = results.get("messages", [])
        if not messages:
            return []

        emails: list[dict[str, Any]] = []
        for msg_stub in messages:
            try:
                msg = (
                    service.users()
                    .messages()
                    .get(
                        userId="me",
                        id=msg_stub["id"],
                        format="metadata",
                        metadataHeaders=[
                            "From", "Subject", "Date",
                            "List-Unsubscribe", "Precedence",
                        ],
                    )
                    .execute()
                )

                headers = {
                    h["name"]: h["value"]
                    for h in msg.get("payload", {}).get("headers", [])
                }

                emails.append({
                    "id": msg.get("id", ""),
                    "thread_id": msg.get("threadId", ""),
                    "from": headers.get("From", ""),
                    "subject": headers.get("Subject", ""),
                    "snippet": msg.get("snippet", ""),
                    "date": headers.get("Date", ""),
                    "list_unsubscribe": headers.get("List-Unsubscribe", ""),
                    "precedence": headers.get("Precedence", ""),
                })
            except Exception:
                continue

        return emails

    except Exception as exc:
        _warn(f"Gmail API error: {exc}")
        return []


# ---------------------------------------------------------------------------
# Email classification
# ---------------------------------------------------------------------------

def _extract_email_address(from_field: str) -> str:
    """Extract bare email from a 'From' header like 'Name <email@example.com>'."""
    if "<" in from_field and ">" in from_field:
        return from_field.split("<")[1].split(">")[0].lower()
    return from_field.strip().lower()


def _extract_domain(email_addr: str) -> str:
    """Extract domain from an email address."""
    if "@" in email_addr:
        return email_addr.split("@")[1].lower()
    return ""


def _classify_email_priority(
    email: dict[str, Any],
    customer_domains: set[str],
    user_domain: str,
    account_hints: set[str] | None = None,
) -> str:
    """Classify email priority: 'high', 'medium', or 'low'.

    High: from customer domains, from known account domains, or subject
          contains urgency keywords.
    Medium: from internal colleagues, or meeting-related.
    Low: newsletters, automated, GitHub notifications.
    """
    from_raw = email.get("from", "")
    from_addr = _extract_email_address(from_raw)
    domain = _extract_domain(from_addr)
    subject_lower = email.get("subject", "").lower()

    # HIGH: Customer domains (from today's meeting attendees)
    if domain in customer_domains:
        return "high"

    # HIGH: Sender domain matches a known customer account
    if account_hints and domain:
        domain_base = domain.split(".")[0]
        for hint in account_hints:
            if hint == domain_base or (len(hint) >= 4 and hint in domain_base):
                return "high"

    # HIGH: Urgency keywords in subject
    if any(kw in subject_lower for kw in HIGH_PRIORITY_SUBJECT_KEYWORDS):
        return "high"

    # LOW: Newsletters, automated, GitHub
    from_lower = from_raw.lower()
    if any(signal in from_lower or signal in subject_lower for signal in LOW_PRIORITY_SIGNALS):
        return "low"
    if "github.com" in domain:
        return "low"

    # LOW: List-Unsubscribe header present (bulk/marketing mail) — I21
    if email.get("list_unsubscribe"):
        return "low"

    # LOW: Precedence: bulk or list — I21
    precedence = email.get("precedence", "").lower()
    if precedence in ("bulk", "list"):
        return "low"

    # LOW: Sender domain is a known bulk/marketing sender — I21
    if domain in BULK_SENDER_DOMAINS:
        return "low"

    # LOW: Noreply local-part (checked AFTER customer/account domain) — I21
    local_part = from_addr.split("@")[0] if "@" in from_addr else ""
    if local_part in NOREPLY_LOCAL_PARTS:
        return "low"

    # MEDIUM: Internal colleagues
    if user_domain and domain == user_domain:
        return "medium"

    # MEDIUM: Meeting-related
    if any(kw in subject_lower for kw in ("meeting", "calendar", "invite")):
        return "medium"

    return "medium"
