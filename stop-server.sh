#!/bin/bash
# This file is part of Moonshadow NVR.
# Copyright (C) 2025-2026 The Moonshadow NVR Authors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.
# Moonshadow NVR - Stop Server and Services (Robust Version)
# Author: Alejandro Fonda <alejandro.fonda@gmail.com>

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; exit 1; }

echo -e "${RED}============================================"
echo "Moonshadow NVR - Stopping Services"
echo -e "============================================${NC}"
echo ""

# Function to stop a process by name pattern
stop_process() {
    local pattern=$1
    local name=$2
    
    log "Stopping $name..."
    if pgrep -f "$pattern" > /dev/null; then
        pkill -f "$pattern" || true
        # Wait up to 5 seconds for process to exit
        for i in {1..5}; do
            if ! pgrep -f "$pattern" > /dev/null; then
                log "  $name stopped"
                return 0
            fi
            sleep 1
        done
        
        warn "  $name did not stop gracefully, force killing..."
        pkill -9 -f "$pattern" || true
        log "  $name force stopped"
    else
        echo "  No $name process found"
    fi
}

# Stop processes
stop_process "moonshadow-nvr" "Moonshadow NVR server"
stop_process "mediamtx" "MediaMTX server"

# Clean up lock files
log "Cleaning up lock files..."
find /tmp -name "moonshadow-*.lock" -delete 2>/dev/null || true
echo "  Cleanup complete"

echo -e "\n${GREEN}All services have been requested to stop.${NC}"

# Verify
if pgrep -f "moonshadow|mediamtx" > /dev/null; then
    warn "Some processes are still active:"
    pgrep -af "moonshadow|mediamtx" || true
else
    success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
    success "All services stopped successfully."
fi
echo ""
