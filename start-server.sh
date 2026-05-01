#!/bin/bash
# This file is part of Moonshadow NVR.
# Copyright (C) 2025-2026 The Moonshadow NVR Authors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.
# Moonshadow NVR - Start Server with AI and LPR (Robust Version)
# Author: Alejandro Fonda <alejandro.fonda@gmail.com>

set -euo pipefail

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

log() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; exit 1; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo -e "${GREEN}============================================"
echo "Moonshadow NVR - Starting Server"
echo -e "============================================${NC}"
echo ""

# Configuration (can be overridden by environment variables)
MODEL="${MODEL:-models/yolov8n.onnx}"
LPR_MODEL="${LPR_MODEL:-models/LPRNet_chilean_fixed.onnx}"
REID_MODEL="${REID_MODEL:-models/osnet_x0_25_msmt17.onnx}"
AI_MODE="${AI_MODE:-high}"
HARDWARE_ACCEL="${HARDWARE_ACCEL:-true}"
OPTIMIZE_DEVICE="${OPTIMIZE_DEVICE:-true}"
CONFIG="${CONFIG:-./server/config.toml}"

# Check for required models and build arguments
log "Checking AI models..."
AI_ARGS=""
for model_path in "$MODEL" "$LPR_MODEL" "$REID_MODEL"; do
    if [[ -f "$model_path" ]]; then
        echo -e "  Found: $model_path"
    else
        warn "Model not found: $model_path"
    fi
done

[[ -f "$MODEL" ]] && AI_ARGS="$AI_ARGS --model=$MODEL"
[[ -f "$LPR_MODEL" ]] && AI_ARGS="$AI_ARGS --lpr-model=$LPR_MODEL"
[[ -f "$REID_MODEL" ]] && AI_ARGS="$AI_ARGS --reid-model=$REID_MODEL"

if [[ -z "$AI_ARGS" ]]; then
    warn "Starting without AI acceleration. Please download ONNX models for intelligent features."
else
    AI_ARGS="$AI_ARGS --ai-mode=$AI_MODE --hardware-acceleration=$HARDWARE_ACCEL --optimize-for-device=$OPTIMIZE_DEVICE"
fi

# Check for binary
BINARY="./server/target/release/moonshadow-nvr"
if [[ ! -f "$BINARY" ]]; then
    warn "Binary not found at $BINARY. Attempting to build..."
    if [[ -d "./server" ]]; then
        (cd server && cargo build --release) || error "Failed to build Moonshadow NVR server."
    else
        error "Server directory not found!"
    fi
fi

if [[ ! -x "$BINARY" ]]; then
    chmod +x "$BINARY"
fi

echo -e "\n${GREEN}Server Configuration:${NC}"
echo "  Config:     $CONFIG"
echo "  AI Enabled: $([[ -n "$AI_ARGS" ]] && echo "Yes" || echo "No")"
echo ""

# Cleanup function to kill background processes on exit
cleanup() {
    echo -e "\n${YELLOW}Shutting down background services...${NC}"
    # Kill background jobs (mediamtx)
    # Use -r to only kill running jobs and ignore errors if none
    jobs -p | xargs -r kill 2>/dev/null || true
    # Also ensure mediamtx is really gone if started via &
    pkill -f "mediamtx" 2>/dev/null || true
    exit 0
}

# Trap SIGINT (Ctrl+C) and SIGTERM
trap cleanup SIGINT SIGTERM

# Start MediaMTX if available
MEDIAMTX="./bin/mediamtx"
if [[ -f "$MEDIAMTX" ]]; then
    log "Starting MediaMTX server..."
    # Stop any existing mediamtx to avoid port conflicts
    pkill -f "mediamtx" 2>/dev/null || true
    sleep 1
    
    if [[ -f "./mediamtx.yml" ]]; then
        "$MEDIAMTX" ./mediamtx.yml > /dev/null 2>&1 &
    else
        "$MEDIAMTX" > /dev/null 2>&1 &
    fi
    log "MediaMTX started in background (PID: $!)"
fi

# Set process priority if possible (requires sudo or CAP_SYS_NICE)
if [[ "$EUID" -eq 0 ]]; then
    NICE_LEVEL=-10
    log "Setting process priority to nice=$NICE_LEVEL (Running as root)"
    # Word splitting intentionally allowed for AI_ARGS
    # shellcheck disable=SC2086
    nice -n $NICE_LEVEL "$BINARY" run --config="$CONFIG" $AI_ARGS
else
    log "Starting server with normal priority (Non-root user)"
    # shellcheck disable=SC2086
    "$BINARY" run --config="$CONFIG" $AI_ARGS
fi

# Cleanup on normal exit
cleanup
