#!/usr/bin/env python3
"""
Phase 1: Inbox Preparation Script
Analyzes files in _inbox/ and generates directives for Claude to process.

This script:
1. Scans _inbox/ for processable files
2. Analyzes each file for type, context, and routing hints
3. Generates .processing-state.json with agent directives
4. Creates backup of original files

Usage:
    python3 prepare_inbox.py [inbox_path]

Exit codes: 0 = success, 1 = any step failed
"""

import sys
import re
import json
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Any, Optional
import shutil

# Document type patterns
DOC_TYPE_PATTERNS = {
    'transcript': ['transcript', 'call', 'recording', 'meeting-notes'],
    'summary': ['summary', 'notes', 'recap'],
    'actions': ['action', 'actions', 'todo', 'tasks'],
    'strategy': ['strategy', 'analysis', 'proposal', 'assessment'],
    'report': ['report', 'update', 'review'],
}

# Default agent assignments by document type
AGENTS_BY_TYPE = {
    'transcript': [
        {'name': 'summary-generator', 'creates_file': True, 'suffix': '-summary.md'},
        {'name': 'action-extractor', 'creates_file': True, 'suffix': '-actions.md'},
        {'name': 'tagger-processor', 'creates_file': False},
    ],
    'summary': [
        {'name': 'tagger-processor', 'creates_file': False},
        {'name': 'document-linker', 'creates_file': False},
    ],
    'strategy': [
        {'name': 'tagger-processor', 'creates_file': False},
        {'name': 'document-linker', 'creates_file': False},
    ],
    'report': [
        {'name': 'tagger-processor', 'creates_file': False},
    ],
    'default': [
        {'name': 'tagger-processor', 'creates_file': False},
    ],
}


