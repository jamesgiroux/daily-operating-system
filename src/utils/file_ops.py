"""
Safe file operations with rollback support.
"""

import os
import shutil
from pathlib import Path
from typing import List, Optional
from datetime import datetime


class FileOperationError(Exception):
    """Custom exception for file operation failures."""
    pass


class FileOperations:
    """
    Safe file operations with tracking for rollback.

    Philosophy: Be conservative. Never overwrite without confirmation.
    Always maintain rollback capability.
    """

    def __init__(self):
        self.created_files: List[Path] = []
        self.created_dirs: List[Path] = []
        self.backed_up_files: List[tuple] = []  # (original, backup)

    def create_directory(self, path: Path, parents: bool = True) -> bool:
        """
        Create a directory if it doesn't exist.

        Args:
            path: Directory path to create
            parents: Create parent directories if needed

        Returns:
            True if created, False if already existed
        """
        if path.exists():
            return False

        try:
            path.mkdir(parents=parents, exist_ok=True)
            self.created_dirs.append(path)
            return True
        except Exception as e:
            raise FileOperationError(f"Failed to create directory {path}: {e}")

    def write_file(self, path: Path, content: str, backup: bool = True) -> bool:
        """
        Write content to a file.

        Args:
            path: File path
            content: Content to write
            backup: Create backup if file exists

        Returns:
            True if written successfully
        """
        # Ensure parent directory exists
        path.parent.mkdir(parents=True, exist_ok=True)

        # Backup if file exists
        if path.exists() and backup:
            backup_path = path.with_suffix(f"{path.suffix}.bak.{datetime.now().strftime('%Y%m%d%H%M%S')}")
            shutil.copy2(path, backup_path)
            self.backed_up_files.append((path, backup_path))
        elif not path.exists():
            self.created_files.append(path)

        try:
            with open(path, 'w') as f:
                f.write(content)
            return True
        except Exception as e:
            raise FileOperationError(f"Failed to write file {path}: {e}")

    def copy_file(self, source: Path, dest: Path, backup: bool = True) -> bool:
        """
        Copy a file.

        Args:
            source: Source file path
            dest: Destination file path
            backup: Create backup if dest exists

        Returns:
            True if copied successfully
        """
        if not source.exists():
            raise FileOperationError(f"Source file does not exist: {source}")

        # Ensure parent directory exists
        dest.parent.mkdir(parents=True, exist_ok=True)

        # Backup if dest exists
        if dest.exists() and backup:
            backup_path = dest.with_suffix(f"{dest.suffix}.bak.{datetime.now().strftime('%Y%m%d%H%M%S')}")
            shutil.copy2(dest, backup_path)
            self.backed_up_files.append((dest, backup_path))
        elif not dest.exists():
            self.created_files.append(dest)

        try:
            shutil.copy2(source, dest)
            return True
        except Exception as e:
            raise FileOperationError(f"Failed to copy {source} to {dest}: {e}")

    def copy_directory(self, source: Path, dest: Path) -> bool:
        """
        Copy a directory recursively.

        Args:
            source: Source directory
            dest: Destination directory

        Returns:
            True if copied successfully
        """
        if not source.exists():
            raise FileOperationError(f"Source directory does not exist: {source}")

        try:
            if dest.exists():
                # Merge into existing directory
                for item in source.rglob("*"):
                    if item.is_file():
                        rel_path = item.relative_to(source)
                        dest_file = dest / rel_path
                        self.copy_file(item, dest_file)
            else:
                shutil.copytree(source, dest)
                self.created_dirs.append(dest)
            return True
        except Exception as e:
            raise FileOperationError(f"Failed to copy directory {source} to {dest}: {e}")

    def rollback(self) -> int:
        """
        Rollback all operations performed.

        Returns:
            Number of operations rolled back
        """
        count = 0

        # Remove created files
        for path in reversed(self.created_files):
            if path.exists():
                path.unlink()
                count += 1

        # Remove created directories (only if empty)
        for path in reversed(self.created_dirs):
            try:
                if path.exists() and not any(path.iterdir()):
                    path.rmdir()
                    count += 1
            except OSError:
                # Directory not empty, skip
                pass

        # Restore backups
        for original, backup in self.backed_up_files:
            if backup.exists():
                shutil.move(backup, original)
                count += 1

        # Clear tracking lists
        self.created_files.clear()
        self.created_dirs.clear()
        self.backed_up_files.clear()

        return count

    def commit(self):
        """
        Commit operations (clear tracking, remove backups).
        """
        # Remove backup files
        for _, backup in self.backed_up_files:
            if backup.exists():
                backup.unlink()

        # Clear tracking
        self.created_files.clear()
        self.created_dirs.clear()
        self.backed_up_files.clear()


def ensure_directory(path: Path) -> Path:
    """Ensure a directory exists, creating if necessary."""
    path.mkdir(parents=True, exist_ok=True)
    return path


def safe_write(path: Path, content: str) -> None:
    """Safely write content to a file."""
    ops = FileOperations()
    ops.write_file(path, content, backup=True)
    ops.commit()


def read_file_if_exists(path: Path) -> Optional[str]:
    """Read a file if it exists, return None otherwise."""
    if not path.exists():
        return None
    with open(path, 'r') as f:
        return f.read()
