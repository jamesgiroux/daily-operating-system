#!/usr/bin/env python3
"""
Google API Helper Script for Claude Code
Handles OAuth authentication and provides CLI interface for Google services.

This script is designed to work with the Daily Operating System setup wizard.
Place credentials.json in the same directory and run 'python3 google_api.py auth' to start.

Usage:
    python3 google_api.py auth                              # Initial authentication

    # Calendar (full access)
    python3 google_api.py calendar list [days]              # List upcoming events
    python3 google_api.py calendar get <id>                 # Get event details
    python3 google_api.py calendar create <title> <start> <end> [desc]  # Create event
    python3 google_api.py calendar delete <id>              # Delete event

    # Gmail (read + draft + labels)
    python3 google_api.py gmail list [max]                  # List recent emails
    python3 google_api.py gmail get <id>                    # Get email content
    python3 google_api.py gmail search <query> [max]        # Search emails
    python3 google_api.py gmail draft <to> <subject> <body> # Create draft
    python3 google_api.py gmail labels list                 # List all labels
    python3 google_api.py gmail labels add <id> <labels>    # Add labels to message
    python3 google_api.py gmail labels remove <id> <labels> # Remove labels

    # Sheets (full access)
    python3 google_api.py sheets get <id> <range>           # Read spreadsheet data
    python3 google_api.py sheets update <id> <range> <json> # Update cells
    python3 google_api.py sheets create <title>             # Create new spreadsheet

    # Docs (full access)
    python3 google_api.py docs get <id>                     # Read document content
    python3 google_api.py docs create <title> [content]     # Create new document
    python3 google_api.py docs append <id> <content>        # Append to document

    # Drive (copy files)
    python3 google_api.py drive copy <file_id> <new_name>   # Copy a file (preserves formatting)
"""

import os
import sys
import json
import base64
from datetime import datetime, timedelta, timezone
from pathlib import Path

# Check for required dependencies
try:
    from google.oauth2.credentials import Credentials
    from google_auth_oauthlib.flow import InstalledAppFlow
    from google.auth.transport.requests import Request
    from googleapiclient.discovery import build
    from googleapiclient.errors import HttpError
    from email.mime.text import MIMEText
except ImportError:
    print("Error: Required Google API packages not installed.")
    print("Install them with: pip install google-auth-oauthlib google-api-python-client")
    sys.exit(1)

# Configuration - credentials stored securely in user home directory
# This keeps credentials out of workspaces (safer for sharing/git)
GOOGLE_DIR = Path.home() / '.dailyos' / 'google'
CREDENTIALS_FILE = GOOGLE_DIR / "credentials.json"
TOKEN_FILE = GOOGLE_DIR / "token.json"
ERROR_LOG_FILE = GOOGLE_DIR / "error.log"

# Legacy paths for migration (workspace-relative)
LEGACY_CONFIG_DIR = Path(__file__).parent
LEGACY_CREDENTIALS_FILE = LEGACY_CONFIG_DIR / "credentials.json"
LEGACY_TOKEN_FILE = LEGACY_CONFIG_DIR / "token.json"

# Error classification for actionable messages
GOOGLE_API_ERRORS = {
    400: ('Bad Request', 'Check your request parameters'),
    401: ('Unauthorized', 'Delete ~/.dailyos/google/token.json and re-authenticate with: dailyos google-setup'),
    403: ('Forbidden', 'Enable the required API in Google Cloud Console or check permissions'),
    404: ('Not Found', 'The requested resource does not exist'),
    429: ('Rate Limited', 'Too many requests. Wait a few minutes and retry'),
    500: ('Server Error', 'Google server issue. Retry in a few minutes'),
    503: ('Service Unavailable', 'Google service temporarily unavailable. Retry shortly'),
}

# Transient errors that should trigger retry
TRANSIENT_ERRORS = {429, 500, 502, 503, 504}


