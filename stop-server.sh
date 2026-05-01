#!/bin/bash
# Moonshadow NVR - Stop Server and Services
# Author: Alejandro Fonda <alejandro.fonda@gmail.com>

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${RED}============================================"
echo "Moonshadow NVR - Stopping Services"
echo -e "============================================${NC}"
echo ""

# Function to stop a process by name pattern
stop_process() {
    local pattern=$1
    local name=$2
    
    echo -e "${YELLOW}Stopping $name...${NC}"
    if pgrep -f "$pattern" > /dev/null; then
        pkill -f "$pattern"
        sleep 1
        if pgrep -f "$pattern" > /dev/null; then
            echo -e "  Force stopping $name..."
            pkill -9 -f "$pattern"
        fi
        echo -e "  ${GREEN}OK: $name stopped${NC}"
    else
        echo -e "  ${NC}No $name process found${NC}"
    fi
}

# Stop processes
stop_process "moonshadow-nvr" "Moonshadow NVR server"
stop_process "mediamtx" "MediaMTX server"

# Clean up lock files
echo -e "${YELLOW}Cleaning up lock files...${NC}"
find /tmp -name "moonshadow-*.lock" -delete 2>/dev/null || true
echo -e "  ${GREEN}OK: Cleanup complete${NC}"

echo -e "\n${GREEN}All services have been requested to stop.${NC}"

# Verify
if pgrep -f "moonshadow|mediamtx" > /dev/null; then
    echo -e "${RED}WARNING: Some processes are still active:${NC}"
    pgrep -af "moonshadow|mediamtx"
else
    echo -e "${GREEN}SUCCESS: All services stopped successfully.${NC}"
fi
echo ""
