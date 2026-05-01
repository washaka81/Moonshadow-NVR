# Moonshadow NVR - Pending Tasks

## AI/ML Training

### License Plate Recognition (LPR)
- [ ] **Train AI for Chilean license plates**
  - Collect more Chilean plate samples with varied lighting conditions
  - Generate synthetic data with different fonts, angles, and backgrounds
  - Train YOLOv8 detection model for plate localization
  - Train LPRNet recognition model optimized for Chilean format (NNNNNN-XX or similar)
  - Validate against real-world Chilean plates (different provinces, years, conditions)
  - Export to ONNX format for production use
  
- [ ] Dataset expansion
  - Current models: `LPRNet_chilean_fixed.onnx`, `chilean_lpr_enhanced.onnx`
  - Need: More diverse samples from different regions of Chile
  - Consider: Night vision, rain, dirt, partial occlusion scenarios

- [ ] Model optimization
  - Quantization for edge devices
  - Performance tuning for Intel iGPU (current hardware)
  - TensorRT optimization for NVIDIA deployment

## Documentation
- [ ] Add installation screenshots
- [ ] Create video tutorial for initial setup
- [ ] Document RTSP camera compatibility list

## Testing
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