def log_error(operation: str, status_code: int, details: str, fix_suggestion: str = None):
    """
    Log error to persistent error log file.

    Args:
        operation: What operation was being attempted (e.g., "Calendar API - listing events")
        status_code: HTTP status code
        details: Error details from the API
        fix_suggestion: Optional suggestion for how to fix
    """
    ensure_secure_directory()

    error_name, default_fix = GOOGLE_API_ERRORS.get(status_code, ('Unknown Error', 'Check the error details'))
    fix = fix_suggestion or default_fix

    timestamp = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
    log_entry = f"""[{timestamp}] {operation} - {status_code} {error_name}
  Operation: {operation}
  Fix: {fix}
  Details: {details}

"""

    try:
        with open(ERROR_LOG_FILE, 'a') as f:
            f.write(log_entry)
    except Exception:
        pass  # Don't fail if we can't write to log

    # Also print to stderr for immediate feedback
    print(f"Error: {error_name} ({status_code})", file=sys.stderr)
    print(f"  {fix}", file=sys.stderr)


def handle_http_error(error: HttpError, operation: str) -> None:
    """
    Handle an HttpError with proper logging and user-friendly message.

    Args:
        error: The HttpError from Google API
        operation: Description of what operation was attempted
    """
    status_code = error.resp.status
    details = str(error)

    # Extract more specific error info if available
    try:
        error_content = json.loads(error.content.decode('utf-8'))
        if 'error' in error_content:
            err = error_content['error']
            details = err.get('message', details)
    except (json.JSONDecodeError, KeyError, AttributeError):
        pass

    log_error(operation, status_code, details)


def retry_on_transient_error(max_retries: int = 3, base_delay: float = 1.0):
    """
    Decorator to retry API calls on transient errors with exponential backoff.

    Args:
        max_retries: Maximum number of retry attempts
        base_delay: Base delay in seconds (doubles each retry)
    """
    import time
    import functools

    def decorator(func):
        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            last_error = None
            for attempt in range(max_retries + 1):
                try:
                    return func(*args, **kwargs)
                except HttpError as e:
                    last_error = e
                    status_code = e.resp.status

                    if status_code not in TRANSIENT_ERRORS or attempt == max_retries:
                        raise

                    delay = base_delay * (2 ** attempt)
                    print(f"Transient error ({status_code}), retrying in {delay:.1f}s... (attempt {attempt + 1}/{max_retries})", file=sys.stderr)
                    time.sleep(delay)

            raise last_error
        return wrapper
    return decorator


# Scopes for all services - can be customized during setup
# Full access scopes (default)
SCOPES_FULL = [
    'https://www.googleapis.com/auth/calendar',       # Full access to Calendar (read + write)
    'https://www.googleapis.com/auth/gmail.modify',   # Read, send, delete, manage labels on messages
    'https://www.googleapis.com/auth/gmail.compose',  # Drafts only, not send
    'https://www.googleapis.com/auth/spreadsheets',   # Full access to Sheets
    'https://www.googleapis.com/auth/documents',      # Full access to Docs
    'https://www.googleapis.com/auth/drive',          # Full access to Drive (needed to copy existing files)
]

# Read-only scopes (for cautious users)
SCOPES_READONLY = [
    'https://www.googleapis.com/auth/calendar.readonly',
    'https://www.googleapis.com/auth/gmail.readonly',
    'https://www.googleapis.com/auth/spreadsheets.readonly',
    'https://www.googleapis.com/auth/documents.readonly',
    'https://www.googleapis.com/auth/drive.readonly',
]

# Load scope preference from config or default to full
def get_scopes():
    """Load scopes from config file or default to full access."""
    config_file = GOOGLE_DIR / "config.json"
    if config_file.exists():
        try:
            with open(config_file, 'r') as f:
                config = json.load(f)
            if config.get('readonly', False):
                return SCOPES_READONLY
        except:
            pass
    return SCOPES_FULL


def ensure_secure_directory():
    """Ensure the credentials directory exists with secure permissions."""
    if not GOOGLE_DIR.exists():
        GOOGLE_DIR.mkdir(parents=True, exist_ok=True)
        os.chmod(GOOGLE_DIR, 0o700)


