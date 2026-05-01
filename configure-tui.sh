#!/bin/bash
# This file is part of Moonshadow NVR.
# Copyright (C) 2025-2026 The Moonshadow NVR Authors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.
# Moonshadow NVR - TUI Configuration Launcher (Robust Version)
# Author: Alejandro Fonda <alejandro.fonda@gmail.com>

set -euo pipefail

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "${BLUE}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo -e "${BLUE}============================================"
echo "Moonshadow NVR - Configuration (TUI)"
echo -e "============================================${NC}"
echo ""

# --- Robust Path Detection ---
# 1. Check if provided as argument
DB_DIR="${1:-}"

# 2. Try to detect from config.toml if not provided
if [ -z "$DB_DIR" ]; then
    CONFIG_PATH=""
    if [ -f "./server/config.toml" ]; then
        CONFIG_PATH="./server/config.toml"
    elif [ -f "/etc/moonshadow-nvr/config.toml" ]; then
        CONFIG_PATH="/etc/moonshadow-nvr/config.toml"
    fi

    if [ -n "$CONFIG_PATH" ]; then
        # Safely extract dbDir from TOML (assuming simple string format)
        DB_DIR=$(grep "^dbDir" "$CONFIG_PATH" | head -n 1 | cut -d'"' -f2 || echo "")
    fi
fi

# 3. Establish a robust default for "plug and play"
if [ -z "$DB_DIR" ]; then
    if [ -w "/var/lib/moonshadow-nvr" ]; then
        DB_DIR="/var/lib/moonshadow-nvr/db"
    elif [ -d "/var/lib/moonshadow-nvr" ] && [ -w "/var/lib/moonshadow-nvr" ]; then
         DB_DIR="/var/lib/moonshadow-nvr/db"
    else
        # Fallback to user-local path for non-root/portable use
        USER_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/moonshadow-nvr"
        DB_DIR="$USER_DATA_DIR/db"
        log "System path not writable, using user-local path: ${YELLOW}$DB_DIR${NC}"
        mkdir -p "$USER_DATA_DIR"
    fi
fi

BINARY="./server/target/release/moonshadow-nvr"

# Check if binary exists, attempt build if missing
if [[ ! -f "$BINARY" ]]; then
    warn "Binary not found: $BINARY"
    log "Attempting to build..."
    if [[ -d "./server" ]]; then
        (cd server && cargo build --release) || error "Failed to build Moonshadow NVR server."
    else
        error "Server directory not found!"
    fi
fi

if [[ ! -x "$BINARY" ]]; then
    chmod +x "$BINARY"
fi

log "Using database directory: ${GREEN}$DB_DIR${NC}"
echo ""

# Ensure the directory exists or can be created
mkdir -p "$DB_DIR" 2>/dev/null || warn "Could not create directory $DB_DIR, it might require sudo."

# Launch TUI config
# Note: exec replaces the shell process
if [[ -t 0 ]]; then
    exec "$BINARY" config --db-dir "$DB_DIR"
else
    error "This script requires an interactive terminal (TTY)."
fi
