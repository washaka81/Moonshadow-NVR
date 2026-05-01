#!/bin/bash
# This file is part of Moonshadow NVR.
# Copyright (C) 2025-2026 The Moonshadow NVR Authors.
# SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.
# Moonshadow NVR Robust Installer
# Author: Alejandro Fonda <alejandro.fonda@gmail.com>

set -euo pipefail

# --- Configuration ---
PROJECT_NAME="moonshadow-nvr"
INSTALL_ROOT="/opt/$PROJECT_NAME"
BIN_DIR="$INSTALL_ROOT/bin"
MODEL_DIR="$INSTALL_ROOT/models"
UI_DIR="$INSTALL_ROOT/ui"
CONFIG_DIR="/etc/$PROJECT_NAME"
DATA_DIR="/var/lib/$PROJECT_NAME"
LOG_FILE="/var/log/${PROJECT_NAME}_install.log"

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${BLUE}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; exit 1; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }

echo -e "${BLUE}============================================"
echo "    Moonshadow NVR - System Installer"
echo "============================================${NC}"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    error "Please run as root (sudo ./install.sh)"
fi

# Detect OS
if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS_ID=$ID
    OS_LIKE=${ID_LIKE:-""}
else
    error "Cannot detect OS. /etc/os-release missing."
fi

# Detect Architecture
ARCH=$(uname -m)
case "$ARCH" in
    x86_64) MTX_ARCH="amd64" ;;
    aarch64) MTX_ARCH="arm64v8" ;;
    armv7l) MTX_ARCH="armv7" ;;
    *) MTX_ARCH="amd64"; warn "Unknown architecture $ARCH, defaulting to amd64 for MediaMTX" ;;
esac

log "Detected OS: ${GREEN}${NAME:-$OS_ID}${NC}"
log "Detected Arch: ${GREEN}$ARCH${NC}"

# Define Package Manager and Packages
PKGS=""
if [[ "$OS_ID" == "arch" || "$OS_LIKE" == *"arch"* || "$OS_ID" == "cachyos" ]]; then
    PKG_MGR="pacman"
    INSTALL_CMD="pacman -S --noconfirm --needed"
    UPDATE_CMD="pacman -Syu --noconfirm"
    # Arch packages (development headers are usually included in the main package)
    PKGS="base-devel rust cargo nodejs npm git sqlite libva intel-media-driver ocl-icd intel-compute-runtime ncurses openssl pkgconf systemd alsa-lib ffmpeg protobuf vulkan-headers vulkan-icd-loader"
    
    # Optional CUDA/NVIDIA support
    if lspci | grep -iq nvidia; then
        log "NVIDIA GPU detected, adding CUDA support packages..."
        PKGS="$PKGS cuda cudnn"
    fi
elif [[ "$OS_ID" == "debian" || "$OS_ID" == "ubuntu" || "$OS_LIKE" == *"debian"* ]]; then
    PKG_MGR="apt"
    INSTALL_CMD="apt-get install -y"
    UPDATE_CMD="apt-get update"
    # Debian/Ubuntu use -dev suffixes for headers
    PKGS="build-essential curl git nodejs npm sqlite3 libsqlite3-dev libva-dev intel-media-va-driver-non-free libssl-dev pkg-config systemd libsystemd-dev libasound2-dev ffmpeg libncurses-dev ocl-icd-libopencl1 intel-level-zero-gpu intel-opencl-icd protobuf-compiler libprotobuf-dev libvulkan-dev"
    
    # Fallback if non-free driver is not available
    if ! apt-cache show intel-media-va-driver-non-free &>/dev/null; then
        warn "intel-media-va-driver-non-free not found, using standard intel-media-va-driver"
        PKGS=${PKGS/intel-media-va-driver-non-free/intel-media-va-driver}
    fi
elif [[ "$OS_ID" == "fedora" || "$OS_ID" == "rhel" || "$OS_LIKE" == *"fedora"* ]]; then
    PKG_MGR="dnf"
    INSTALL_CMD="dnf install -y"
    UPDATE_CMD="dnf check-update || true"
    # Fedora uses -devel suffixes
    PKGS="gcc gcc-c++ make curl git nodejs npm sqlite sqlite-devel libva-devel intel-media-driver openssl-devel pkgconf-pkg-config systemd-devel alsa-lib-devel ffmpeg ncurses-devel ocl-icd oneapi-level-zero intel-level-zero protobuf-compiler protobuf-devel vulkan-loader-devel"
else
    error "Unsupported distribution: $OS_ID. Please install dependencies manually."
fi