def migrate_legacy_credentials():
    """
    Migrate credentials from legacy workspace location to secure home directory.

    Returns True if migration occurred.
    """
    ensure_secure_directory()
    migrated = False

    # Migrate credentials.json
    if LEGACY_CREDENTIALS_FILE.exists() and not CREDENTIALS_FILE.exists():
        import shutil
        shutil.copy2(LEGACY_CREDENTIALS_FILE, CREDENTIALS_FILE)
        os.chmod(CREDENTIALS_FILE, 0o600)
        print(f"Migrated credentials.json to {CREDENTIALS_FILE}", file=sys.stderr)
        migrated = True

    # Migrate token.json
    if LEGACY_TOKEN_FILE.exists() and not TOKEN_FILE.exists():
        import shutil
        shutil.copy2(LEGACY_TOKEN_FILE, TOKEN_FILE)
        os.chmod(TOKEN_FILE, 0o600)
        print(f"Migrated token.json to {TOKEN_FILE}", file=sys.stderr)
        migrated = True

    return migrated


def get_credentials():
    """Get valid credentials, refreshing or re-authenticating as needed."""
    creds = None
    scopes = get_scopes()

    # Try migration from legacy location first
    migrate_legacy_credentials()

    if TOKEN_FILE.exists():
        creds = Credentials.from_authorized_user_file(str(TOKEN_FILE), scopes)

    if not creds or not creds.valid:
        if creds and creds.expired and creds.refresh_token:
            try:
                creds.refresh(Request())
            except Exception as e:
                log_error("Token refresh", 401, str(e), "Delete token.json and re-authenticate")
                print(f"Token refresh failed: {e}", file=sys.stderr)
                creds = None

        if not creds:
            if not CREDENTIALS_FILE.exists():
                print(f"Error: credentials.json not found at {CREDENTIALS_FILE}", file=sys.stderr)
                print("\nTo set up Google API access, run:")
                print("  dailyos google-setup")
                print("\nOr manually:")
                print("1. Go to https://console.cloud.google.com")
                print("2. Create a project or select existing one")
                print("3. Enable Calendar, Gmail, Sheets, Docs, and Drive APIs")
                print("4. Create OAuth 2.0 credentials (Desktop app)")
                print("5. Download credentials.json to ~/.dailyos/google/")
                sys.exit(1)

            flow = InstalledAppFlow.from_client_secrets_file(str(CREDENTIALS_FILE), scopes)
            creds = flow.run_local_server(port=0)

        # Save the credentials with secure permissions
        ensure_secure_directory()
        with open(TOKEN_FILE, 'w') as token:
            token.write(creds.to_json())
        os.chmod(TOKEN_FILE, 0o600)

    return creds


def cmd_auth():
    """Authenticate and store credentials."""
    print("Starting authentication flow...")
    creds = get_credentials()
    print(f"Authentication successful! Token saved to {TOKEN_FILE}")
    return True


@retry_on_transient_error(max_retries=3)
def cmd_calendar_list(days=7):
    """List upcoming calendar events."""
    creds = get_credentials()
    service = build('calendar', 'v3', credentials=creds)

    now = datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')
    end = (datetime.now(timezone.utc) + timedelta(days=int(days))).isoformat().replace('+00:00', 'Z')

    try:
        events_result = service.events().list(
            calendarId='primary',
            timeMin=now,
            timeMax=end,
            maxResults=50,
            singleEvents=True,
            orderBy='startTime'
        ).execute()
        events = events_result.get('items', [])

        if not events:
            print("No upcoming events found.")
            return

        output = []
        for event in events:
            start = event['start'].get('dateTime', event['start'].get('date'))
            output.append({
                'id': event['id'],
                'summary': event.get('summary', 'No title'),
                'start': start,
                'end': event['end'].get('dateTime', event['end'].get('date')),
                'location': event.get('location', ''),
                'attendees': [a.get('email') for a in event.get('attendees', [])],
            })

        print(json.dumps(output, indent=2))

    except HttpError as e:
        handle_http_error(e, "Calendar API")
        sys.exit(1)


