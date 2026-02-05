#!/bin/bash

# DailyOS UI Launcher
# Simple script to start the DailyOS web interface

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}"
echo "╔═══════════════════════════════════════════════════╗"
echo "║                                                   ║"
echo "║   DailyOS UI                                      ║"
echo "║                                                   ║"
echo "╚═══════════════════════════════════════════════════╝"
echo -e "${NC}"

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install
fi

# Check for port argument
PORT=${1:-5050}

# Kill any existing server on this port
if lsof -ti:$PORT > /dev/null 2>&1; then
    echo "Stopping existing server on port $PORT..."
    lsof -ti:$PORT | xargs kill -9 2>/dev/null || true
    sleep 1
fi

# Open browser after a short delay
(sleep 1 && open "http://localhost:$PORT" 2>/dev/null || true) &

# Start the server
echo -e "${GREEN}Starting server on http://localhost:$PORT${NC}"
echo ""
echo "Press Ctrl+C to stop"
echo "Or run: npm stop"
echo ""

PORT=$PORT npm start
