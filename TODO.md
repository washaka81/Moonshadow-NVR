# Moonshadow NVR - Pending Tasks

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
- [ ] Add installation screenshots
- [x] Document new AI Training Hub (`lpr_training_hub.py`)
- [x] Document Multi-Backend hardware support
- [ ] Document RTSP camera compatibility list

## Testing
- [x] Multi-backend verification (NPU/GPU/CPU)
- [ ] Multi-camera stress test (>10 streams)
- [ ] Long-term stability test (7+ days)
- [ ] Disk space exhaustion recovery

## Project Stability & Infrastructure (Completed)
- [x] **Fix TUI configuration persistence** (Sincronización de base de datos y guardado de claves)
- [x] **Enhance TUI Security** (Enmascaramiento de contraseñas)
- [x] **UTF-8 support for TUI inputs** (Soporte para acentos y caracteres especiales)
- [x] **Restore Server Compilation** (Arreglos en retina-patch y stream.rs)
- [x] **Verify core logic with unit tests** (71 tests pasando exitosamente)
- [x] **Automate Deployment Scripts** (Instalador con detección de OS y orquestación de MediaMTX)

---
*Last updated: 2026-04-30 (Restauración completa realizada)*
