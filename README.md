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
- **🧠 Integrated AI Acceleration:** Native, multi-backend support for **ONNX Runtime**, **OpenVINO**, **Vulkan Compute**, and **NVIDIA CUDA**. Powering real-time YOLOv8 object detection, Chilean/Universal LPR, and Person Re-Identification.
- **📡 Low-Latency Streaming:** Seamless multi-camera live viewing with sub-second latency via **WebRTC** and **MediaMTX** proxy integration.
- **📊 Real-Time System Monitor:** Detailed hardware telemetry (CPU Cores, RAM/Swap, Disk IO, GPU/VRAM load, and Temperatures) accessible directly from your web dashboard.
- **💻 Interactive TUI Configuration:** A robust **Terminal User Interface** to manage cameras, users, storage pools, and AI settings without manually editing complex configuration files.
- **🌐 Modern Web Dashboard:** Sleek, responsive React-based interface with intelligent timeline playback, AI event filtering (Person, Vehicle, Plate), and easy MP4 downloads.

---

## 🏗️ Tech Stack

- **Backend:** [Rust](https://www.rust-lang.org/) (Hyper, Tokio, Rusqlite, ONNX Runtime)
- **Frontend:** [React 18](https://reactjs.org/), [TypeScript](https://www.typescriptlang.org/), [Material UI (MUI)](https://mui.com/)
- **Streaming:** [MediaMTX](https://github.com/bluenviron/mediamtx), WebRTC, RTSP
- **AI/ML:** YOLOv8, LPRNet, OpenVINO, Vulkan, CUDA
- **Storage:** SQLite (Metadata), Direct MP4 (Video)

---

## 🛠️ Automated Deployment (Arch Linux / CachyOS)

Moonshadow NVR provides a professional suite of automation scripts for installation, service management, and pre-flight AI model checks.

### 1. Installation

Our intelligent installer detects your OS (optimized for **Arch Linux** and **CachyOS**), configures hardware acceleration (auto-detects NVIDIA/Intel), builds all components, and sets up a `systemd` service.

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

The NVR is managed as a standard Linux service:

```bash
sudo systemctl enable --now moonshadow-nvr
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
