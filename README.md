# Moonshadow NVR 🌙🛡️

[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE.txt)
[![Rust](https://img.shields.io/badge/Rust-1.91+-orange.svg)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/UI-React_18-61dafb.svg)](https://reactjs.org/)

**Moonshadow NVR** is a high-performance, intelligent Network Video Recorder built with **Rust** and **React**. Designed to run on modern Linux systems, it offers rock-solid RTSP recording, ultra-low latency WebRTC viewing, and integrated hardware-accelerated AI for object detection and License Plate Recognition (LPR).

---

## ✨ Features

- **🚀 High-Performance Rust Core:** Direct-to-disk recording and efficient memory management ensuring low CPU overhead.
- **🧠 Integrated AI Acceleration:** Native support for OpenVINO, Vulkan Compute, and CUDA to power real-time object detection (YOLO), LPR, and Person Re-Identification.
- **💻 Interactive TUI Configuration:** A fully-featured Terminal User Interface to easily manage cameras, users, and storage pools without editing raw config files.
- **🌐 Modern Web Dashboard:** A sleek React-based UI for live multi-camera viewing, timeline playback, AI event filtering, and system monitoring.
- **📊 Advanced System Monitor:** Real-time hardware metrics (CPU Cores, RAM, Swap, Disk, GPU/VRAM load, and Temperatures) directly in the web dashboard.
- **📡 Universal Streaming:** Integrated MediaMTX proxy for seamless sub-second latency viewing via WebRTC.

---

## 🛠️ Automated Deployment (Arch Linux / CachyOS)

Moonshadow NVR includes a robust suite of shell scripts to automate deployment, build processes, and service orchestration on modern Arch-based distributions.

### 1. Installation

Clone the repository and run the automated installer. The installer detects your OS, installs necessary dependencies (including NVIDIA/CUDA support if hardware is detected), builds both the Rust backend and React frontend, and sets up a `systemd` service.

```bash
git clone https://github.com/washaka81/Moonshadow-NVR.git
cd Moonshadow-NVR
sudo ./install.sh
```

### 2. Configuration

Moonshadow comes with a built-in Terminal UI (TUI) to easily configure your cameras, users, and AI hardware settings.

```bash
./configure-tui.sh
```

### 3. Running the Server

If you used `./install.sh`, the NVR is already configured as a systemd service:

```bash
sudo systemctl enable moonshadow-nvr
sudo systemctl start moonshadow-nvr
```

To run it manually (for debugging or development), use the start script which automatically manages the MediaMTX proxy, checks AI models, and sets process priorities:

```bash
./start-server.sh
```

**Access the Web Interface at:** `http://<your-server-ip>:8080`

---

## 📁 Repository Structure

- `server/`: The core Rust application handling RTSP ingestion, MP4 writing, AI inference via ONNX Runtime, and the HTTP API.
- `ui/`: The modern React/Material-UI frontend dashboard.
- `bin/`: Pre-compiled or downloaded binaries (e.g., MediaMTX).
- `models/`: *[Optional]* Directory to place `.onnx` models (`yolov8n.onnx`, `LPRNet.onnx`) for AI acceleration.

---

## 🤝 Contributing

We welcome contributions! Please feel free to submit Pull Requests or open Issues to discuss potential improvements. See [CONTRIBUTING.md](CONTRIBUTING.md) for more details.

## 📄 License

Moonshadow NVR is licensed under [GPL-3.0-or-later](LICENSE.txt) with a linking exception for OpenSSL.

---

*Started by Scott Lamb & evolved by the Moonshadow Community.*

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/I2I21X53HM)