def cmd_calendar_get(event_id):
    """Get details of a specific calendar event."""
    creds = get_credentials()
    service = build('calendar', 'v3', credentials=creds)

    try:
        event = service.events().get(calendarId='primary', eventId=event_id).execute()
        print(json.dumps(event, indent=2))
    except HttpError as e:
        handle_http_error(e, "Calendar API")
        sys.exit(1)


def cmd_calendar_create(summary, start_time, end_time, description=''):
    """Create a calendar event.

    Args:
        summary: Event title
        start_time: ISO format datetime (e.g., 2026-01-12T09:00:00-05:00)
        end_time: ISO format datetime
        description: Optional event description
    """
    creds = get_credentials()
    service = build('calendar', 'v3', credentials=creds)

    # Try to detect timezone from system or default to UTC
    try:
        import time
        tz_name = time.tzname[0] if time.tzname else 'UTC'
        # Map common timezone abbreviations
        tz_map = {
            'EST': 'America/New_York',
            'EDT': 'America/New_York',
            'CST': 'America/Chicago',
            'CDT': 'America/Chicago',
            'MST': 'America/Denver',
            'MDT': 'America/Denver',
            'PST': 'America/Los_Angeles',
            'PDT': 'America/Los_Angeles',
        }
        tz = tz_map.get(tz_name, 'UTC')
    except:
        tz = 'UTC'

    try:
        event = {
            'summary': summary,
            'description': description,
            'start': {
                'dateTime': start_time,
                'timeZone': tz,
            },
            'end': {
                'dateTime': end_time,
                'timeZone': tz,
            },
        }

        created_event = service.events().insert(calendarId='primary', body=event).execute()

        print(json.dumps({
            'status': 'created',
            'id': created_event['id'],
            'summary': created_event.get('summary'),
            'start': created_event['start'].get('dateTime'),
            'end': created_event['end'].get('dateTime'),
            'htmlLink': created_event.get('htmlLink')
        }, indent=2))

    except HttpError as e:
        handle_http_error(e, "Calendar API")
        sys.exit(1)


def cmd_calendar_delete(event_id):
    """Delete a calendar event."""
    creds = get_credentials()
    service = build('calendar', 'v3', credentials=creds)

    try:
        service.events().delete(calendarId='primary', eventId=event_id).execute()
        print(json.dumps({
            'status': 'deleted',
            'id': event_id
        }, indent=2))
    except HttpError as e:
        handle_http_error(e, "Calendar API")
        sys.exit(1)


def cmd_gmail_list(max_results=20):
    """List recent emails."""
    creds = get_credentials()
    service = build('gmail', 'v1', credentials=creds)

    try:
        results = service.users().messages().list(
            userId='me',
            maxResults=int(max_results),
            labelIds=['INBOX']
        ).execute()
        messages = results.get('messages', [])

        if not messages:
            print("No messages found.")
            return

        output = []
        for msg in messages:
            msg_data = service.users().messages().get(
                userId='me',
                id=msg['id'],
                format='metadata',
                metadataHeaders=['From', 'To', 'Subject', 'Date']
            ).execute()

            headers = {h['name']: h['value'] for h in msg_data.get('payload', {}).get('headers', [])}
            output.append({
                'id': msg['id'],
                'threadId': msg['threadId'],
                'snippet': msg_data.get('snippet', ''),
                'from': headers.get('From', ''),
                'to': headers.get('To', ''),
                'subject': headers.get('Subject', ''),
                'date': headers.get('Date', ''),
            })

        print(json.dumps(output, indent=2))

    except HttpError as e:
        handle_http_error(e, "Gmail API")
        sys.exit(1)


