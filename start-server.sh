#!/bin/bash
# This file is part of Moonshadow NVR.
# Copyright (C) 2025 The Moonshadow NVR Authors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.
# Moonshadow NVR - Start Server with AI and LPR
# Author: Alejandro Fonda <alejandro.fonda@gmail.com>

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

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
echo -e "${YELLOW}Checking AI models...${NC}"
AI_ARGS=""
for model_path in "$MODEL" "$LPR_MODEL" "$REID_MODEL"; do
    if [[ -f "$model_path" ]]; then
        echo -e "  Found: $model_path"
    else
        echo -e "  ${RED}Warning: Model not found ($model_path)${NC}"
    fi
done

if [[ -f "$MODEL" ]]; then AI_ARGS="$AI_ARGS --model=$MODEL"; fi
if [[ -f "$LPR_MODEL" ]]; then AI_ARGS="$AI_ARGS --lpr-model=$LPR_MODEL"; fi
if [[ -f "$REID_MODEL" ]]; then AI_ARGS="$AI_ARGS --reid-model=$REID_MODEL"; fi

if [[ -z "$AI_ARGS" ]]; then
    echo -e "${YELLOW}Starting without AI acceleration. Please download ONNX models for intelligent features.${NC}"
else
    AI_ARGS="$AI_ARGS --ai-mode=$AI_MODE --hardware-acceleration=$HARDWARE_ACCEL --optimize-for-device=$OPTIMIZE_DEVICE"
fi

# Check for binary
BINARY="./server/target/release/moonshadow-nvr"
if [[ ! -f "$BINARY" ]]; then
    echo -e "${YELLOW}Binary not found at $BINARY. Attempting to build...${NC}"
    if [[ -d "./server" ]]; then
        (cd server && cargo build --release)
    else
        echo -e "${RED}ERROR: Server directory not found!${NC}"
        exit 1
    fi
fi

if [[ ! -f "$BINARY" ]]; then
    echo -e "${RED}ERROR: Failed to find or build the server binary.${NC}"
    exit 1
fi

echo -e "\n${GREEN}Server Configuration:${NC}"
echo "  Config:     $CONFIG"
echo "  AI Enabled: $(if [[ -n "$AI_ARGS" ]]; then echo "Yes"; else echo "No"; fi)"
echo ""

# Cleanup function to kill background processes on exit
cleanup() {
    echo -e "\n${YELLOW}Shutting down background services...${NC}"
    # Kill background jobs (mediamtx)
    jobs -p | xargs -r kill 2>/dev/null || true
    exit 0
}

# Trap SIGINT (Ctrl+C) and SIGTERM
trap cleanup SIGINT SIGTERM

# Start MediaMTX if available
MEDIAMTX="./bin/mediamtx"
if [[ -f "$MEDIAMTX" ]]; then
    echo -e "${YELLOW}Starting MediaMTX server...${NC}"
    # Stop any existing mediamtx to avoid port conflicts
    pkill -f "mediamtx" 2>/dev/null || true
    sleep 1
    
    if [[ -f "./mediamtx.yml" ]]; then
        "$MEDIAMTX" ./mediamtx.yml > /dev/null 2>&1 &
    else
        "$MEDIAMTX" > /dev/null 2>&1 &
    fi
    echo -e "  ${GREEN}OK: MediaMTX started in background (PID: $!)${NC}"
fi

# Set process priority if possible (requires sudo or CAP_SYS_NICE)
if [[ "$EUID" -eq 0 ]]; then
    NICE_LEVEL=-10
    echo -e "${YELLOW}Setting process priority to nice=$NICE_LEVEL (Running as root)${NC}"
    # Word splitting intentionally allowed for AI_ARGS
    nice -n $NICE_LEVEL "$BINARY" run --config="$CONFIG" $AI_ARGS
else
    echo -e "${YELLOW}Starting server with normal priority (Non-root user)${NC}"
    "$BINARY" run --config="$CONFIG" $AI_ARGS
fi

# Cleanup on normal exit
cleanup
