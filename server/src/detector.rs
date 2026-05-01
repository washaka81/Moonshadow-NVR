// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use base::clock::Clocks;
use base::{err, Error};
use image::{DynamicImage, GenericImageView, Pixel};
use ndarray::Array4;
use ort::session::Session;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::vulkan_engine::VulkanEngine;

#[derive(Debug, Clone)]
pub struct Detection {
    pub class_id: u32,
    pub confidence: f32,
    #[allow(dead_code)]
    pub x: f32,
    #[allow(dead_code)]
    pub y: f32,
    #[allow(dead_code)]
    pub w: f32,
    #[allow(dead_code)]
    pub h: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiMode {
    Off,
    Low,
    Medium,
    High,
    Auto,
}

pub struct Detector {
    detection_model: Session,
    #[allow(dead_code)]
    reid_model: Option<Session>,
    #[allow(dead_code)]
    lpr_model: Option<Session>,
    vulkan_engine: Option<VulkanEngine>,
}

impl Detector {
    pub fn new(
        model_path: &Path,
        reid_model_path: Option<&Path>,
        lpr_model_path: Option<&Path>,
        _ai_mode: AiMode,
        hardware_acceleration: bool,
        vulkan_preprocessing: bool,
        openvino_repair: bool,
        _optimize: bool,
    ) -> Result<Self, Error> {
        info!("--- AI LOG: Initializing Universal Engine (H.264/H.265) ---");
        let _ = ort::init().with_name("moonshadow").commit();

        if openvino_repair {
            info!("--- AI LOG: OpenVINO Bridge Repair requested ---");
            repair_openvino_bridge();
        }

        let vulkan_engine = if vulkan_preprocessing {
            VulkanEngine::new()
        } else {
            None
        };

        let mut builder = Session::builder()
            .map_err(|e| err!(Unknown, msg("fail EP builder"), source(e.to_string())))?;
        if hardware_acceleration {
            info!("--- AI LOG: Engaging Hardware Acceleration ---");

            // Log Vulkan status for informational purposes as requested by user
            let _ = crate::vulkan_check::verify_vulkan_gpu();

            // Attempt to load OpenVINO, but fallback gracefully if the shared library is missing
            let mut ep_registered = false;

            // Try OpenVINO GPU
            let ep_gpu = ort::ep::OpenVINO::default()
                .with_device_type("GPU")
                .with_dynamic_shapes(true)
                .build();
            if let Ok(new_builder) = builder.clone().with_execution_providers([ep_gpu]) {
                builder = new_builder;
                ep_registered = true;
                info!("--- AI LOG: OpenVINO (GPU) Execution Provider registered successfully ---");
            }

            // If GPU failed, try OpenVINO CPU
            if !ep_registered {
                let ep_cpu = ort::ep::OpenVINO::default()
                    .with_device_type("CPU")
                    .with_dynamic_shapes(true)
                    .build();
                if let Ok(new_builder) = builder.clone().with_execution_providers([ep_cpu]) {
                    builder = new_builder;
                    ep_registered = true;
                    info!(
                        "--- AI LOG: OpenVINO (CPU) Execution Provider registered successfully ---"
                    );
                }
            }

            if !ep_registered {
                warn!("--- AI LOG: OpenVINO Provider missing or failed to load. Falling back to CPU ---");
                // Always ensure CPU fallback is available
                let ep_fallback = ort::ep::CPU::default().build();
                builder = builder
                    .with_execution_providers([ep_fallback])
                    .map_err(|e| {
                        err!(Unknown, msg("fail CPU EP fallback"), source(e.to_string()))
                    })?;
            }
        }
        let detection_model = builder
            .commit_from_file(model_path)
            .map_err(|e| err!(Unknown, msg("fail load model"), source(e.to_string())))?;
        info!("--- AI LOG: YOLOv8 model loaded ---");

        let reid_model = if let Some(path) = reid_model_path {
            Some(
                Session::builder()
                    .map_err(|e| err!(Unknown, msg("fail reid builder"), source(e.to_string())))?
                    .commit_from_file(path)
                    .map_err(|e| err!(Unknown, msg("fail reid model"), source(e.to_string())))?,
            )
        } else {
            None
        };

        let lpr_model = if let Some(path) = lpr_model_path {
            Some(
                Session::builder()
                    .map_err(|e| err!(Unknown, msg("fail lpr builder"), source(e.to_string())))?
                    .commit_from_file(path)
                    .map_err(|e| err!(Unknown, msg("fail lpr model"), source(e.to_string())))?,
            )
        } else {
            None
        };

        Ok(Self {
            detection_model,
            reid_model,
            lpr_model,
            vulkan_engine,
        })
    }

