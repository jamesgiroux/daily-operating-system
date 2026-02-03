#!/bin/bash
#
# Daily Operating System - Setup Wizard Launcher
#
# Double-click this file to launch the web-based setup wizard.
# The wizard guides you through setting up your productivity system.
#
# Requirements: Node.js 18+ (will prompt to download if missing)
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the directory containing this script
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SERVER_DIR="$SCRIPT_DIR/server"

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║   Daily Operating System Setup Wizard    ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════╝${NC}"
echo ""

# Check for Node.js
if ! command -v node &> /dev/null; then
    echo -e "${RED}Node.js is required but not installed.${NC}"
    echo ""
    echo "Would you like to open the download page? (y/n)"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        open "https://nodejs.org/"
    fi
    echo ""
    echo "After installing Node.js, run this script again."
    echo ""
    echo "Press any key to exit..."
    read -n 1
    exit 1
fi

# Check Node.js version (need 18+)
NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt 18 ]; then
    echo -e "${YELLOW}Warning: Node.js 18+ recommended. You have v$(node -v)${NC}"
    echo "The wizard may not work correctly with older versions."
    echo ""
fi

echo -e "${GREEN}✓${NC} Node.js $(node -v) found"

# Check for npm
if ! command -v npm &> /dev/null; then
    echo -e "${RED}npm is required but not installed.${NC}"
    echo "npm usually comes with Node.js. Please reinstall Node.js."
    echo ""
    echo "Press any key to exit..."
    read -n 1
    exit 1
fi

echo -e "${GREEN}✓${NC} npm $(npm -v) found"

# Check for Python (required for core initialization)
if command -v python3 &> /dev/null; then
    echo -e "${GREEN}✓${NC} Python $(python3 --version | cut -d' ' -f2) found"
    PYTHON_CMD="python3"
elif command -v python &> /dev/null; then
    echo -e "${GREEN}✓${NC} Python $(python --version 2>&1 | cut -d' ' -f2) found"
    PYTHON_CMD="python"
else
    echo -e "${YELLOW}!${NC} Python not found (optional - some features may be limited)"
    PYTHON_CMD=""
fi

# Initialize DailyOS core (~/.dailyos)
CORE_DIR="$HOME/.dailyos"
if [ ! -d "$CORE_DIR" ] || [ ! -f "$CORE_DIR/VERSION" ]; then
    echo ""
    echo -e "${BLUE}Initializing DailyOS core...${NC}"

    if [ -n "$PYTHON_CMD" ]; then
        # Use Python to initialize core
        $PYTHON_CMD -c "
import sys
sys.path.insert(0, '$SCRIPT_DIR/src')
from version import initialize_core
from pathlib import Path
success, msg = initialize_core(Path('$SCRIPT_DIR'))
print(msg)
sys.exit(0 if success else 1)
" 2>/dev/null

        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✓${NC} Core initialized at $CORE_DIR"
        else
            echo -e "${YELLOW}!${NC} Could not initialize core via Python, using fallback..."
            # Fallback: manual copy
            mkdir -p "$CORE_DIR"
            cp "$SCRIPT_DIR/VERSION" "$CORE_DIR/"
            cp "$SCRIPT_DIR/CHANGELOG.md" "$CORE_DIR/" 2>/dev/null || true
            cp -r "$SCRIPT_DIR/commands" "$CORE_DIR/" 2>/dev/null || true
            cp -r "$SCRIPT_DIR/skills" "$CORE_DIR/" 2>/dev/null || true
            cp -r "$SCRIPT_DIR/src" "$CORE_DIR/" 2>/dev/null || true
            cp "$SCRIPT_DIR/dailyos" "$CORE_DIR/" 2>/dev/null || true
            chmod +x "$CORE_DIR/dailyos" 2>/dev/null || true
            echo -e "${GREEN}✓${NC} Core initialized at $CORE_DIR (fallback method)"
        fi
    else
        # No Python - use bash fallback
        mkdir -p "$CORE_DIR"
        cp "$SCRIPT_DIR/VERSION" "$CORE_DIR/"
        cp "$SCRIPT_DIR/CHANGELOG.md" "$CORE_DIR/" 2>/dev/null || true
        cp -r "$SCRIPT_DIR/commands" "$CORE_DIR/" 2>/dev/null || true
        cp -r "$SCRIPT_DIR/skills" "$CORE_DIR/" 2>/dev/null || true
        cp -r "$SCRIPT_DIR/src" "$CORE_DIR/" 2>/dev/null || true
        cp "$SCRIPT_DIR/dailyos" "$CORE_DIR/" 2>/dev/null || true
        chmod +x "$CORE_DIR/dailyos" 2>/dev/null || true
        echo -e "${GREEN}✓${NC} Core initialized at $CORE_DIR"
    fi