def cmd_gmail_get(message_id):
    """Get full content of an email."""
    creds = get_credentials()
    service = build('gmail', 'v1', credentials=creds)

    try:
        msg = service.users().messages().get(
            userId='me',
            id=message_id,
            format='full'
        ).execute()

        headers = {h['name']: h['value'] for h in msg.get('payload', {}).get('headers', [])}

        # Extract body
        body = ''
        payload = msg.get('payload', {})

        if 'body' in payload and payload['body'].get('data'):
            body = base64.urlsafe_b64decode(payload['body']['data']).decode('utf-8')
        elif 'parts' in payload:
            for part in payload['parts']:
                if part['mimeType'] == 'text/plain' and part['body'].get('data'):
                    body = base64.urlsafe_b64decode(part['body']['data']).decode('utf-8')
                    break

        output = {
            'id': msg['id'],
            'threadId': msg['threadId'],
            'from': headers.get('From', ''),
            'to': headers.get('To', ''),
            'subject': headers.get('Subject', ''),
            'date': headers.get('Date', ''),
            'body': body,
            'labels': msg.get('labelIds', []),
        }

        print(json.dumps(output, indent=2))

    except HttpError as e:
        handle_http_error(e, "Gmail API")
        sys.exit(1)


def cmd_gmail_draft(to, subject, body):
    """Create a draft email (does not send)."""
    creds = get_credentials()
    service = build('gmail', 'v1', credentials=creds)

    try:
        message = MIMEText(body)
        message['to'] = to
        message['subject'] = subject

        raw = base64.urlsafe_b64encode(message.as_bytes()).decode('utf-8')

        draft = service.users().drafts().create(
            userId='me',
            body={'message': {'raw': raw}}
        ).execute()

        print(json.dumps({
            'status': 'draft_created',
            'id': draft['id'],
            'message_id': draft['message']['id'],
            'note': 'Draft saved. Open Gmail to review and send.'
        }))

    except HttpError as e:
        handle_http_error(e, "Gmail API")
        sys.exit(1)


def cmd_gmail_search(query, max_results=20):
    """Search emails with Gmail query syntax."""
    creds = get_credentials()
    service = build('gmail', 'v1', credentials=creds)

    try:
        results = service.users().messages().list(
            userId='me',
            maxResults=int(max_results),
            q=query
        ).execute()
        messages = results.get('messages', [])

        if not messages:
            print("No messages found.")
            return

        output = []
        for msg in messages:
            msg_data = service.users().messages().get(
                userId='me',
                id=msg['id'],
                format='metadata',
                metadataHeaders=['From', 'To', 'Subject', 'Date']
            ).execute()

            headers = {h['name']: h['value'] for h in msg_data.get('payload', {}).get('headers', [])}
            output.append({
                'id': msg['id'],
                'threadId': msg['threadId'],
                'snippet': msg_data.get('snippet', ''),
                'from': headers.get('From', ''),
                'to': headers.get('To', ''),
                'subject': headers.get('Subject', ''),
                'date': headers.get('Date', ''),
            })

        print(json.dumps(output, indent=2))

    except HttpError as e:
        handle_http_error(e, "Gmail API")
        sys.exit(1)


def cmd_gmail_labels_list():
    """List all Gmail labels."""
    creds = get_credentials()
    service = build('gmail', 'v1', credentials=creds)

    try:
        results = service.users().labels().list(userId='me').execute()
        labels = results.get('labels', [])
        print(json.dumps(labels, indent=2))

    except HttpError as e:
        handle_http_error(e, "Gmail API")
        sys.exit(1)


def cmd_gmail_labels_add(message_id, label_ids_json):
    """Add labels to a message."""
    creds = get_credentials()
    service = build('gmail', 'v1', credentials=creds)

    try:
        label_ids = json.loads(label_ids_json)
        result = service.users().messages().modify(
            userId='me',
            id=message_id,
            body={'addLabelIds': label_ids}
        ).execute()

        print(json.dumps({
            'status': 'labels_added',
            'messageId': message_id,
            'labelIds': result.get('labelIds')
        }, indent=2))

    except HttpError as e:
        handle_http_error(e, "Gmail API")
        sys.exit(1)