    pub fn detect(&mut self, image: &DynamicImage) -> Result<Vec<Detection>, Error> {
        let input = if let Some(engine) = &self.vulkan_engine {
            let rgba = image.to_rgba8();
            let data = engine
                .preprocess(rgba.as_raw(), image.width(), image.height(), 640, 640)
                .ok_or_else(|| err!(Unknown, msg("vulkan pre-processing failed")))?;
            Array4::from_shape_vec((1, 3, 640, 640), data)
                .map_err(|e| err!(Unknown, msg("fail reshape"), source(e.to_string())))?
        } else {
            let resized = image.resize_exact(640, 640, image::imageops::FilterType::Triangle);
            let mut input = Array4::<f32>::zeros((1, 3, 640, 640));
            for (x, y, pixel) in resized.pixels() {
                let rgb = pixel.to_rgb();
                input[[0, 0, y as usize, x as usize]] = (rgb[0] as f32) / 255.0;
                input[[0, 1, y as usize, x as usize]] = (rgb[1] as f32) / 255.0;
                input[[0, 2, y as usize, x as usize]] = (rgb[2] as f32) / 255.0;
            }
            input
        };

        let input_tensor = ort::value::Value::from_array(input)
            .map_err(|e| err!(Unknown, msg("fail create tensor"), source(e.to_string())))?;
        let outputs = self
            .detection_model
            .run(ort::inputs![input_tensor])
            .map_err(|e| err!(Unknown, msg("fail infer"), source(e.to_string())))?;
        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| err!(Unknown, msg("fail extract"), source(e.to_string())))?;

        let mut candidates = Vec::new();
        // Shape is [1, 84, 8400]. 84 = [x, y, w, h, class0, ..., class79]
        let view = ndarray::ArrayView3::from_shape(
            (shape[0] as usize, shape[1] as usize, shape[2] as usize),
            data,
        )
        .unwrap();
        let box_data = view.index_axis(ndarray::Axis(0), 0); // [84, 8400]

        for i in 0..8400 {
            let mut max_conf = 0.0;
            let mut max_id = 0;

            // Only check classes of interest: person=0, car=2, moto=3, bus=5, truck=7
            for &class_idx in &[0, 2, 3, 5, 7] {
                let conf = box_data[[class_idx + 4, i]];
                if conf > max_conf {
                    max_conf = conf;
                    max_id = class_idx;
                }
            }

            if max_conf > 0.45 {
                candidates.push(Detection {
                    class_id: max_id as u32,
                    confidence: max_conf,
                    x: box_data[[0, i]],
                    y: box_data[[1, i]],
                    w: box_data[[2, i]],
                    h: box_data[[3, i]],
                });
            }
        }

        // Explicitly drop outputs to release the borrow of self
        drop(outputs);

        // Simple NMS to reduce redundant detections
        candidates.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        let mut detections = Vec::new();
        for cand in candidates {
            let mut keep = true;
            for det in &detections {
                if iou(&cand, det) > 0.45 {
                    keep = false;
                    break;
                }
            }
            if keep {
                detections.push(cand);
            }
        }

        if !detections.is_empty() {
            info!(
                "--- AI DEBUG: YOLOv8 Detections: {:?} ---",
                detections.iter().map(|d| d.class_id).collect::<Vec<_>>()
            );
        }

