#!/bin/bash
# This file is part of Moonshadow NVR.
# Copyright (C) 2025-2026 The Moonshadow NVR Authors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.
# Moonshadow NVR - LPR Training Hub Wrapper
# Ensures the script runs with the correct virtual environment python.

set -euo pipefail

# Colors
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENV_PYTHON="$SCRIPT_DIR/models/venv/bin/python3"

if [[ ! -f "$VENV_PYTHON" ]]; then
    echo -e "${RED}[ERROR]${NC} Virtual environment python not found at $VENV_PYTHON"
    echo "Please ensure the virtual environment is installed in models/venv/"
    exit 1
fi

# Pass all arguments to the actual python script
exec "$VENV_PYTHON" "$SCRIPT_DIR/lpr_training_hub.py" "$@"
