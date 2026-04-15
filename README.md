# Moonshadow NVR 🌙🛡️

[![CI](https://github.com/washaka81/Moonshadow-NVR/workflows/CI/badge.svg)](https://github.com/washaka81/Moonshadow-NVR/actions)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-blue.svg)](LICENSE.txt)

Moonshadow NVR is an intelligent, high-performance, and open-source Network Video Recorder. Based on the original Moonfire NVR by Scott Lamb, it has been evolved into a security system with built-in AI capabilities and hardware-accelerated computer vision.

## 🚀 Key Evolutions in Moonshadow NVR

### 🧠 Intelligent AI Engine
*   **Person Re-Identification (ReID)**: Recognizes and tracks individuals across multiple camera streams using OSNet.
*   **License Plate Recognition (LPR)**: Integrated LPRNet for real-time license plate detection and decoding (supporting Chinese and Chilean formats).
*   **Smart Event Correlation**: Detection events are automatically mapped to the video timeline in the UI.
*   **Adaptive Sampling**: Dynamic processing modes (Off, Low, Medium, High, Auto) to balance accuracy and power consumption.

### ⚡ Hardware Acceleration
Powered by **ONNX Runtime (`ort`)**, Moonshadow NVR automatically optimizes inference based on your hardware:
1.  **NVIDIA TensorRT / CUDA**: Maximum performance for high-end GPUs.
2.  **Intel OpenVINO**: Optimized for Intel iGPUs, CPUs, and NPUs.
3.  **CPU Fallback**: Universal support for any architecture.

### 🖥️ Modern Interfaces
*   **Interactive TUI Dashboard**: A completely redesigned terminal interface for configuration. No more boring wizards—manage cameras, directories, and users with a fluid, tabbed dashboard.
*   **Enhanced Web UI**:
    *   **Live Multiview**: View multiple streams simultaneously.
    *   **AI Timelines**: Recordings now feature "AI Chips" (🚗 Plates, 👤 Persons) for instant visual navigation of events.
    *   **Autodetect RTSP**: One-click camera setup—the system automatically scans for valid RTSP paths.

---

## 📸 Screenshots

*Coming soon: Updated screenshots of the TUI Dashboard and AI Timeline.*

---

## ⚙️ Core Features
*   **Efficient Recording**: Saves H.264 streams directly to disk without re-encoding, keeping CPU usage extremely low (e.g., <10% on a Raspberry Pi 2 for 6 streams).
*   **Hybrid Storage**: Video frames are stored in a simple directory structure, while metadata is managed in a high-performance SQLite database.
*   **On-the-fly MP4**: Construct `.mp4` files for any time range instantly for export or viewing.
*   **RTSP Robustness**: Built-in "Retina Patch" to handle non-standard RTSP headers from various camera manufacturers (Dahua, Hikvision, etc.).

## 📖 Documentation

*   [**Installation Guide**](guide/install.md)
*   [**Building from Source**](guide/build.md)
*   [**UI Development**](guide/developing-ui.md)
*   [**Securing Your NVR**](guide/secure.md)
*   [**API Reference**](ref/api.md)
*   [**Configuration Reference**](ref/config.md)

## 🤝 Contributing
Contributions are welcome! Whether it's adding new AI models, improving the TUI, or enhancing the web interface, please see our [Contributing Guide](CONTRIBUTING.md).

## 📄 License
Moonshadow NVR is licensed under [GPL-3.0-or-later](LICENSE.txt) with a linking exception for OpenSSL.

---
*Started by Scott Lamb & evolved by the Moonshadow Community.*