        Ok(detections)
    }
}

fn iou(a: &Detection, b: &Detection) -> f32 {
    let x1 = (a.x - a.w / 2.0).max(b.x - b.w / 2.0);
    let y1 = (a.y - a.h / 2.0).max(b.y - b.h / 2.0);
    let x2 = (a.x + a.w / 2.0).min(b.x + b.w / 2.0);
    let y2 = (a.y + a.h / 2.0).min(b.y + b.h / 2.0);
    let intersection = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let union = a.w * a.h + b.w * b.h - intersection;
    intersection / union
}

fn repair_openvino_bridge() {
    info!("--- AI REPAIR: Scanning for OpenVINO ONNX Bridge ---");

    let target_lib = "libonnxruntime_providers_openvino.so";
    let search_paths = ["/usr/lib", "/usr/local/lib", "/opt/intel/openvino/lib"];

    let mut found_path = None;
    for path in search_paths {
        let p = Path::new(path).join(target_lib);
        if p.exists() {
            found_path = Some(p);
            break;
        }
    }

    if let Some(src) = found_path {
        info!("--- AI REPAIR: Found library at {:?} ---", src);
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let dst = exe_dir.join(target_lib);
                if !dst.exists() {
                    info!("--- AI REPAIR: Linking library to {:?} ---", dst);
                    let _ = std::os::unix::fs::symlink(src, dst);
                } else {
                    info!("--- AI REPAIR: Library already exists in target directory ---");
                }
            }
        }
    } else {
        warn!("--- AI REPAIR: Could not find libonnxruntime_providers_openvino.so in standard paths ---");
        warn!("--- AI REPAIR: Please install onnxruntime-openvino or provide the library manually ---");
    }
}

impl Detector {
    pub fn read_plate(&mut self, crop: &DynamicImage) -> Result<String, Error> {
        let model = match &mut self.lpr_model {
            Some(m) => m,
            None => return Ok("LPR_DISABLED".to_string()),
        };

        let input = if let Some(engine) = &self.vulkan_engine {
            let rgba = crop.to_rgba8();
            let data = engine
                .preprocess(rgba.as_raw(), crop.width(), crop.height(), 94, 24)
                .ok_or_else(|| err!(Unknown, msg("vulkan pre-processing failed (LPR)")))?;
            Array4::from_shape_vec((1, 3, 24, 94), data)
                .map_err(|e| err!(Unknown, msg("fail reshape lpr"), source(e.to_string())))?
        } else {
            // Preprocessing: Resize to 94x24, Normalize to 0-1, NCHW
            let resized = crop.resize_exact(94, 24, image::imageops::FilterType::Triangle);
            let mut input = Array4::<f32>::zeros((1, 3, 24, 94));
            for (x, y, pixel) in resized.pixels() {
                let rgb = pixel.to_rgb();
                input[[0, 0, y as usize, x as usize]] = (rgb[0] as f32) / 255.0;
                input[[0, 1, y as usize, x as usize]] = (rgb[1] as f32) / 255.0;
                input[[0, 2, y as usize, x as usize]] = (rgb[2] as f32) / 255.0;
            }
            input
        };

        let input_tensor = ort::value::Value::from_array(input).map_err(|e| {
            err!(
                Unknown,
                msg("fail create lpr tensor"),
                source(e.to_string())
            )
        })?;
        let outputs = model
            .run(ort::inputs![input_tensor])
            .map_err(|e| err!(Unknown, msg("fail lpr infer"), source(e.to_string())))?;
        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|e| err!(Unknown, msg("fail lpr extract"), source(e.to_string())))?;

        // LPRNet Decoder: shape is usually [1, 68, 18] (batch, classes, sequence)
        // Classes: 0-30 (Chinese - skip), 31-40 (0-9), 41-66 (A-Z), 67 (blank)
        let num_classes = shape[1] as usize;
        let seq_len = shape[2] as usize;
        let view = ndarray::ArrayView3::from_shape(
            (shape[0] as usize, shape[1] as usize, shape[2] as usize),
            data,
        )
        .unwrap();

