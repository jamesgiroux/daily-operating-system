"""
ANSI color codes for terminal output.
"""


class Colors:
    """ANSI color codes for terminal styling."""

    # Reset
    RESET = "\033[0m"

    # Regular colors
    BLACK = "\033[30m"
    RED = "\033[31m"
    GREEN = "\033[32m"
    YELLOW = "\033[33m"
    BLUE = "\033[34m"
    MAGENTA = "\033[35m"
    CYAN = "\033[36m"
    WHITE = "\033[37m"

    # Bright colors
    BRIGHT_BLACK = "\033[90m"
    BRIGHT_RED = "\033[91m"
    BRIGHT_GREEN = "\033[92m"
    BRIGHT_YELLOW = "\033[93m"
    BRIGHT_BLUE = "\033[94m"
    BRIGHT_MAGENTA = "\033[95m"
    BRIGHT_CYAN = "\033[96m"
    BRIGHT_WHITE = "\033[97m"

    # Background colors
    BG_BLACK = "\033[40m"
    BG_RED = "\033[41m"
    BG_GREEN = "\033[42m"
    BG_YELLOW = "\033[43m"
    BG_BLUE = "\033[44m"
    BG_MAGENTA = "\033[45m"
    BG_CYAN = "\033[46m"
    BG_WHITE = "\033[47m"

    # Text styles
    BOLD = "\033[1m"
    DIM = "\033[2m"
    ITALIC = "\033[3m"
    UNDERLINE = "\033[4m"


def colorize(text: str, color: str) -> str:
    """Apply color to text."""
    return f"{color}{text}{Colors.RESET}"


def success(text: str) -> str:
    """Green success text."""
    return colorize(text, Colors.GREEN)


def error(text: str) -> str:
    """Red error text."""
    return colorize(text, Colors.RED)


def warning(text: str) -> str:
    """Yellow warning text."""
    return colorize(text, Colors.YELLOW)


def info(text: str) -> str:
    """Cyan info text."""
    return colorize(text, Colors.CYAN)


def header(text: str) -> str:
    """Bold header text."""
    return colorize(text, Colors.BOLD)


def dim(text: str) -> str:
    """Dimmed text."""
    return colorize(text, Colors.DIM)


def highlight(text: str) -> str:
    """Highlighted text (bold cyan)."""
    return f"{Colors.BOLD}{Colors.CYAN}{text}{Colors.RESET}"


def bold(text: str) -> str:
    """Bold text."""
    return colorize(text, Colors.BOLD)


def box(title: str, content: str, width: int = 63) -> str:
    """Create a boxed section."""
    border = Colors.CYAN + '-' * width + Colors.RESET
    top = f"+{border}+"
    bottom = f"+{border}+"

    lines = [top]
    lines.append(f"|  {Colors.BOLD}{title}{Colors.RESET}" + ' ' * (width - len(title) - 1) + '|')
    lines.append(f"|{' ' * (width + 2)}|")

    for line in content.split('\n'):
        # Pad line to width (approximate - doesn't account for ANSI codes)
        padding = max(0, width - len(line))
        lines.append(f"|  {line}{' ' * padding}|")

    lines.append(bottom)
    return '\n'.join(lines)