def cmd_gmail_labels_remove(message_id, label_ids_json):
    """Remove labels from a message."""
    creds = get_credentials()
    service = build('gmail', 'v1', credentials=creds)

    try:
        label_ids = json.loads(label_ids_json)
        result = service.users().messages().modify(
            userId='me',
            id=message_id,
            body={'removeLabelIds': label_ids}
        ).execute()

        print(json.dumps({
            'status': 'labels_removed',
            'messageId': message_id,
            'labelIds': result.get('labelIds')
        }, indent=2))

    except HttpError as e:
        handle_http_error(e, "Gmail API")
        sys.exit(1)


def cmd_sheets_get(spreadsheet_id, range_name):
    """Get data from a Google Sheet."""
    creds = get_credentials()
    service = build('sheets', 'v4', credentials=creds)

    try:
        result = service.spreadsheets().values().get(
            spreadsheetId=spreadsheet_id,
            range=range_name
        ).execute()

        print(json.dumps(result, indent=2))

    except HttpError as e:
        handle_http_error(e, "Sheets API")
        sys.exit(1)


def cmd_sheets_update(spreadsheet_id, range_name, values_json):
    """Update data in a Google Sheet."""
    creds = get_credentials()
    service = build('sheets', 'v4', credentials=creds)

    try:
        values = json.loads(values_json)
        body = {'values': values}
        result = service.spreadsheets().values().update(
            spreadsheetId=spreadsheet_id,
            range=range_name,
            valueInputOption='USER_ENTERED',
            body=body
        ).execute()

        print(json.dumps({
            'status': 'updated',
            'updatedCells': result.get('updatedCells'),
            'updatedRange': result.get('updatedRange')
        }, indent=2))

    except HttpError as e:
        handle_http_error(e, "Sheets API")
        sys.exit(1)


def cmd_sheets_create(title):
    """Create a new Google Sheet."""
    creds = get_credentials()
    service = build('sheets', 'v4', credentials=creds)

    try:
        spreadsheet = {'properties': {'title': title}}
        result = service.spreadsheets().create(body=spreadsheet).execute()

        print(json.dumps({
            'status': 'created',
            'spreadsheetId': result.get('spreadsheetId'),
            'spreadsheetUrl': result.get('spreadsheetUrl'),
            'title': result.get('properties', {}).get('title')
        }, indent=2))

    except HttpError as e:
        handle_http_error(e, "Sheets API")
        sys.exit(1)


def cmd_docs_get(document_id):
    """Get content of a Google Doc."""
    creds = get_credentials()
    service = build('docs', 'v1', credentials=creds)

    try:
        doc = service.documents().get(documentId=document_id).execute()

        # Extract plain text from document
        content = []
        for element in doc.get('body', {}).get('content', []):
            if 'paragraph' in element:
                for elem in element['paragraph'].get('elements', []):
                    if 'textRun' in elem:
                        content.append(elem['textRun'].get('content', ''))

        output = {
            'id': doc['documentId'],
            'title': doc.get('title', ''),
            'content': ''.join(content),
        }

        print(json.dumps(output, indent=2))

    except HttpError as e:
        handle_http_error(e, "Docs API")
        sys.exit(1)


def cmd_docs_create(title, content=''):
    """Create a new Google Doc."""
    creds = get_credentials()
    service = build('docs', 'v1', credentials=creds)

    try:
        doc = service.documents().create(body={'title': title}).execute()
        doc_id = doc.get('documentId')

        # If content provided, insert it
        if content:
            requests = [{'insertText': {'location': {'index': 1}, 'text': content}}]
            service.documents().batchUpdate(documentId=doc_id, body={'requests': requests}).execute()

        print(json.dumps({
            'status': 'created',
            'documentId': doc_id,
            'title': doc.get('title'),
            'url': f"https://docs.google.com/document/d/{doc_id}/edit"
        }, indent=2))

    except HttpError as e:
        handle_http_error(e, "Docs API")
        sys.exit(1)