        let mut plate_chars = Vec::new();
        let blank_idx = 67;

        for i in 0..seq_len {
            let mut max_prob = -f32::INFINITY;
            let mut max_idx = 0;
            for c in 0..num_classes {
                let prob = view[[0, c, i]];
                if prob > max_prob {
                    max_prob = prob;
                    max_idx = c;
                }
            }

            if max_idx != blank_idx && max_idx >= 31 && max_idx <= 66 {
                let ch = if max_idx <= 40 {
                    (b'0' + (max_idx - 31) as u8) as char
                } else {
                    (b'A' + (max_idx - 41) as u8) as char
                };
                plate_chars.push(ch);
            }
        }

        // Collapse consecutive duplicates (CTC decoding)
        let mut raw_plate = String::new();
        let mut prev_char = None;
        for ch in plate_chars {
            if Some(ch) != prev_char {
                raw_plate.push(ch);
                prev_char = Some(ch);
            }
        }

        if raw_plate.is_empty() {
            Ok("UNKNOWN".to_string())
        } else {
            let formatted = format_chilean_plate(&raw_plate);
            info!(
                "--- AI DEBUG: Decoded Plate: {} (raw: {}) ---",
                formatted, raw_plate
            );
            Ok(formatted)
        }
    }
    #[allow(dead_code)]
    pub fn reid(&self, _crop: &DynamicImage) -> Result<Vec<f32>, Error> {
        Ok(vec![0.0; 128])
    }
}

fn format_chilean_plate(raw: &str) -> String {
    let chars: Vec<char> = raw.chars().collect();
    if chars.len() != 6 {
        return raw.to_string();
    }

    let is_letter = |c: char| c.is_ascii_alphabetic();
    let is_digit = |c: char| c.is_ascii_digit();

    // Format: ABCD12 -> ABCD-12 (4 letters, 2 digits)
    if chars[0..4].iter().all(|&c| is_letter(c)) && chars[4..6].iter().all(|&c| is_digit(c)) {
        return format!("{}-{}", &raw[0..4], &raw[4..6]);
    }

    // Format: AB1234 -> AB-1234 (2 letters, 4 digits)
    if chars[0..2].iter().all(|&c| is_letter(c)) && chars[2..6].iter().all(|&c| is_digit(c)) {
        return format!("{}-{}", &raw[0..2], &raw[2..6]);
    }

    // Format: ABC123 -> ABC-123 (3 letters, 3 digits)
    if chars[0..3].iter().all(|&c| is_letter(c)) && chars[3..6].iter().all(|&c| is_digit(c)) {
        return format!("{}-{}", &raw[0..3], &raw[3..6]);
    }

    raw.to_string()
}

pub struct DetectionWorker<C: Clocks + Clone> {
    detector: Arc<tokio::sync::Mutex<Detector>>,
    receiver: mpsc::Receiver<(Vec<u8>, i32, i64, Arc<db::Stream>)>,
    prev_image: Option<DynamicImage>,
    _phantom: std::marker::PhantomData<C>,
}

