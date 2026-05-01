#!/bin/bash
# Moonshadow NVR - TUI Configuration Launcher
# Author: Alejandro Fonda <alejandro.fonda@gmail.com>
# Note: Requires an interactive terminal (not for headless/CI use)

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo -e "${BLUE}============================================"
echo "Moonshadow NVR - Configuration (TUI)"
echo -e "============================================${NC}"
echo ""

# Try to detect database directory from config.toml (local or system)
DB_DIR=""
CONFIG_PATH=""
if [ -f "./server/config.toml" ]; then
    CONFIG_PATH="./server/config.toml"
elif [ -f "/etc/moonshadow-nvr/config.toml" ]; then
    CONFIG_PATH="/etc/moonshadow-nvr/config.toml"
fi

if [ ! -z "$CONFIG_PATH" ]; then
    DB_DIR=$(grep "dbDir" "$CONFIG_PATH" | cut -d'"' -f2)
fi

# Allow override via argument
if [ ! -z "$1" ]; then
    DB_DIR="$1"
fi

BINARY="./server/target/release/moonshadow-nvr"

# Check if binary exists
if [[ ! -f "$BINARY" ]]; then
    echo -e "${YELLOW}WARNING: Binary not found: $BINARY${NC}"
    echo "Attempting to build..."
    if [[ -d "./server" ]]; then
        (cd server && cargo build --release)
    else
        echo -e "${RED}ERROR: Server directory not found!${NC}"
        exit 1
    fi
fi

if [[ -n "$DB_DIR" ]]; then
    echo -e "Using database directory: ${GREEN}$DB_DIR${NC}"
    echo ""
    # Launch TUI config with specified db-dir
    exec "$BINARY" config --db-dir "$DB_DIR"
else
    echo -e "${YELLOW}Note: This requires an interactive terminal.${NC}"
    echo "Starting TUI configuration (default db-dir)..."
    echo -e "If this fails, edit config directly: ${BLUE}./server/config.toml${NC}"
    echo ""
    exec "$BINARY" config
fi