def cmd_docs_append(document_id, content):
    """Append content to an existing Google Doc."""
    creds = get_credentials()
    service = build('docs', 'v1', credentials=creds)

    try:
        # Get current doc to find end index
        doc = service.documents().get(documentId=document_id).execute()
        end_index = doc.get('body', {}).get('content', [])[-1].get('endIndex', 1) - 1

        requests = [{'insertText': {'location': {'index': end_index}, 'text': content}}]
        service.documents().batchUpdate(documentId=document_id, body={'requests': requests}).execute()

        print(json.dumps({
            'status': 'appended',
            'documentId': document_id,
            'title': doc.get('title')
        }, indent=2))

    except HttpError as e:
        handle_http_error(e, "Docs API")
        sys.exit(1)


def cmd_drive_copy(file_id, new_name):
    """Copy a file in Google Drive (preserves all formatting, validation, etc.)."""
    creds = get_credentials()
    service = build('drive', 'v3', credentials=creds)

    try:
        # Copy the file with the new name
        file_metadata = {'name': new_name}
        copied_file = service.files().copy(
            fileId=file_id,
            body=file_metadata
        ).execute()

        # Determine the file type and generate appropriate URL
        mime_type = copied_file.get('mimeType', '')
        file_id_new = copied_file.get('id')

        if 'spreadsheet' in mime_type:
            url = f"https://docs.google.com/spreadsheets/d/{file_id_new}/edit"
        elif 'document' in mime_type:
            url = f"https://docs.google.com/document/d/{file_id_new}/edit"
        elif 'presentation' in mime_type:
            url = f"https://docs.google.com/presentation/d/{file_id_new}/edit"
        else:
            url = f"https://drive.google.com/file/d/{file_id_new}/view"

        print(json.dumps({
            'status': 'copied',
            'fileId': file_id_new,
            'name': copied_file.get('name'),
            'mimeType': mime_type,
            'url': url
        }, indent=2))

    except HttpError as e:
        handle_http_error(e, "Drive API")
        sys.exit(1)


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    cmd = sys.argv[1]

    if cmd == 'auth':
        cmd_auth()

    elif cmd == 'calendar':
        if len(sys.argv) < 3:
            print("Usage: calendar list [days] | get <id> | create <summary> <start> <end> [desc] | delete <id>")
            sys.exit(1)
        subcmd = sys.argv[2]
        if subcmd == 'list':
            days = sys.argv[3] if len(sys.argv) > 3 else 7
            cmd_calendar_list(days)
        elif subcmd == 'get':
            if len(sys.argv) < 4:
                print("Usage: calendar get <event_id>")
                sys.exit(1)
            cmd_calendar_get(sys.argv[3])
        elif subcmd == 'create':
            if len(sys.argv) < 6:
                print("Usage: calendar create <summary> <start_time> <end_time> [description]")
                print("  Times in ISO format: 2026-01-12T09:00:00-05:00")
                sys.exit(1)
            description = sys.argv[6] if len(sys.argv) > 6 else ''
            cmd_calendar_create(sys.argv[3], sys.argv[4], sys.argv[5], description)
        elif subcmd == 'delete':
            if len(sys.argv) < 4:
                print("Usage: calendar delete <event_id>")
                sys.exit(1)
            cmd_calendar_delete(sys.argv[3])
        else:
            print(f"Unknown calendar command: {subcmd}")
            sys.exit(1)

    elif cmd == 'gmail':
        if len(sys.argv) < 3:
            print("Usage: gmail list | get | draft | search | labels")
            sys.exit(1)
        subcmd = sys.argv[2]
        if subcmd == 'list':
            max_results = sys.argv[3] if len(sys.argv) > 3 else 20
            cmd_gmail_list(max_results)
        elif subcmd == 'get':
            if len(sys.argv) < 4:
                print("Usage: gmail get <message_id>")
                sys.exit(1)
            cmd_gmail_get(sys.argv[3])
        elif subcmd == 'draft':
            if len(sys.argv) < 6:
                print("Usage: gmail draft <to> <subject> <body>")
                sys.exit(1)
            cmd_gmail_draft(sys.argv[3], sys.argv[4], sys.argv[5])
        elif subcmd == 'search':
            if len(sys.argv) < 4:
                print("Usage: gmail search <query> [max_results]")
                sys.exit(1)
            max_results = sys.argv[4] if len(sys.argv) > 4 else 20
            cmd_gmail_search(sys.argv[3], max_results)
        elif subcmd == 'labels':
            if len(sys.argv) < 4:
                print("Usage: gmail labels list | add <msg_id> <labels_json> | remove <msg_id> <labels_json>")
                sys.exit(1)
            labels_cmd = sys.argv[3]
            if labels_cmd == 'list':
                cmd_gmail_labels_list()
            elif labels_cmd == 'add':
                if len(sys.argv) < 6:
                    print("Usage: gmail labels add <message_id> '[\"Label1\", \"Label2\"]'")
                    sys.exit(1)
                cmd_gmail_labels_add(sys.argv[4], sys.argv[5])
            elif labels_cmd == 'remove':
                if len(sys.argv) < 6:
                    print("Usage: gmail labels remove <message_id> '[\"Label1\", \"Label2\"]'")
                    sys.exit(1)
                cmd_gmail_labels_remove(sys.argv[4], sys.argv[5])
            else:
                print(f"Unknown labels command: {labels_cmd}")
                sys.exit(1)
        else:
            print(f"Unknown gmail command: {subcmd}")
            sys.exit(1)

    elif cmd == 'sheets':
        if len(sys.argv) < 3:
            print("Usage: sheets get | update | create")
            sys.exit(1)
        subcmd = sys.argv[2]
        if subcmd == 'get':
            if len(sys.argv) < 5:
                print("Usage: sheets get <spreadsheet_id> <range>")
                sys.exit(1)
            cmd_sheets_get(sys.argv[3], sys.argv[4])
        elif subcmd == 'update':
            if len(sys.argv) < 6:
                print("Usage: sheets update <spreadsheet_id> <range> '[[\"val1\", \"val2\"], [\"val3\", \"val4\"]]'")
                sys.exit(1)
            cmd_sheets_update(sys.argv[3], sys.argv[4], sys.argv[5])
        elif subcmd == 'create':
            if len(sys.argv) < 4:
                print("Usage: sheets create <title>")
                sys.exit(1)
            cmd_sheets_create(sys.argv[3])
        else:
            print(f"Unknown sheets command: {subcmd}")
            sys.exit(1)

    elif cmd == 'docs':
        if len(sys.argv) < 3:
            print("Usage: docs get | create | append")
            sys.exit(1)
        subcmd = sys.argv[2]
        if subcmd == 'get':
            if len(sys.argv) < 4:
                print("Usage: docs get <document_id>")
                sys.exit(1)
            cmd_docs_get(sys.argv[3])
        elif subcmd == 'create':
            if len(sys.argv) < 4:
                print("Usage: docs create <title> [content]")
                sys.exit(1)
            content = sys.argv[4] if len(sys.argv) > 4 else ''
            cmd_docs_create(sys.argv[3], content)
        elif subcmd == 'append':
            if len(sys.argv) < 5:
                print("Usage: docs append <document_id> <content>")
                sys.exit(1)
            cmd_docs_append(sys.argv[3], sys.argv[4])
        else:
            print(f"Unknown docs command: {subcmd}")
            sys.exit(1)

    elif cmd == 'drive':
        if len(sys.argv) < 3:
            print("Usage: drive copy <file_id> <new_name>")
            sys.exit(1)
        subcmd = sys.argv[2]
        if subcmd == 'copy':
            if len(sys.argv) < 5:
                print("Usage: drive copy <file_id> <new_name>")
                sys.exit(1)
            cmd_drive_copy(sys.argv[3], sys.argv[4])
        else:
            print(f"Unknown drive command: {subcmd}")
            sys.exit(1)

    else:
        print(f"Unknown command: {cmd}")
        print(__doc__)
        sys.exit(1)


if __name__ == '__main__':
    main()
