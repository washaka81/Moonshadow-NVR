# Moonshadow NVR 🌙🛡️
### *The Intelligent, High-Performance Network Video Recorder for Linux*

[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE.txt)
[![Rust](https://img.shields.io/badge/Rust-1.91+-orange.svg)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/UI-React_18-61dafb.svg)](https://reactjs.org/)
[![AI-Powered](https://img.shields.io/badge/AI-YOLOv8_%7C_LPRNet-green.svg)](#-integrated-ai-acceleration)

**Moonshadow NVR** is a high-performance, open-source **Network Video Recorder (NVR)** and **Surveillance System** engineered for speed and intelligence. Built with a **Rust** core and a **React** frontend, it delivers rock-solid RTSP recording, ultra-low latency **WebRTC** viewing, and native hardware-accelerated AI for real-time **Object Detection** and **License Plate Recognition (LPR)**.

---

## ✨ Key Features

- **🚀 High-Performance Rust Core:** Optimized direct-to-disk recording (H.264/H.265) and efficient memory management, ensuring minimal CPU overhead even with multiple high-bitrate 4K streams.
- **🧠 Integrated AI Acceleration:** Native, multi-backend support for **ONNX Runtime**, **OpenVINO (NPU/GPU/CPU)**, **ARM ACL (Compute Library)**, **Vulkan Compute**, and **NVIDIA CUDA**. Powering real-time YOLOv8 object detection, Chilean/Universal LPR, Face ID, and **Suspicious Behavior Heatmaps (Experimental)**.
- **📡 Low-Latency Streaming:** Seamless multi-camera live viewing with sub-second latency via **WebRTC** and **MediaMTX** proxy integration.
- **🎮 ONVIF & PTZ Control:** Native support for ONVIF device discovery (SSDP) and real-time PTZ (Pan-Tilt-Zoom) controls directly from the live view interface.
- **📊 Real-Time System Monitor:** Detailed hardware telemetry (CPU Cores, RAM/Swap, Disk IO, GPU/NPU/VRAM load, and Temperatures) accessible directly from your web dashboard.
- **💻 Interactive TUI Configuration:** A robust **Terminal User Interface** to manage cameras, users, storage pools, and granular AI settings (Individual feature toggles, NPU/TPU preference).
- **🌐 Modern Web Dashboard:** Sleek, responsive React-based interface with intelligent timeline playback, AI event filtering (Person, Vehicle, Plate, Face), and real-time behavior heatmaps.
- **🔄 Dynamic LPR Training:** Automated feedback loop that uses real captures and **Synthetic Plate Generation** (specifically optimized for Chilean formats) to continuously improve recognition accuracy.

---

## 🚀 Advanced AI Features

### 🔍 Suspicious Behavior Heatmap (Experimental)
The system now includes a real-time occupancy heatmap. By tracking person presence over time, it identifies "hotspots" of activity. If a person stays in a specific area longer than a predefined threshold, the system generates a `suspicious_behavior` event. *Note: This feature is currently in experimental stage.*

### 🇨🇱 Chilean LPR Synthetic Training
Retraining your LPR model is now easier with the integrated synthetic data generator. 
```bash
# Generate 500 synthetic Chilean license plates for training
source models/venv/bin/activate
python3 lpr_training_hub.py generate --count 500
```
This tool creates realistic variations of Chilean plates (ABCD-12, AB-1234, ABC-123) to harden the model against different lighting and angles.

---

## 🏗️ Tech Stack

- **Backend:** [Rust](https://www.rust-lang.org/) (Hyper, Tokio, Rusqlite, ONNX Runtime)
- **Frontend:** [React 18](https://reactjs.org/), [TypeScript](https://www.typescriptlang.org/), [Material UI (MUI)](https://mui.com/)
- **Streaming:** [MediaMTX](https://github.com/bluenviron/mediamtx), WebRTC, RTSP
- **AI/ML:** YOLOv8, LPRNet, OpenVINO, Vulkan, CUDA
- **Storage:** SQLite (Metadata), Direct MP4 (Video)

---

## 🛠️ Installation and Setup (All Linux Distributions)

Moonshadow NVR provides a robust installer script that automatically detects your Linux distribution (supports **Arch/CachyOS**, **Debian/Ubuntu**, and **Fedora/RHEL**), installs the required dependencies, builds the server with the bundled UI, downloads the AI models, and configures the systemd services.

### 1. Automated Installation

Clone the repository and run the installer script with root privileges. The script will handle everything from dependency installation (Rust, Node.js, system libraries) to building the project and setting up the `moonshadow-nvr` user and `systemd` service.

```bash
git clone https://github.com/washaka81/Moonshadow-NVR.git
cd Moonshadow-NVR
sudo ./install.sh
```

### 2. Fast Configuration

Use the built-in Terminal UI to setup your surveillance network in seconds.

```bash
./configure-tui.sh
```

### 3. Run and Monitor

**For Production (Systemd):**
The installer automatically sets up a systemd service. You can enable and start it:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now moonshadow-nvr
```

**For Development / Manual Testing:**
You can start the server directly using the provided robust start script, which also handles starting the MediaMTX proxy in the background:

```bash
# Start the server (will build automatically if not found)
./start-server.sh
```

**Access your Dashboard at:** `http://<server-ip>:8080` (Default login: `admin` / `admin`)

---

## 📸 Interface Preview
*(Coming soon: Updated screenshots of the Mosaic Multiview, AI Timeline, and TUI Manager)*

---

## 📁 Project Structure

- `/server`: Core NVR engine, RTSP handler, and REST API.
- `/ui`: Modern React frontend and static assets.
- `/bin`: Orchestrated third-party binaries (MediaMTX).
- `/models`: Dynamic directory for ONNX AI models.

---

## 🤝 Contributing & Support

Moonshadow NVR is an evolving project. We welcome contributions, bug reports, and feature requests. Check out [CONTRIBUTING.md](CONTRIBUTING.md) to get started.

## 📄 License

Licensed under [GPL-3.0-or-later](LICENSE.txt) with a linking exception for OpenSSL.

---

*Evolved by the Moonshadow Community.*

[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/I2I21X53HM)