impl<C: Clocks + Clone> DetectionWorker<C> {
    pub fn new(
        detector: Arc<tokio::sync::Mutex<Detector>>,
        receiver: mpsc::Receiver<(Vec<u8>, i32, i64, Arc<db::Stream>)>,
        _clocks: C,
    ) -> Self {
        Self {
            detector,
            receiver,
            prev_image: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn run(mut self, db: Arc<db::Database<C>>) {
        info!("--- AI LOG: Worker Service Online ---");
        while let Some((data, camera_id, time_90k, stream)) = self.receiver.recv().await {
            if let Ok(image) = self.decode_any_codec_to_image(&data, camera_id) {
                if self.has_motion(&image) {
                    let detections = {
                        let mut det_lock = self.detector.lock().await;
                        det_lock.detect(&image).unwrap_or_default()
                    };
                    if !detections.is_empty() {
                        self.process_detections(&image, &detections, time_90k, &stream, &db)
                            .await;
                    }
                }
                self.prev_image = Some(image);
            }
        }
    }

    fn has_motion(&self, _current: &DynamicImage) -> bool {
        true
    }

    async fn process_detections(
        &mut self,
        image: &DynamicImage,
        detections: &[Detection],
        time_90k: i64,
        stream: &Arc<db::Stream>,
        db: &Arc<db::Database<C>>,
    ) {
        let (camera_id, camera_uuid) = {
            let l = db.lock();
            let cam_id = stream.inner.lock().camera_id;
            let uuid = l
                .cameras_by_id()
                .get(&cam_id)
                .map(|c| c.uuid.to_string())
                .unwrap_or_default();
            (cam_id, uuid)
        };
        for det in detections {
            let start_t = time_90k; // Start exactly at detection time (removed -5s offset)
            let end_t = time_90k + 900000; // 10 seconds of video
            let video_link = format!(
                "/api/cameras/{}/main/view.mp4?startTime90k={}&endTime90k={}",
                camera_uuid, start_t, end_t
            );
            let type_str = match det.class_id {
                0 => "person",
                2 => "car",
                3 => "motorcycle",
                5 => "bus",
                7 => "truck",
                _ => "vehicle",
            };

            let mut final_type = type_str.to_string();
            let mut final_payload = format!(
                "{{\"type\": \"{}\", \"conf\": {:.2}}}",
                type_str, det.confidence
            );

            if matches!(det.class_id, 2 | 3 | 5 | 7) {
                let x = (det.x - det.w / 2.0).max(0.0) as u32;
                let y = (det.y - det.h / 2.0).max(0.0) as u32;
                let w = det.w.min(image.width() as f32 - x as f32) as u32;
                let h = det.h.min(image.height() as f32 - y as f32) as u32;
                if w > 0 && h > 0 {
                    let crop = image.crop_imm(x, y, w, h);
                    let mut det_lock = self.detector.lock().await;
                    if let Ok(plate) = det_lock.read_plate(&crop) {
                        if plate != "UNKNOWN" {
                            final_type = "license_plate".to_string();
                            final_payload = format!(
                                "{{\"type\": \"{}\", \"plate\": \"{}\", \"conf\": {:.2}}}",
                                type_str, plate, det.confidence
                            );
                        }
                    }
                }
            }

            let _ = db.lock().insert_ai_event(
                camera_id,
                time_90k,
                &final_type,
                &final_payload,
                &video_link,
            );
        }
    }

    fn decode_any_codec_to_image(
        &self,
        data: &[u8],
        camera_id: i32,
    ) -> Result<DynamicImage, Error> {
        let raw_path = format!("/tmp/nvr_{}.h264", camera_id);
        let png_path = format!("/tmp/nvr_{}.png", camera_id);

        // Convert length-prefixed (AVCC/MP4) to Annex-B (Start Codes)
        let mut bitstream = Vec::with_capacity(data.len() + 32);
        let mut i = 0;
        while i + 4 <= data.len() {
            let len = u32::from_be_bytes(data[i..i + 4].try_into().unwrap()) as usize;
            if i + 4 + len > data.len() {
                break;
            }
            bitstream.extend_from_slice(&[0, 0, 0, 1]);
            bitstream.extend_from_slice(&data[i + 4..i + 4 + len]);
            i += 4 + len;
        }

        let _ = std::fs::write(&raw_path, &bitstream);

        let output = std::process::Command::new("ffmpeg")
            .args(["-i", &raw_path, "-frames:v", "1", "-y", &png_path])
            .output();

        if let Ok(out) = output {
            if !out.status.success() {
                warn!(
                    "--- AI DEBUG: ffmpeg failed: {} ---",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        }

        let img =
            image::open(&png_path).map_err(|e| err!(Unknown, msg("decode error"), source(e)))?;
        let _ = std::fs::remove_file(&raw_path);
        let _ = std::fs::remove_file(&png_path);
        Ok(img)
    }
}
