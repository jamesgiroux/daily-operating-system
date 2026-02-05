"""
Step 5: Google API Setup.

Handles Google API OAuth configuration for Calendar, Gmail, Sheets, and Docs.

Credentials are stored securely at ~/.dailyos/google/ with restricted permissions.
"""

import os
import json
from pathlib import Path
from typing import Tuple, Optional, List, Dict


# Secure credential storage location
GOOGLE_CREDENTIALS_DIR = Path.home() / '.dailyos' / 'google'
CREDENTIALS_FILE = GOOGLE_CREDENTIALS_DIR / 'credentials.json'
TOKEN_FILE = GOOGLE_CREDENTIALS_DIR / 'token.json'


# Google API scopes by access level
SCOPES_FULL = [
    'https://www.googleapis.com/auth/calendar',
    'https://www.googleapis.com/auth/gmail.readonly',
    'https://www.googleapis.com/auth/gmail.compose',
    'https://www.googleapis.com/auth/gmail.modify',
    'https://www.googleapis.com/auth/spreadsheets',
    'https://www.googleapis.com/auth/documents',
    'https://www.googleapis.com/auth/drive.file',
]

SCOPES_READONLY = [
    'https://www.googleapis.com/auth/calendar.readonly',
    'https://www.googleapis.com/auth/gmail.readonly',
    'https://www.googleapis.com/auth/spreadsheets.readonly',
    'https://www.googleapis.com/auth/documents.readonly',
]


def get_google_setup_instructions() -> str:
    """
    Get step-by-step instructions for Google API setup.

    Returns:
        Formatted instruction string
    """
    return f"""
Google API Setup Instructions
=============================

Quick setup: Run 'dailyos google-setup' for guided setup.

Manual setup:

1. Go to Google Cloud Console:
   https://console.cloud.google.com/

2. Create a new project (or select existing):
   - Click the project dropdown at the top
   - Click "New Project"
   - Name it (e.g., "Productivity System")
   - Click "Create"

3. Enable required APIs:
   Go to "APIs & Services" > "Library" and enable:
   - Google Calendar API
   - Gmail API
   - Google Sheets API
   - Google Docs API
   - Google Drive API

4. Configure OAuth consent screen:
   Go to "APIs & Services" > "OAuth consent screen"
   - Choose "External" user type
   - Fill in app name (e.g., "Productivity System")
   - Add your email as test user
   - Save

5. Create OAuth credentials:
   Go to "APIs & Services" > "Credentials"
   - Click "Create Credentials" > "OAuth client ID"
   - Choose "Desktop app" as application type
   - Name it (e.g., "Desktop Client")
   - Click "Create"
   - Download the JSON file

6. Save the credentials:
   Save the downloaded file as:
   {CREDENTIALS_FILE}

   (This secure location keeps credentials out of your workspace)

7. Authorize the application:
   Run 'dailyos google-setup --verify' or use the Google API
   script - it will open a browser for authorization.
"""