class InboxPreparer:
    """Prepare inbox files for Claude processing."""

    def __init__(self, inbox_path: str = "_inbox"):
        self.inbox_path = Path(inbox_path)
        self.files_analyzed = []
        self.directives = {}

        # Ensure inbox exists
        if not self.inbox_path.exists():
            print(f"Creating inbox directory: {self.inbox_path}")
            self.inbox_path.mkdir(parents=True, exist_ok=True)

    def detect_doc_type(self, filepath: Path) -> str:
        """Detect document type from filename and content."""
        filename = filepath.name.lower()

        for doc_type, patterns in DOC_TYPE_PATTERNS.items():
            for pattern in patterns:
                if pattern in filename:
                    return doc_type

        # Try to detect from content
        try:
            with open(filepath, 'r') as f:
                content = f.read(2000)  # Read first 2KB

            content_lower = content.lower()

            # Check for transcript indicators
            if any(ind in content_lower for ind in ['speaker:', '[00:', 'transcript', 'recording']):
                return 'transcript'

            # Check for meeting summary indicators
            if any(ind in content_lower for ind in ['attendees:', 'key points:', 'discussion points:']):
                return 'summary'

            # Check for action items
            if any(ind in content_lower for ind in ['- [ ]', 'action items:', 'next steps:', 'todo:']):
                return 'actions'

        except:
            pass

        return 'general'

    def extract_date(self, filepath: Path) -> Optional[str]:
        """Extract date from filename (expects YYYY-MM-DD prefix)."""
        filename = filepath.name

        # Try YYYY-MM-DD format
        match = re.match(r'^(\d{4}-\d{2}-\d{2})', filename)
        if match:
            return match.group(1)

        # Try to extract from content
        try:
            with open(filepath, 'r') as f:
                content = f.read(1000)

            # Look for date in frontmatter or content
            date_match = re.search(r'date:\s*(\d{4}-\d{2}-\d{2})', content)
            if date_match:
                return date_match.group(1)
        except:
            pass

        return None

    def detect_context(self, filepath: Path) -> Dict[str, Any]:
        """Detect account/project context from file content."""
        context = {
            'account': None,
            'project': None,
            'area': None,
        }

        try:
            with open(filepath, 'r') as f:
                content = f.read()

            # Check frontmatter for explicit context
            if content.startswith('---'):
                end_match = re.search(r'\n---\n', content)
                if end_match:
                    yaml_content = content[4:end_match.start()]

                    # Extract account
                    account_match = re.search(r'account:\s*["\']?([^"\'\n]+)', yaml_content)
                    if account_match:
                        context['account'] = account_match.group(1).strip()

                    # Extract project
                    project_match = re.search(r'project:\s*["\']?([^"\'\n]+)', yaml_content)
                    if project_match:
                        context['project'] = project_match.group(1).strip()

                    # Extract area
                    area_match = re.search(r'area:\s*["\']?([^"\'\n]+)', yaml_content)
                    if area_match:
                        context['area'] = area_match.group(1).strip()

            # Try to detect account from filename
            if not context['account']:
                filename = filepath.name.lower()
                # Look for common patterns like "2026-01-15-acme-call.md"
                parts = filename.replace('-', ' ').replace('_', ' ').split()
                # Skip date parts and type markers
                skip_words = ['transcript', 'summary', 'call', 'meeting', 'notes', 'sync', 'review']
                potential_account = None
                for i, part in enumerate(parts[3:], start=3):  # Skip date parts (YYYY MM DD)
                    if part not in skip_words and len(part) > 2 and not part.isdigit():
                        potential_account = part.title()
                        break
                if potential_account:
                    context['account'] = potential_account

            # Infer area from account/project
            if context['account']:
                context['area'] = 'Accounts'
            elif context['project']:
                context['area'] = 'Projects'

        except Exception as e:
            print(f"  Warning: Could not detect context: {e}")

        return context

    def determine_destination(self, filepath: Path, doc_type: str, context: Dict[str, Any]) -> Optional[str]:
        """Determine the canonical destination for a file."""
        account = context.get('account')
        project = context.get('project')
        area = context.get('area')

        if area == 'Accounts' and account:
            # Map doc_type to account subdirectory
            type_to_dir = {
                'transcript': '03-Call-Transcripts',
                'summary': '02-Meetings',
                'actions': '04-Action-Items',
                'strategy': '05-Projects',
                'report': '07-Reporting',
            }
            subdir = type_to_dir.get(doc_type, '02-Meetings')
            return f"Accounts/{account}/{subdir}"

        elif area == 'Projects' and project:
            return f"Projects/{project}"

        return None

    def analyze_file(self, filepath: Path) -> Dict[str, Any]:
        """Analyze a single file and generate processing directives."""
        print(f"\n  Analyzing: {filepath.name}")

        doc_type = self.detect_doc_type(filepath)
        date = self.extract_date(filepath)
        context = self.detect_context(filepath)
        destination = self.determine_destination(filepath, doc_type, context)

        # Get agents for this document type
        agents_config = AGENTS_BY_TYPE.get(doc_type, AGENTS_BY_TYPE['default'])
        agents = []

        for agent_cfg in agents_config:
            agent = {
                'name': agent_cfg['name'],
                'creates_file': agent_cfg.get('creates_file', False),
            }
            if agent_cfg.get('creates_file') and agent_cfg.get('suffix'):
                # Generate output filename
                base_name = filepath.stem
                if date and base_name.startswith(date):
                    # Keep date prefix
                    output_name = f"{date}{agent_cfg['suffix']}"
                else:
                    output_name = f"{base_name}{agent_cfg['suffix']}"
                agent['creates_file'] = str(self.inbox_path / output_name)
            agents.append(agent)

        analysis = {
            'path': str(filepath),
            'filename': filepath.name,
            'type': doc_type,
            'date': date,
            'context': context,
            'destination': destination,
            'agents': agents,
            'status': 'ready_for_processing',
        }

        print(f"    Type: {doc_type}")
        print(f"    Date: {date or 'unknown'}")
        print(f"    Account: {context.get('account', 'none detected')}")
        print(f"    Destination: {destination or 'to be determined'}")
        print(f"    Agents: {[a['name'] for a in agents]}")

        return analysis

    def create_backup(self, filepath: Path):
        """Create a backup of the original file."""
        backup_dir = self.inbox_path / '.backups' / datetime.now().strftime('%Y-%m-%d')
        backup_dir.mkdir(parents=True, exist_ok=True)

        backup_path = backup_dir / filepath.name
        shutil.copy2(filepath, backup_path)

    def prepare_all(self) -> bool:
        """Prepare all files in the inbox for processing."""
        print("=" * 60)
        print("PHASE 1: INBOX PREPARATION AND ANALYSIS")
        print("=" * 60)
        print(f"Scanning: {self.inbox_path}")

        # Find all markdown files (excluding system files)
        md_files = [f for f in self.inbox_path.glob("*.md")
                   if not f.name.startswith('.')]

        if not md_files:
            print("\n✅ Inbox is empty - no files to process")
            return True

        print(f"\nFound {len(md_files)} files to analyze")

        # Analyze each file
        for filepath in md_files:
            try:
                analysis = self.analyze_file(filepath)
                self.files_analyzed.append(analysis)
                self.create_backup(filepath)
            except Exception as e:
                print(f"    ERROR: {e}")
                continue

        # Generate directives file
        self.directives = {
            'prepared_at': datetime.now().isoformat(),
            'total_files': len(self.files_analyzed),
            'total_agents': sum(len(f['agents']) for f in self.files_analyzed),
            'files': {f['filename']: f for f in self.files_analyzed},
        }

        directives_file = self.inbox_path / '.phase1-directives.json'
        with open(directives_file, 'w') as f:
            json.dump(self.directives, f, indent=2)

        # Update processing state
        state_file = self.inbox_path / '.processing-state.json'
        state = {
            'created': datetime.now().isoformat(),
            'phases': {
                'phase1': {
                    'status': 'completed',
                    'timestamp': datetime.now().isoformat(),
                    'files_analyzed': len(self.files_analyzed),
                }
            }
        }
        with open(state_file, 'w') as f:
            json.dump(state, f, indent=2)

        # Output summary
        print("\n" + "=" * 60)
        print("PREPARATION COMPLETE")
        print("=" * 60)
        print(f"\nFiles analyzed: {len(self.files_analyzed)}")
        print(f"Agent tasks: {self.directives['total_agents']}")
        print(f"\nDirectives saved to: {directives_file}")

        # Print agent directives for Claude
        print("\n" + "=" * 60)
        print("AGENT DIRECTIVES FOR CLAUDE")
        print("=" * 60)

        for filename, info in self.directives['files'].items():
            print(f"\n### {filename}")
            print(f"Type: {info['type']}")
            print(f"Account: {info['context'].get('account', 'none')}")

            for agent in info['agents']:
                print(f"\n→ Agent: {agent['name']}")
                if agent.get('creates_file'):
                    print(f"  Creates: {agent['creates_file']}")
                else:
                    print(f"  Updates: frontmatter")

        print("\n\nAfter Claude completes enrichment, run: python3 deliver_inbox.py")

        return True


def main():
    """Main entry point."""
    inbox_path = sys.argv[1] if len(sys.argv) > 1 else "_inbox"

    preparer = InboxPreparer(inbox_path)
    success = preparer.prepare_all()

    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