# 1. Update and Install Dependencies
log "Updating system and installing dependencies via $PKG_MGR..."
$UPDATE_CMD
$INSTALL_CMD $PKGS

# 2. Rust Setup (ensure recent version)
if ! command -v cargo &> /dev/null; then
    log "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env" || export PATH="$HOME/.cargo/bin:$PATH"
fi

# 3. pnpm Setup (Recommended for UI)
if ! command -v pnpm &> /dev/null; then
    log "pnpm not found. Installing via npm..."
    npm install -g pnpm || warn "Failed to install pnpm globally. Will attempt to use npm for UI build."
fi

# 4. Build UI
log "Building Moonshadow UI..."
if [[ -d "./ui" ]]; then
    pushd ui
    if command -v pnpm &> /dev/null; then
        pnpm install
        pnpm run build
    else
        npm install
        npm run build
    fi
    popd
else
    warn "UI directory not found, skipping UI build."
fi

# 5. Build Server
log "Building Moonshadow NVR Server..."
if [[ -d "./server" ]]; then
    pushd server
    cargo build --release
    popd
else
    error "Server directory not found!"
fi

# 6. Create Directories
log "Configuring system folders..."
mkdir -p "$BIN_DIR" "$MODEL_DIR" "$UI_DIR" "$CONFIG_DIR" "$DATA_DIR/db" "$DATA_DIR/recordings"

# 7. Download MediaMTX if missing
MTX_VER="v1.9.3"
if [[ ! -f "$BIN_DIR/mediamtx" ]]; then
    log "Downloading MediaMTX $MTX_VER for $ARCH..."
    MTX_URL="https://github.com/bluenviron/mediamtx/releases/download/${MTX_VER}/mediamtx_${MTX_VER}_linux_${MTX_ARCH}.tar.gz"
    if curl -L -o mediamtx.tar.gz "$MTX_URL"; then
        tar -xzf mediamtx.tar.gz mediamtx
        mv mediamtx "$BIN_DIR/"
        rm mediamtx.tar.gz
        chmod +x "$BIN_DIR/mediamtx"
    else
        warn "Failed to download MediaMTX. You may need to install it manually."
    fi
fi

# 8. Download default AI model if missing
if [[ ! -f "$MODEL_DIR/yolov8n.onnx" ]]; then
    log "Downloading default YOLOv8n object detection model..."
    curl -L -o "$MODEL_DIR/yolov8n.onnx" "https://github.com/ultralytics/assets/releases/download/v8.2.0/yolov8n.onnx"
fi

# 9. Install Binaries and Assets
log "Installing binaries, default config, UI and AI models..."
cp server/target/release/moonshadow-nvr "$INSTALL_ROOT/"
if [[ -d "ui/dist" ]]; then
    cp -r ui/dist/* "$UI_DIR/"
fi

# 10. Create System User
if ! id -u moonshadow >/dev/null 2>&1; then
    log "Creating moonshadow system user..."
    useradd -r -s /bin/false -M moonshadow || true
fi

# 11. Install Default Config
if [[ ! -f "$CONFIG_DIR/config.toml" ]]; then
    if [[ -f "server/config.toml.example" ]]; then
        cp server/config.toml.example "$CONFIG_DIR/config.toml"
    elif [[ -f "server/config.toml" ]]; then
        cp server/config.toml "$CONFIG_DIR/config.toml"
    fi
fi

# 12. Set Permissions
log "Setting permissions..."
chown -R moonshadow:moonshadow "$DATA_DIR"
chown -R moonshadow:moonshadow "$CONFIG_DIR"
chown -R moonshadow:moonshadow "$INSTALL_ROOT"

# 13. Install systemd service
log "Generating systemd service..."
cat > /etc/systemd/system/moonshadow-nvr.service << EOF
[Unit]
Description=Moonshadow NVR Server
After=network.target

[Service]
Type=simple
WorkingDirectory=$INSTALL_ROOT
ExecStart=$INSTALL_ROOT/moonshadow-nvr run \\
    --config=$CONFIG_DIR/config.toml \\
    --model=$MODEL_DIR/yolov8n.onnx \\
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
success "Installation complete!"
echo "============================================"
echo -e "Next steps:"
echo -e "1. Edit configuration: ${YELLOW}sudo nano $CONFIG_DIR/config.toml${NC}"
echo -e "2. Enable service:     ${YELLOW}sudo systemctl enable moonshadow-nvr${NC}"
echo -e "3. Start service:      ${YELLOW}sudo systemctl start moonshadow-nvr${NC}"
echo ""
echo -e "Web Interface: ${BLUE}http://localhost:8080${NC}"
echo "============================================"
echo ""