def get_interactive_setup_steps() -> List[dict]:
    """
    Get the interactive setup steps for beginner-friendly Google API setup.

    Each step includes:
    - title: Short title for the step
    - instruction: Detailed instructions
    - action_url: URL to open (if applicable)
    - check_message: Message to confirm completion
    - tips: List of helpful tips

    Returns:
        List of step dictionaries
    """
    return [
        {
            'number': 1,
            'title': 'Open Google Cloud Console',
            'instruction': '''First, we need to open Google Cloud Console in your browser.

This is Google's free developer platform. You'll create a "project"
that lets Claude read your calendar and email.''',
            'action_url': 'https://console.cloud.google.com/',
            'action_text': 'Open Google Cloud Console',
            'check_message': 'Did the page load and are you signed into your Google account?',
            'tips': [
                'Sign in with the Google account you use for calendar/email',
                "It's free - Google doesn't charge for personal API use",
                'If you see "Activate" prompt for free trial, you can skip it',
            ]
        },
        {
            'number': 2,
            'title': 'Create a New Project',
            'instruction': '''Now create a project to contain your API access:

1. Look at the top of the page for a dropdown (might say "Select a project")
2. Click it, then click "New Project"
3. Name it "Productivity System" (or any name you like)
4. Click "Create"

Wait a few seconds for it to create...''',
            'action_url': None,
            'action_text': None,
            'check_message': 'Did you create the project and see it selected at the top?',
            'tips': [
                'The project name is just for you - pick anything memorable',
                'Make sure it\'s selected in the dropdown after creation',
            ]
        },
        {
            'number': 3,
            'title': 'Enable the APIs',
            'instruction': '''Now we need to "turn on" each Google service:

1. In the left menu, click "APIs & Services"
2. Click "Library"
3. Search for and enable EACH of these (click the API, then click "Enable"):

   • Google Calendar API
   • Gmail API
   • Google Sheets API
   • Google Docs API
   • Google Drive API

Do this for all 5 APIs before continuing.''',
            'action_url': 'https://console.cloud.google.com/apis/library',
            'action_text': 'Open API Library',
            'check_message': 'Did you enable all 5 APIs?',
            'tips': [
                'You can search for each one by name',
                'After clicking Enable, use the back button to return to Library',
                'If it says "Manage" instead of "Enable", it\'s already enabled',
            ]
        },
        {
            'number': 4,
            'title': 'Configure OAuth Consent Screen',
            'instruction': '''Google needs to know what your "app" is called:

1. In the left menu, go to "APIs & Services" → "OAuth consent screen"
2. Select "External" as the user type, click "Create"
3. Fill in:
   • App name: "Productivity System"
   • User support email: Your email
   • Developer contact: Your email
4. Click "Save and Continue"
5. On Scopes page, just click "Save and Continue"
6. On Test Users, click "Add Users", add YOUR email
7. Click "Save and Continue", then "Back to Dashboard"''',
            'action_url': 'https://console.cloud.google.com/apis/credentials/consent',
            'action_text': 'Open OAuth Consent',
            'check_message': 'Did you complete the consent screen setup?',
            'tips': [
                'The "External" type is fine - only you will use it',
                'Adding yourself as a test user is required',
                'You can skip the optional fields',
            ]
        },
        {
            'number': 5,
            'title': 'Create OAuth Credentials',
            'instruction': '''Now we create the actual credentials file:

1. Go to "APIs & Services" → "Credentials"
2. Click "Create Credentials" at the top
3. Choose "OAuth client ID"
4. For "Application type", select "Desktop app"
5. Name it "Desktop Client" (or any name)
6. Click "Create"

A popup will appear with your credentials!''',
            'action_url': 'https://console.cloud.google.com/apis/credentials',
            'action_text': 'Open Credentials',
            'check_message': 'Did you see the popup with "Your Client ID" and "Your Client Secret"?',
            'tips': [
                'Choose "Desktop app" not "Web application"',
                'The popup has a "Download JSON" button - use it next',
            ]
        },
        {
            'number': 6,
            'title': 'Download the Credentials File',
            'instruction': f'''Download and save the credentials:

1. In the popup (or click the download icon in the credentials list)
2. Click "Download JSON"
3. Save the file as exactly: credentials.json
4. Move it to: {CREDENTIALS_FILE}

This secure location keeps credentials out of your workspace.''',
            'action_url': None,
            'action_text': None,
            'check_message': 'Did you download the credentials.json file?',
            'tips': [
                'The file will have a long name - rename it to "credentials.json"',
                'Keep this file private - it gives access to your Google data',
                f'Store at {GOOGLE_CREDENTIALS_DIR} (secure, not in workspace)',
            ]
        },
    ]


def print_step_interactive(step: dict, workspace: Path = None) -> bool:
    """
    Print a single setup step and wait for user confirmation.

    Args:
        step: Step dictionary from get_interactive_setup_steps()
        workspace: Optional workspace path for file operations

    Returns:
        True if user confirmed, False if they want to skip
    """
    print(f"\n{'='*60}")
    print(f"Step {step['number']}: {step['title']}")
    print('='*60)
    print()
    print(step['instruction'])

    if step.get('tips'):
        print()
        print("💡 Tips:")
        for tip in step['tips']:
            print(f"   • {tip}")

    if step.get('action_url'):
        print()
        print(f"🔗 {step['action_text']}: {step['action_url']}")

    print()
    print("-"*40)
    response = input(f"{step['check_message']} [Y/n/skip]: ").strip().lower()

    if response == 'skip':
        return False
    elif response == 'n' or response == 'no':
        print("No problem! Re-read the instructions above and try again.")
        print("Press Enter when ready to continue...")
        input()
        return True
    else:
        return True


