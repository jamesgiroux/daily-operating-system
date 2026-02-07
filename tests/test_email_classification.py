"""Tests for email classification (I21: FYI expansion).

Validates that the priority ordering is correct:
1. Customer domain check (high) runs before noreply check (low)
2. Expanded signals correctly classify bulk/marketing emails as low
3. Existing behavior for high/medium is preserved
"""

import sys
from pathlib import Path
from unittest.mock import patch

# Add scripts/ to path so we can import ops
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

from ops.email_fetch import _classify_email_priority


def _email(
    from_addr: str = "user@example.com",
    subject: str = "Hello",
    list_unsubscribe: str = "",
    precedence: str = "",
) -> dict:
    """Build a minimal email dict for classification."""
    return {
        "from": from_addr,
        "subject": subject,
        "list_unsubscribe": list_unsubscribe,
        "precedence": precedence,
    }


# --- HIGH priority ---

def test_customer_domain_is_high():
    """Email from a customer meeting attendee domain → high."""
    email = _email(from_addr="jane@acme.com")
    assert _classify_email_priority(email, {"acme.com"}, "myco.com") == "high"


def test_account_hint_match_is_high():
    """Email from a domain matching an account slug → high."""
    email = _email(from_addr="support@bringatrailer.com")
    assert _classify_email_priority(
        email, set(), "myco.com", account_hints={"bringatrailer"}
    ) == "high"


def test_urgency_keywords_is_high():
    """Email with urgency keywords in subject → high."""
    email = _email(subject="URGENT: Contract renewal deadline")
    assert _classify_email_priority(email, set(), "myco.com") == "high"


# --- HIGH overrides LOW (ordering test) ---

def test_noreply_at_customer_domain_is_high():
    """noreply@customer.com should be high (customer check runs first)."""
    email = _email(from_addr="noreply@acme.com")
    assert _classify_email_priority(email, {"acme.com"}, "myco.com") == "high"


def test_noreply_account_hint_is_high():
    """noreply@known-account.com should be high (account hint check runs first)."""
    email = _email(from_addr="noreply@bringatrailer.com")
    assert _classify_email_priority(
        email, set(), "myco.com", account_hints={"bringatrailer"}
    ) == "high"


# --- LOW priority ---

def test_list_unsubscribe_header_is_low():
    """Email with List-Unsubscribe header → low."""
    email = _email(
        from_addr="deals@somestore.com",
        list_unsubscribe="<https://somestore.com/unsub>",
    )
    assert _classify_email_priority(email, set(), "myco.com") == "low"


def test_precedence_bulk_is_low():
    """Email with Precedence: bulk → low."""
    email = _email(from_addr="updates@someservice.com", precedence="bulk")
    assert _classify_email_priority(email, set(), "myco.com") == "low"


def test_precedence_list_is_low():
    """Email with Precedence: list → low."""
    email = _email(from_addr="updates@someservice.com", precedence="list")
    assert _classify_email_priority(email, set(), "myco.com") == "low"


def test_bulk_sender_domain_is_low():
    """Email from sendgrid.net → low."""
    email = _email(from_addr="bounce@sendgrid.net")
    assert _classify_email_priority(email, set(), "myco.com") == "low"


def test_noreply_random_domain_is_low():
    """noreply@random.com → low."""
    email = _email(from_addr="noreply@random.com")
    assert _classify_email_priority(email, set(), "myco.com") == "low"


def test_donotreply_is_low():
    """do-not-reply@random.com → low."""
    email = _email(from_addr="do-not-reply@random.com")
    assert _classify_email_priority(email, set(), "myco.com") == "low"


def test_newsletter_signal_is_low():
    """Email with 'newsletter' in from → low."""
    email = _email(from_addr="newsletter@company.com")
    assert _classify_email_priority(email, set(), "myco.com") == "low"


def test_github_is_low():
    """Email from github.com → low."""
    email = _email(from_addr="notifications@github.com")
    assert _classify_email_priority(email, set(), "myco.com") == "low"


# --- MEDIUM priority ---

def test_internal_colleague_is_medium():
    """Email from same domain → medium."""
    email = _email(from_addr="colleague@myco.com")
    assert _classify_email_priority(email, set(), "myco.com") == "medium"


def test_regular_external_is_medium():
    """Regular external email with no special signals → medium."""
    email = _email(from_addr="someone@random.com", subject="Hey, quick question")
    assert _classify_email_priority(email, set(), "myco.com") == "medium"


def test_meeting_related_is_medium():
    """Email with meeting keyword in subject → medium."""
    email = _email(from_addr="someone@random.com", subject="Meeting agenda for next week")
    assert _classify_email_priority(email, set(), "myco.com") == "medium"
