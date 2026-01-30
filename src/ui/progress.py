"""
Progress bar and status display utilities.
"""

import sys
import time
from typing import Optional
from .colors import Colors


class ProgressBar:
    """Simple progress bar for terminal output."""

    def __init__(
        self,
        total: int,
        width: int = 40,
        prefix: str = "",
        suffix: str = "",
        fill: str = "█",
        empty: str = "░"
    ):
        self.total = total
        self.width = width
        self.prefix = prefix
        self.suffix = suffix
        self.fill = fill
        self.empty = empty
        self.current = 0

    def update(self, current: Optional[int] = None, suffix: Optional[str] = None):
        """Update the progress bar."""
        if current is not None:
            self.current = current
        else:
            self.current += 1

        if suffix is not None:
            self.suffix = suffix

        percent = self.current / self.total
        filled_length = int(self.width * percent)
        bar = self.fill * filled_length + self.empty * (self.width - filled_length)

        line = f"\r{self.prefix} [{bar}] {percent:.0%} {self.suffix}"
        sys.stdout.write(line)
        sys.stdout.flush()

    def complete(self, message: str = "Done"):
        """Mark progress as complete."""
        self.update(self.total, message)
        print()


class Spinner:
    """Simple spinner for long operations."""

    FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

    def __init__(self, message: str = ""):
        self.message = message
        self.frame_index = 0

    def spin(self):
        """Display next spinner frame."""
        frame = self.FRAMES[self.frame_index % len(self.FRAMES)]
        sys.stdout.write(f"\r{Colors.CYAN}{frame}{Colors.RESET} {self.message}")
        sys.stdout.flush()
        self.frame_index += 1

    def update(self, message: str):
        """Update spinner message."""
        self.message = message
        self.spin()

    def succeed(self, message: Optional[str] = None):
        """Mark as success."""
        msg = message or self.message
        sys.stdout.write(f"\r{Colors.GREEN}✓{Colors.RESET} {msg}\n")
        sys.stdout.flush()

    def fail(self, message: Optional[str] = None):
        """Mark as failure."""
        msg = message or self.message
        sys.stdout.write(f"\r{Colors.RED}✗{Colors.RESET} {msg}\n")
        sys.stdout.flush()

    def warn(self, message: Optional[str] = None):
        """Mark as warning."""
        msg = message or self.message
        sys.stdout.write(f"\r{Colors.YELLOW}⚠{Colors.RESET} {msg}\n")
        sys.stdout.flush()


def print_checklist(items: list, title: str = "Setup Status"):
    """
    Print a checklist of items.

    Args:
        items: List of (message, status) tuples where status is 'done', 'pending', or 'skip'
        title: Checklist title
    """
    print(f"\n{Colors.BOLD}{title}{Colors.RESET}")
    print("-" * 40)

    for message, status in items:
        if status == "done":
            marker = f"{Colors.GREEN}✓{Colors.RESET}"
        elif status == "pending":
            marker = f"{Colors.YELLOW}○{Colors.RESET}"
        elif status == "skip":
            marker = f"{Colors.DIM}○{Colors.RESET}"
        else:
            marker = f"{Colors.RED}✗{Colors.RESET}"

        print(f"  {marker} {message}")

    print()


def simulate_progress(message: str, duration: float = 1.0, steps: int = 20):
    """Simulate progress for operations without real progress feedback."""
    bar = ProgressBar(steps, prefix=message)

    for i in range(steps):
        time.sleep(duration / steps)
        bar.update()

    bar.complete()
