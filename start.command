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