else
    # Check if we need to update core
    REPO_VERSION=$(cat "$SCRIPT_DIR/VERSION" 2>/dev/null || echo "0.0.0")
    CORE_VERSION=$(cat "$CORE_DIR/VERSION" 2>/dev/null || echo "0.0.0")

    if [ "$REPO_VERSION" != "$CORE_VERSION" ]; then
        echo ""
        echo -e "${BLUE}Updating DailyOS core ($CORE_VERSION → $REPO_VERSION)...${NC}"
        cp "$SCRIPT_DIR/VERSION" "$CORE_DIR/"
        cp "$SCRIPT_DIR/CHANGELOG.md" "$CORE_DIR/" 2>/dev/null || true
        cp -r "$SCRIPT_DIR/commands" "$CORE_DIR/" 2>/dev/null || true
        cp -r "$SCRIPT_DIR/skills" "$CORE_DIR/" 2>/dev/null || true
        cp -r "$SCRIPT_DIR/src" "$CORE_DIR/" 2>/dev/null || true
        cp "$SCRIPT_DIR/dailyos" "$CORE_DIR/" 2>/dev/null || true
        chmod +x "$CORE_DIR/dailyos" 2>/dev/null || true
        echo -e "${GREEN}✓${NC} Core updated to v$REPO_VERSION"
    else
        echo -e "${GREEN}✓${NC} Core up to date (v$CORE_VERSION)"
    fi
fi

# Create dailyos CLI symlink if not exists (automatic for easy-start)
if [ ! -f "/usr/local/bin/dailyos" ] && [ -f "$CORE_DIR/dailyos" ]; then
    echo ""
    echo -e "${BLUE}Installing 'dailyos' command...${NC}"
    if sudo ln -sf "$CORE_DIR/dailyos" /usr/local/bin/dailyos 2>/dev/null; then
        echo -e "${GREEN}✓${NC} Installed 'dailyos' command"
        echo "  You can now run: dailyos start, dailyos stop, dailyos doctor, etc."
    else
        echo -e "${YELLOW}!${NC} Could not install to /usr/local/bin (need admin rights)"
        echo "    You can add this to your shell profile instead:"
        echo "    export PATH=\"\$HOME/.dailyos:\$PATH\""
    fi
fi

# Navigate to server directory
cd "$SERVER_DIR"

# Install dependencies if needed
if [ ! -d "node_modules" ]; then
    echo ""
    echo -e "${BLUE}Installing dependencies...${NC}"
    npm install --silent
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} Dependencies installed"
    else
        echo -e "${RED}Failed to install dependencies${NC}"
        echo ""
        echo "Press any key to exit..."
        read -n 1
        exit 1
    fi
fi

# Function to cleanup on exit
cleanup() {
    echo ""
    echo -e "${YELLOW}Shutting down server...${NC}"
    kill $SERVER_PID 2>/dev/null
    exit 0
}

trap cleanup SIGINT SIGTERM

# Open browser after a short delay (in background)
(
    sleep 2
    if command -v open &> /dev/null; then
        open "http://localhost:5050"
    elif command -v xdg-open &> /dev/null; then
        xdg-open "http://localhost:5050"
    fi
) &

echo ""
echo -e "${BLUE}Starting setup wizard...${NC}"
echo ""
echo "The wizard will open in your browser at: http://localhost:5050"
echo ""
echo -e "${YELLOW}Press Ctrl+C to stop the server when done.${NC}"
echo ""

# Start the server
npm start &
SERVER_PID=$!

# Wait for server process
wait $SERVER_PID
