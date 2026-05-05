# AI and Hardware Acceleration Architecture

Moonshadow NVR uses a modular AI engine designed for high performance across diverse hardware.

## Inference Engine
The core is built on **ONNX Runtime**, with multiple Execution Providers (EP):
- **OpenVINO**: Primary for Intel CPUs, iGPUs, and NPUs.
- **ACL (Arm Compute Library)**: Optimized for ARM (Raspberry Pi 5).
- **Vulkan**: Cross-vendor parallel preprocessing (resizing/normalization).
- **CUDA**: For NVIDIA discrete GPUs.

## Feature Pipeline
1. **Object Detection**: YOLOv8 (Nano) for Person and Vehicle detection.
2. **LPR**: LPRNet optimized for Chilean plates.
3. **Identification (Re-ID)**: OSNet-based person re-identification embedding extraction (128x256 input).
4. **Behavioral Analysis**: Dwell-time heatmaps for Loitering detection and peak activity reporting.

## Feedback Loop (Dynamic Training)
The system captures real-world samples and uses them alongside synthetic data (generated with real country fonts) to retrain models via `lpr_training_hub.py`.
- **Scheduled Training**: Automated loops via systemd timers or cron jobs, configurable through the TUI.
- **Incremental Learning**: Identity management with sample/embedding updates for registered persons.

## Stability & Recovery
- **Disk Recovery**: Autonomous emergency rotation that triggers at 5% free space to restore 10% availability, preventing database and stream failures.
- **Hardware Fallback**: Automatic selection of the best available EP (NPU > GPU > CUDA > CPU).
