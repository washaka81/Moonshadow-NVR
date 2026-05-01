#!/bin/bash
# This file is part of Moonshadow NVR.
# Copyright (C) 2025 The Moonshadow NVR Authors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.
# Moonshadow NVR Installer
# Author: Alejandro Fonda <alejandro.fonda@gmail.com>
# License: GPL-3.0-or-later

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}============================================"
echo "Moonshadow NVR - System Installer"
echo "============================================${NC}"
echo ""

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}ERROR: Please run as root (sudo ./install.sh)${NC}"
    exit 1
fi

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS_NAME=$NAME
    OS_ID=$ID
    OS_LIKE=$ID_LIKE
else
    OS_NAME="Unknown"
    OS_ID="unknown"
fi

echo -e "Detected OS: ${GREEN}$OS_NAME${NC}"

# Check for Arch-based distros
if [[ "$OS_ID" == "arch" || "$OS_LIKE" == *"arch"* || "$OS_ID" == "cachyos" ]]; then
    PKG_MGR="pacman"
    INSTALL_CMD="pacman -S --noconfirm --needed"
    UPDATE_CMD="pacman -Syu --noconfirm"
else
    echo -e "${RED}ERROR: This installer currently only supports Arch Linux or derivatives (CachyOS, Manjaro, etc.)${NC}"
    echo "Please install dependencies manually for other distributions."
    exit 1
fi

# System update
echo -e "${YELLOW}[1/6] Updating system packages...${NC}"
$UPDATE_CMD

# Install base dependencies
echo -e "${YELLOW}[2/6] Installing base dependencies...${NC}"
$INSTALL_CMD \
    base-devel rust cargo nodejs npm git sqlite \
    libva libva-intel-driver intel-media-driver ocl-icd level-zero \
    onnx-runtime ncurses openssl pkgconf systemd alsa-lib ffmpeg

# Optional CUDA/NVIDIA support (check if hardware exists)
if lspci | grep -iq nvidia; then
    echo -e "${GREEN}NVIDIA GPU detected, installing CUDA support...${NC}"
    $INSTALL_CMD cuda cudnn
fi

# Install Rust toolchain if not present
if ! command -v rustc &> /dev/null; then
    echo -e "${YELLOW}Setting up Rust toolchain via rustup...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # Ensure cargo is in PATH for the current script
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# Install Node.js dependencies
echo -e "${YELLOW}[4/6] Installing UI dependencies and building frontend...${NC}"
if [[ -d "./ui" ]]; then
    (cd ui && npm install && npm run build)
else
    echo -e "${RED}WARNING: UI directory not found, skipping UI build.${NC}"
fi

# Build server
echo -e "${YELLOW}[5/6] Building Moonshadow NVR server...${NC}"
if [[ -d "./server" ]]; then
    (cd server && cargo build --release)
else
    echo -e "${RED}ERROR: Server directory not found!${NC}"
    exit 1
fi

# Create directories
echo -e "${YELLOW}[6/6] Configuring system folders and external components...${NC}"
mkdir -p /var/lib/moonshadow-nvr/{db,recordings}
mkdir -p /etc/moonshadow-nvr
mkdir -p /opt/moonshadow-nvr/{bin,models,ui}

# Download MediaMTX if missing
if [[ ! -f "./bin/mediamtx" ]]; then
    echo -e "${BLUE}Downloading MediaMTX server...${NC}"
    MTX_VER="v1.9.3"
    curl -L -o mediamtx.tar.gz "https://github.com/bluenviron/mediamtx/releases/download/${MTX_VER}/mediamtx_${MTX_VER}_linux_amd64.tar.gz"
    mkdir -p bin
    tar -xzf mediamtx.tar.gz -C bin/ mediamtx
    rm mediamtx.tar.gz
fi

# Download default AI model if missing
if [[ ! -f "models/yolov8n.onnx" ]]; then
    echo -e "${BLUE}Downloading default YOLOv8n object detection model...${NC}"
    mkdir -p models
    curl -L -o models/yolov8n.onnx "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n.onnx"
fi

# Copy files
echo "Installing binaries, default config, UI and AI models..."
cp server/target/release/moonshadow-nvr /opt/moonshadow-nvr/
cp bin/mediamtx /opt/moonshadow-nvr/bin/ 2>/dev/null || true
chmod +x /opt/moonshadow-nvr/bin/mediamtx 2>/dev/null || true
cp -r models/*.onnx /opt/moonshadow-nvr/models/ 2>/dev/null || true

# Create system user
if ! id -u moonshadow >/dev/null 2>&1; then
    useradd -r -s /bin/false -M moonshadow
fi

# Copy UI build
if [ -d "ui/dist" ]; then
    cp -r ui/dist /opt/moonshadow-nvr/ui/
fi

# Install default config if not present
if [ ! -f /etc/moonshadow-nvr/config.toml ]; then
    if [ -f "server/config.toml.example" ]; then
        cp server/config.toml.example /etc/moonshadow-nvr/config.toml
    else
        cp server/config.toml /etc/moonshadow-nvr/config.toml
    fi
fi

chown -R moonshadow:moonshadow /var/lib/moonshadow-nvr
chown -R moonshadow:moonshadow /etc/moonshadow-nvr
chown -R moonshadow:moonshadow /opt/moonshadow-nvr

# Install systemd service
echo "Generating systemd service..."
cat > /etc/systemd/system/moonshadow-nvr.service << EOF
[Unit]
Description=Moonshadow NVR Server
After=network.target

[Service]
Type=simple
WorkingDirectory=/opt/moonshadow-nvr
ExecStart=/opt/moonshadow-nvr/moonshadow-nvr run \\
    --config=/etc/moonshadow-nvr/config.toml \\
    --model=/opt/moonshadow-nvr/models/yolov8n.onnx \\
    --lpr-model=/opt/moonshadow-nvr/models/LPRNet_chilean_fixed.onnx \\
    --reid-model=/opt/moonshadow-nvr/models/osnet_x0_25_msmt17.onnx \\
    --ai-mode=high
Restart=on-failure
User=moonshadow
Group=moonshadow
StandardOutput=append:/var/log/moonshadow-nvr.log
StandardError=append:/var/log/moonshadow-nvr.log

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd
systemctl daemon-reload

echo ""
echo -e "${GREEN}============================================"
echo "Installation complete!"
echo "============================================${NC}"
echo ""
echo "Next steps:"
echo -e "1. Edit configuration: ${YELLOW}sudo nano /etc/moonshadow-nvr/config.toml${NC}"
echo -e "2. Enable service:     ${YELLOW}sudo systemctl enable moonshadow-nvr${NC}"
echo -e "3. Start service:      ${YELLOW}sudo systemctl start moonshadow-nvr${NC}"
echo ""
echo -e "Web Interface: ${BLUE}http://localhost:8080${NC}"
echo ""