def run_interactive_google_setup(workspace: Path) -> dict:
    """
    Run the full interactive Google setup flow for beginners.

    This walks users through each step of Google Cloud setup with
    clear instructions and confirmation prompts.

    Args:
        workspace: Root workspace path

    Returns:
        Dictionary with setup status
    """
    print()
    print("╔" + "="*58 + "╗")
    print("║" + " Google API Setup Wizard ".center(58) + "║")
    print("╚" + "="*58 + "╝")
    print()
    print("This wizard will guide you through connecting Google services.")
    print("It takes about 10 minutes. You can skip any step if needed.")
    print()
    print("Press Enter to begin...")
    input()

    steps = get_interactive_setup_steps()
    completed_steps = []

    for step in steps:
        success = print_step_interactive(step, workspace)
        if success:
            completed_steps.append(step['number'])

    # After all steps, check for credentials file
    print()
    print("="*60)
    print("Final Step: Place your credentials file")
    print("="*60)
    print()

    print(f"Copy your downloaded credentials.json to:")
    print(f"  {CREDENTIALS_FILE}")
    print()
    print("(This secure location keeps credentials out of your workspace)")
    print()

    # Ensure directory exists
    GOOGLE_CREDENTIALS_DIR.mkdir(parents=True, exist_ok=True)
    os.chmod(GOOGLE_CREDENTIALS_DIR, 0o700)

    # Wait for file or skip
    while True:
        if CREDENTIALS_FILE.exists():
            print("✅ Found credentials.json!")
            # Ensure secure permissions
            os.chmod(CREDENTIALS_FILE, 0o600)
            break

        response = input("Press Enter after copying the file (or 'skip' to continue without Google): ").strip().lower()
        if response == 'skip':
            print("Skipping Google setup. You can set this up later by running:")
            print("  dailyos google-setup")
            return {
                'completed': False,
                'steps_done': completed_steps,
                'credentials_found': False,
            }

    # Try to run auth
    print()
    print("Now let's authorize the application...")
    print("A browser window will open for you to sign in.")
    print()
    print("Press Enter to open the authorization page...")
    input()

    return {
        'completed': True,
        'steps_done': completed_steps,
        'credentials_found': True,
        'credentials_path': str(CREDENTIALS_FILE),
    }


def check_credentials_exist(workspace: Path = None) -> Tuple[bool, Optional[Path]]:
    """
    Check if Google credentials file exists in the secure location.

    Args:
        workspace: Deprecated - kept for API compatibility, not used

    Returns:
        Tuple of (exists, path)
    """
    return CREDENTIALS_FILE.exists(), CREDENTIALS_FILE


def check_token_exists(workspace: Path = None) -> Tuple[bool, Optional[Path]]:
    """
    Check if Google token file exists (indicates successful auth).

    Args:
        workspace: Deprecated - kept for API compatibility, not used

    Returns:
        Tuple of (exists, path)
    """
    return TOKEN_FILE.exists(), TOKEN_FILE


def check_legacy_credentials(workspace: Path) -> Tuple[bool, Optional[Path]]:
    """
    Check if Google credentials exist in the legacy workspace location.

    Args:
        workspace: Root workspace path

    Returns:
        Tuple of (exists, path)
    """
    legacy_path = workspace / '.config' / 'google' / 'credentials.json'
    return legacy_path.exists(), legacy_path


def validate_credentials_json(content: str) -> Tuple[bool, Optional[str]]:
    """
    Validate that content is a valid Google OAuth credentials file.

    Args:
        content: JSON string content

    Returns:
        Tuple of (is_valid, error_message)
    """
    try:
        data = json.loads(content)

        # Handle null/None JSON values
        if data is None:
            return False, "Invalid credentials format: null value"

        # Handle non-dict types (arrays, strings, numbers)
        if not isinstance(data, dict):
            return False, "Invalid credentials format: expected JSON object"

        # Check for required fields (Desktop app credentials)
        if 'installed' in data:
            installed = data['installed']
            required = ['client_id', 'client_secret', 'auth_uri', 'token_uri']
            missing = [f for f in required if f not in installed]
            if missing:
                return False, f"Missing required fields: {', '.join(missing)}"
            return True, None

        # Check for web app credentials format
        if 'web' in data:
            return False, "This appears to be a Web Application credential. Please use Desktop App type instead."

        return False, "Invalid credentials format. Expected 'installed' key for Desktop App credentials."

    except json.JSONDecodeError as e:
        return False, f"Invalid JSON: {e}"


