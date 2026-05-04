# Moonshadow NVR - Pending Tasks

## Next Session / Upcoming Features
- [ ] **Scheduled AI Training**: Add scheduled/cron functionality to automate the AI training loops.
- [ ] **Live Mosaic Overhaul (Professional NVR Aesthetic)**:
  - [ ] **Cleaner HUD**: Only display a minimal date-time box on the live camera view.
  - [ ] **UI Polish**: Add slight transparency to HUD backgrounds and improve the overall design for a professional look.
  - [ ] **Grid Expansions**: Add more screen layout options to increase the number of cameras that can be visualized simultaneously.

## AI/ML Training

### License Plate Recognition (LPR) (Completed)
- [x] **Train AI for Chilean license plates**
  - [x] Collect more Chilean plate samples with varied lighting conditions
  - [x] Generate synthetic data with different fonts, angles, and backgrounds
  - [x] Train LPRNet recognition model optimized for Chilean format
  - [x] Export to ONNX format for production use
  
### Advanced AI Features
- [ ] **Continuous Face Re-ID improvement**
  - [ ] Implement incremental learning for registered identities
  - [ ] Optimize facial landmarks detection for low-light conditions
- [ ] **Behavioral Analytics**
  - [ ] Add "Loitering" alert based on Heatmap Dwell-Time
  - [ ] Detect "Falling" or "Fighting" using Pose Estimation
- [ ] **Multi-Backend Expansion**
  - [ ] Optimize for Hailo-8 NPU and Google Coral TPU
  - [ ] ARM ACL fine-tuning for Raspberry Pi 5

## Documentation
- [x] Comprehensive Codebase Audit (Optimization, Security, Code Smells)
- [ ] Add installation screenshots
- [x] Document new AI Training Hub (`lpr_training_hub.py`)
- [x] Document Multi-Backend hardware support
- [ ] Document RTSP camera compatibility list

## Testing
- [x] Multi-backend verification (NPU/GPU/CPU)
- [ ] Multi-camera stress test (>10 streams)
- [ ] Long-term stability test (7+ days)
- [ ] Disk space exhaustion recovery

## Security, Stability & Optimization (Completed)
- [x] **Optimization**: In-process video decoding for AI (removed disk I/O dependency and fixed command injection vulnerability).
- [x] **Optimization**: Vulkan compute shader improved to use bilinear interpolation for AI inputs.
- [x] **Optimization**: React LiveCamera buffer trimming frequency reduced to save CPU.
- [x] **Security**: Eliminated unsafe `.unwrap()` panics in the database/authentication module (DoS prevention).
- [x] **Stability**: Implemented dynamic AI stream fallback (Sub stream to Main stream fallback when Sub fails).
- [x] **Code Smells**: Cleaned up incomplete AI pipeline methods with explicit Error Results.

## Project Stability & Infrastructure (Completed)
- [x] **Advanced TUI Management**: Added ONVIF, RTSP URLs, Transports, and Retention config to the terminal UI.
- [x] **TUI Log Viewer**: Added `journalctl` troubleshooting logs directly into the terminal interface.
- [x] **Fix TUI configuration persistence** (Sincronización de base de datos y guardado de claves)
- [x] **Enhance TUI Security** (Enmascaramiento de contraseñas)
- [x] **UTF-8 support for TUI inputs** (Soporte para acentos y caracteres especiales)
- [x] **Restore Server Compilation** (Arreglos en retina-patch y stream.rs)
- [x] **Verify core logic with unit tests** (71 tests pasando exitosamente)
- [x] **Automate Deployment Scripts** (Instalador con detección de OS y orquestación de MediaMTX)

---
*Last updated: 2026-05-04*
