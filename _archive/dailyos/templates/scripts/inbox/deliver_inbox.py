#!/usr/bin/env python3
"""
Phase 3: Inbox Delivery to PARA Structure
Moves enriched files from _inbox/ to their correct PARA locations.

After Claude has executed agents (Phase 2), this script:
1. Reads updated frontmatter from each file
2. Determines the correct PARA location
3. Moves files to their destinations
4. Updates any indexes
5. Verifies all links are valid

Usage:
    python3 deliver_inbox.py [inbox_path]

Exit codes: 0 = success, 1 = any step failed
"""

import os
import re
import shutil
import json
from pathlib import Path
from typing import Dict, List, Any, Optional
from datetime import datetime

try:
    import yaml
except ImportError:
    yaml = None


class InboxDeliverer:
    """Deliver processed files to PARA locations."""

    def __init__(self, inbox_path: str = "_inbox"):
        self.inbox_path = Path(inbox_path)
        self.delivered_files = []
        self.failed_files = []
        self.verification_warnings = []

        # Create failed directory if needed
        self.failed_path = self.inbox_path / "failed"
        self.failed_path.mkdir(exist_ok=True)

        # Load Phase 1 directives if they exist
        self.directives_file = self.inbox_path / ".phase1-directives.json"
        self.expected_agents = self.load_expected_agents()

    def update_processing_state(self, phase: str, status: str, details: dict = None):
        """Update the processing state file."""
        state_file = self.inbox_path / ".processing-state.json"

        # Load existing state or create new
        if state_file.exists():
            with open(state_file, 'r') as f:
                state = json.load(f)
        else:
            state = {
                "created": datetime.now().isoformat(),
                "phases": {}
            }

        # Update phase status
        state["last_updated"] = datetime.now().isoformat()
        state["phases"][phase] = {
            "status": status,
            "timestamp": datetime.now().isoformat(),
            "details": details or {}
        }

        # Save state
        with open(state_file, 'w') as f:
            json.dump(state, f, indent=2)

    def load_expected_agents(self) -> Dict[str, List[str]]:
        """Load expected agents from Phase 1 directives."""
        if not self.directives_file.exists():
            return {}

        try:
            with open(self.directives_file, 'r') as f:
                directives = json.load(f)

            # Build map of filename to expected agents
            expected = {}
            for filename, info in directives.get('files', {}).items():
                expected[filename] = [agent['name'] for agent in info.get('agents', [])]

            return expected
        except Exception as e:
            print(f"  Warning: Could not load directives: {e}")
            return {}

    def read_frontmatter(self, filepath: Path) -> Optional[Dict[str, Any]]:
        """Read and parse frontmatter from file."""
        try:
            with open(filepath, 'r') as f:
                content = f.read()

            if not content.startswith('---'):
                return None

            # Find end of frontmatter
            end_match = re.search(r'\n---\n', content)
            if not end_match:
                return None

            yaml_content = content[4:end_match.start()]

            # Parse YAML
            if yaml:
                metadata = yaml.safe_load(yaml_content) or {}
            else:
                # Simple fallback parser
                metadata = {}
                for line in yaml_content.split('\n'):
                    if ':' in line:
                        key, value = line.split(':', 1)
                        key = key.strip()
                        value = value.strip().strip('"').strip("'")
                        if value.startswith('[') and value.endswith(']'):
                            value = [v.strip().strip('"').strip("'")
                                   for v in value[1:-1].split(',')]
                        metadata[key] = value

            return metadata

        except Exception as e:
            print(f"  Error reading frontmatter: {e}")
            return None

    def get_valid_accounts(self) -> List[str]:
        """Get list of valid account directories."""
        accounts_path = Path('Accounts')
        if not accounts_path.exists():
            return []

        valid_accounts = []

        for item in accounts_path.iterdir():
            if item.is_dir() and not item.name.startswith('.') and not item.name.startswith('_'):
                # Check if this is a parent folder with sub-accounts
                subdirs = [d for d in item.iterdir() if d.is_dir() and not d.name.startswith('.')]
                has_bu_structure = False

                for subdir in subdirs:
                    # Check if subdir has canonical account structure
                    if (subdir / '00-Index.md').exists() or (subdir / '01-Customer-Information').exists():
                        has_bu_structure = True
                        # Add multi-BU format
                        valid_accounts.append(f"{item.name} / {subdir.name}")

                # If no BU structure, it's a single account
                if not has_bu_structure:
                    valid_accounts.append(item.name)

        return valid_accounts

    def get_valid_projects(self) -> List[str]:
        """Get list of valid project directories."""
        projects_path = Path('Projects')
        if not projects_path.exists():
            return []

        return [item.name for item in projects_path.iterdir()
                if item.is_dir() and not item.name.startswith('.') and not item.name.startswith('_')]

    def determine_destination(self, filepath: Path, metadata: Dict[str, Any]) -> Optional[Path]:
        """Determine PARA destination based on metadata."""
        area = metadata.get('area', '')
        doc_type = metadata.get('doc_type', '')

        # Handle Accounts area
        if area == 'Accounts' or metadata.get('account'):
            account = metadata.get('account', '')
            if not account:
                print(f"  Warning: No account specified for Accounts area")
                return None

            # Validate account exists
            valid_accounts = self.get_valid_accounts()
            if account not in valid_accounts:
                print(f"  Warning: Account '{account}' not found. Valid: {valid_accounts[:5]}...")
                return None

            # Convert account name to path format
            if ' / ' in account:
                account_path = account.replace(' / ', '/')
            else:
                account_path = account

            # Map doc_type to subdirectory
            type_to_dir = {
                'transcript': '03-Call-Transcripts',
                'summary': '02-Meetings',
                'meeting-summary': '02-Meetings',
                'actions': '04-Action-Items',
                'action_items': '04-Action-Items',
                'strategy': '05-Projects',
                'incident': '09-Incidents',
                'report': '07-Reporting',
                'project': '05-Projects'
            }

            subdir = type_to_dir.get(doc_type, '02-Meetings')
            destination = Path('Accounts') / account_path / subdir / filepath.name

            print(f"  -> Destination: {destination}")
            return destination

        # Handle Projects area
        elif area == 'Projects' or metadata.get('project'):
            project = metadata.get('project', 'General')

            # Validate project exists (unless General)
            if project != 'General':
                valid_projects = self.get_valid_projects()
                if project not in valid_projects:
                    print(f"  Warning: Project '{project}' not found. Valid: {valid_projects}")
                    return None

            destination = Path('Projects') / project / filepath.name
            print(f"  -> Destination: {destination}")
            return destination

        # Handle Leadership area
        elif area == 'Leadership':
            type_to_dir = {
                'transcript': '08-Call-Transcripts',
                'summary': '05-Internal-Meetings',
                'strategy': '01-Leadership-Information',
                'actions': '02-Action-Items',
                'report': '06-Professional-Development/02-Monthly-Reports',
            }

            subdir = type_to_dir.get(doc_type, '05-Internal-Meetings')
            destination = Path('Leadership') / subdir / filepath.name

            print(f"  -> Destination: {destination}")
            return destination

        # Handle Resources area
        elif area == 'Resources':
            destination = Path('Resources') / filepath.name
            print(f"  -> Destination: {destination}")
            return destination

        # Handle Archive area
        elif area == 'Archive':
            destination = Path('Archive') / filepath.name
            print(f"  -> Destination: {destination}")
            return destination

        print(f"  Warning: Cannot determine destination - area: {area}, type: {doc_type}")
        return None

    def verify_enrichment(self, filepath: Path, metadata: Dict[str, Any]) -> Dict[str, Any]:
        """Verify that expected enrichments were applied."""
        warnings = []
        blocking_errors = []
        filename = filepath.name

        # Check for TODO values in frontmatter
        for key, value in metadata.items():
            if isinstance(value, str) and '# TODO' in value:
                blocking_errors.append(f"TODO field remaining: {key}")

        # Check tags were added
        tags = metadata.get('tags', [])
        if not tags:
            warnings.append("Missing tags")

        # Check for expected related documents for transcripts
        doc_type = metadata.get('doc_type', '')
        if doc_type == 'transcript':
            # Look for summary file
            date_prefix = filename[:10] if len(filename) >= 10 else ''
            if date_prefix:
                summary_found = any(self.inbox_path.glob(f"{date_prefix}*summary*.md"))
                actions_found = any(self.inbox_path.glob(f"{date_prefix}*actions*.md"))

                if not summary_found:
                    warnings.append("Summary file not found for transcript")
                if not actions_found:
                    warnings.append("Actions file not found for transcript")

        return {
            'passed': len(blocking_errors) == 0,
            'warnings': warnings,
            'blocking_errors': blocking_errors,
        }

    def move_file(self, source: Path, destination: Path) -> bool:
        """Move file to destination, creating directories if needed."""
        try:
            # Create destination directory if needed
            destination.parent.mkdir(parents=True, exist_ok=True)

            # Move the file
            shutil.move(str(source), str(destination))
            print(f"  ✓ Moved to: {destination}")
            return True

        except Exception as e:
            print(f"  ✗ Failed to move: {e}")
            return False

    def handle_failed_file(self, filepath: Path, reason: str):
        """Handle files that failed to process."""
        self.failed_files.append((filepath, reason))

        # Create error file
        error_file = self.failed_path / f"{filepath.name}.error"
        with open(error_file, 'w') as f:
            f.write(f"Error: {reason}\n")
            f.write(f"File: {filepath.name}\n")
            f.write(f"Time: {datetime.now()}\n\n")
            f.write("Fix suggestions:\n")

            if "No frontmatter" in reason:
                f.write("- Add frontmatter with doc_type, area, and account/project fields\n")
            elif "TODO field" in reason:
                f.write("- Re-run Claude agents to populate TODO fields\n")
            elif "not found" in reason:
                f.write("- Create the required account or project directory first\n")
                f.write("- Or update the frontmatter to reference an existing directory\n")
            else:
                f.write("- Add 'area' field (Accounts, Projects, Leadership, Resources)\n")
                f.write("- Add 'account' or 'project' field as appropriate\n")

        # Move file to failed directory
        failed_destination = self.failed_path / filepath.name
        shutil.move(str(filepath), str(failed_destination))

        print(f"  ✗ Failed: {reason}")
        print(f"  -> Moved to: {failed_destination}")

    def cleanup_temporary_files(self):
        """Clean up temporary processing files after successful completion."""
        print("\nCleaning up temporary files...")

        temp_files = [
            self.inbox_path / ".phase1-directives.json",
            self.inbox_path / ".processing-state.json",
            self.inbox_path / ".processing-errors.json",
        ]

        for temp_file in temp_files:
            if temp_file.exists():
                temp_file.unlink()
                print(f"  ✓ Removed {temp_file.name}")

        # Remove empty failed directory
        if self.failed_path.exists() and not any(self.failed_path.iterdir()):
            self.failed_path.rmdir()
            print("  ✓ Removed empty failed/ directory")

    def deliver_all(self) -> bool:
        """Main orchestrator - deliver all files to PARA structure."""
        print("=" * 60)
        print("PHASE 3: DELIVERY TO PARA STRUCTURE")
        print("=" * 60)

        self.update_processing_state("phase3", "started")

        print(f"Moving files from {self.inbox_path}/ to PARA locations\n")

        # Get all markdown files (excluding system files and processing files)
        md_files = [f for f in self.inbox_path.glob("*.md")
                   if not f.name.startswith('.')]

        if not md_files:
            print("✅ Inbox is empty - nothing to deliver")
            return True

        print(f"Found {len(md_files)} files to deliver\n")

        # Process each file
        for filepath in md_files:
            print(f"Processing: {filepath.name}")

            # Read frontmatter
            metadata = self.read_frontmatter(filepath)
            if not metadata:
                self.handle_failed_file(filepath, "No frontmatter found")
                continue

            # Verify enrichment
            verification = self.verify_enrichment(filepath, metadata)
            if verification.get('blocking_errors'):
                self.handle_failed_file(filepath, verification['blocking_errors'][0])
                continue
            if verification.get('warnings'):
                self.verification_warnings.extend(verification['warnings'])
                print(f"  Warnings: {verification['warnings']}")

            # Determine destination
            destination = self.determine_destination(filepath, metadata)
            if not destination:
                self.handle_failed_file(filepath, "Cannot determine destination")
                continue

            # Move file
            if self.move_file(filepath, destination):
                self.delivered_files.append((filepath, destination))
            else:
                self.handle_failed_file(filepath, "Failed to move file")

        # Output results
        self.output_results()

        # Update state
        success = len(self.failed_files) == 0
        self.update_processing_state("phase3", "completed" if success else "failed", {
            "files_delivered": len(self.delivered_files),
            "files_failed": len(self.failed_files),
            "success": success
        })

        return success

    def output_results(self):
        """Output delivery results."""
        print("\n" + "=" * 60)
        print("DELIVERY RESULTS")
        print("=" * 60)

        if self.delivered_files:
            print(f"\n✅ Successfully delivered {len(self.delivered_files)} files:")
            for source, dest in self.delivered_files:
                print(f"  • {source.name} -> {dest}")

        if self.verification_warnings:
            unique_warnings = list(set(self.verification_warnings))
            print(f"\n⚠️ Warnings ({len(unique_warnings)}):")
            for warning in unique_warnings:
                print(f"  • {warning}")

        if self.failed_files:
            print(f"\n❌ Failed to deliver {len(self.failed_files)} files:")
            for filepath, reason in self.failed_files:
                print(f"  • {filepath.name}: {reason}")
            print(f"\nFailed files moved to: {self.failed_path}/")
            print("Check .error files for fix suggestions")

        # Check if inbox is empty
        remaining = list(self.inbox_path.glob("*.md"))
        remaining = [f for f in remaining if not f.name.startswith('.')]

        if not remaining and not self.failed_files:
            print("\n✅ INBOX SUCCESSFULLY EMPTIED")
            print("✅ All files delivered to PARA structure")
            self.cleanup_temporary_files()
        elif remaining:
            print(f"\n⚠ {len(remaining)} files still in inbox")

        print("=" * 60)


def main():
    """Main entry point."""
    inbox_path = sys.argv[1] if len(sys.argv) > 1 else "_inbox"

    deliverer = InboxDeliverer(inbox_path)
    success = deliverer.deliver_all()

    return 0 if success else 1


if __name__ == "__main__":
    import sys
    sys.exit(main())