def save_credentials_secure(content: str) -> Tuple[bool, Optional[str]]:
    """
    Save credentials to the secure location with proper permissions.

    Args:
        content: JSON string content of credentials

    Returns:
        Tuple of (success, error_message)
    """
    # Validate first
    is_valid, error = validate_credentials_json(content)
    if not is_valid:
        return False, error

    try:
        # Ensure directory exists with secure permissions
        GOOGLE_CREDENTIALS_DIR.mkdir(parents=True, exist_ok=True)
        os.chmod(GOOGLE_CREDENTIALS_DIR, 0o700)

        # Write credentials with secure permissions
        with open(CREDENTIALS_FILE, 'w') as f:
            f.write(content)
        os.chmod(CREDENTIALS_FILE, 0o600)

        return True, None
    except Exception as e:
        return False, str(e)


def get_api_features() -> List[dict]:
    """
    Get list of API features and their descriptions.

    Returns:
        List of feature dictionaries
    """
    return [
        {
            'name': 'Calendar',
            'full': 'View, create, update, and delete events',
            'readonly': 'View events only',
            'use_cases': [
                'Daily meeting prep',
                'Schedule management',
                'Time blocking',
            ]
        },
        {
            'name': 'Gmail',
            'full': 'Read emails, create drafts, manage labels',
            'readonly': 'Read emails only',
            'use_cases': [
                'Email triage',
                'Draft follow-ups',
                'Customer communication tracking',
            ]
        },
        {
            'name': 'Sheets',
            'full': 'Read and update spreadsheets',
            'readonly': 'Read spreadsheets only',
            'use_cases': [
                'Account tracking',
                'Data dashboards',
                'Reporting',
            ]
        },
        {
            'name': 'Docs',
            'full': 'Read, create, and edit documents',
            'readonly': 'Read documents only',
            'use_cases': [
                'Meeting agendas',
                'Shared documents',
                'Templates',
            ]
        },
    ]


def get_google_api_script_content() -> str:
    """
    Get the content for the google_api.py helper script.

    Returns:
        Python script content as string
    """
    # This will be loaded from templates/scripts/google_api.py
    # For now, return a placeholder that will be replaced
    return '''#!/usr/bin/env python3
"""
Google API Helper Script.

Usage:
    python3 google_api.py calendar list [days]
    python3 google_api.py calendar get <event_id>
    python3 google_api.py calendar create <title> <start> <end> [desc]
    python3 google_api.py gmail list [max]
    python3 google_api.py gmail get <message_id>
    python3 google_api.py gmail search <query> [max]
    python3 google_api.py sheets get <spreadsheet_id> <range>
    python3 google_api.py docs get <doc_id>

First run will open browser for OAuth authorization.
"""

import sys

def main():
    print("Google API helper - install full version with setup wizard")
    print("Run: python3 advanced-start.py")
    sys.exit(1)

if __name__ == "__main__":
    main()
'''


def install_google_api_script(workspace: Path, file_ops) -> bool:
    """
    Install the Google API helper script.

    Args:
        workspace: Root workspace path
        file_ops: FileOperations instance

    Returns:
        True if installed successfully
    """
    script_path = workspace / '.config' / 'google' / 'google_api.py'
    content = get_google_api_script_content()
    file_ops.write_file(script_path, content)
    return True


def verify_google_setup(workspace: Path = None) -> dict:
    """
    Verify Google API setup status.

    Args:
        workspace: Optional workspace path (for legacy script check)

    Returns:
        Dictionary with verification results
    """
    creds_exist, creds_path = check_credentials_exist()
    token_exist, token_path = check_token_exists()

    # Check for legacy credentials that could be migrated
    legacy_creds_exist = False
    if workspace:
        legacy_creds_exist, _ = check_legacy_credentials(workspace)

    result = {
        'credentials_exist': creds_exist,
        'credentials_path': str(creds_path) if creds_path else None,
        'token_exist': token_exist,
        'token_path': str(token_path) if token_path else None,
        'authorized': token_exist,  # Token exists means auth was successful
        'legacy_credentials_exist': legacy_creds_exist,
        'secure_location': str(GOOGLE_CREDENTIALS_DIR),
    }

    # Check script existence if workspace provided
    if workspace:
        result['script_exist'] = (workspace / '.config' / 'google' / 'google_api.py').exists()

    return result
