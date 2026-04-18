// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Fork of Moonshadow NVR. Copyright (C) 2022 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use std::path::Path;

use ndarray::{Array4, ArrayViewD};
use ort::{inputs, session::Session, value::Tensor};

use base::clock::Clocks;
use base::Error;
use base::{bail, err};
use image::{DynamicImage, GenericImageView};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

use crate::cmds::run::AiMode;

use byteorder::{BigEndian, ReadBytesExt};
use std::io::Cursor;

#[derive(Debug, Clone)]
pub struct Detection {
    pub class_id: usize,
    pub confidence: f32,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

pub struct Detector {
    detection_model: std::sync::Mutex<Session>,
    reid_model: Option<std::sync::Mutex<Session>>,
    lpr_model: Option<std::sync::Mutex<Session>>,
    input_size: u32,
    ai_mode: AiMode,
    hardware_acceleration: bool,
    optimize_for_device: bool,
}

unsafe impl Send for Detector {}
unsafe impl Sync for Detector {}

impl Detector {
    pub fn new(
        detection_model_path: &Path,
        reid_model_path: Option<&Path>,
        lpr_model_path: Option<&Path>,
        ai_mode: AiMode,
        hardware_acceleration: bool,
        optimize_for_device: bool,
    ) -> Result<Self, Error> {
        info!("loading detection AI model from {:?}", detection_model_path);
        info!(
            "AI mode: {:?}, hardware acceleration: {}, optimize for device: {}",
            ai_mode, hardware_acceleration, optimize_for_device
        );

        let _ = ort::init().with_name("moonshadow").commit();

        let eps = [
            ort::ep::TensorRT::default().build(),
            ort::ep::CUDA::default().build(),
            ort::ep::OpenVINO::default().build(),
            ort::ep::CPU::default().build(),
        ];

        let detection_model = Session::builder()
            .map_err(|e| {
                err!(
                    Unknown,
                    msg("failed to build session"),
                    source(std::io::Error::other(e.to_string()))
                )
            })?
            .with_execution_providers(eps.clone())
            .map_err(|e| {
                err!(
                    Unknown,
                    msg("failed to set execution providers"),
                    source(std::io::Error::other(e.to_string()))
                )
            })?
            .commit_from_file(detection_model_path)
            .map_err(|e| {
                err!(
                    Unknown,
                    msg("failed to load detection model"),
                    source(std::io::Error::other(e.to_string()))
                )
            })?;

        let reid_model = if let Some(path) = reid_model_path {
            info!("loading ReID AI model from {:?}", path);
            Some(std::sync::Mutex::new(
                Session::builder()
                    .map_err(|e| {
                        err!(
                            Unknown,
                            msg("failed to build session"),
                            source(std::io::Error::other(e.to_string()))
                        )
                    })?
                    .with_execution_providers(eps.clone())
                    .map_err(|e| {
                        err!(
                            Unknown,
                            msg("failed to set execution providers"),
                            source(std::io::Error::other(e.to_string()))
                        )
                    })?
                    .commit_from_file(path)
                    .map_err(|e| {
                        err!(
                            Unknown,
                            msg("failed to load ReID model"),
                            source(std::io::Error::other(e.to_string()))
                        )
                    })?,
            ))
        } else {
            None
        };

        let lpr_model = if let Some(path) = lpr_model_path {
            info!("loading LPR AI model from {:?}", path);
            Some(std::sync::Mutex::new(
                Session::builder()
                    .map_err(|e| {
                        err!(
                            Unknown,
                            msg("failed to build session"),
                            source(std::io::Error::other(e.to_string()))
                        )
                    })?
                    .with_execution_providers(eps.clone())
                    .map_err(|e| {
                        err!(
                            Unknown,
                            msg("failed to set execution providers"),
                            source(std::io::Error::other(e.to_string()))
                        )
                    })?
                    .commit_from_file(path)
                    .map_err(|e| {
                        err!(
                            Unknown,
                            msg("failed to load LPR model"),
                            source(std::io::Error::other(e.to_string()))
                        )
                    })?,
            ))
        } else {
            None
        };

        Ok(Self {
            detection_model: std::sync::Mutex::new(detection_model),
            reid_model,
            lpr_model,
            input_size: 640,
            ai_mode,
            hardware_acceleration,
            optimize_for_device,
        })
    }

    pub fn sample_interval_90k(&self) -> i64 {
        match self.ai_mode {
            AiMode::Off => i64::MAX,
            AiMode::Low => 30 * 90000,
            AiMode::Medium => 8 * 90000,
            AiMode::High => 2 * 90000,
            AiMode::Auto => 8 * 90000,
        }
    }

    pub fn detect(&self, image: &DynamicImage) -> Result<Vec<Detection>, Error> {
        info!(
            "Detector: AI mode {:?}, hardware acceleration {}, optimize for device {}",
            self.ai_mode, self.hardware_acceleration, self.optimize_for_device
        );
        let (orig_w, orig_h) = image.dimensions();
        let resized = image.resize_exact(
            self.input_size,
            self.input_size,
            image::imageops::FilterType::Triangle,
        );

        let mut input = Array4::<f32>::zeros((1, 3, 640, 640));
        for (x, y, rgb) in resized.pixels() {
            input[[0, 0, y as usize, x as usize]] = rgb[0] as f32 / 255.0; // R
            input[[0, 1, y as usize, x as usize]] = rgb[1] as f32 / 255.0; // G
            input[[0, 2, y as usize, x as usize]] = rgb[2] as f32 / 255.0; // B
        }

        info!("Using ORT ONNX backend for detection (auto-accelerated)");

        let tensor = Tensor::from_array(input).map_err(|e| {
            err!(
                Unknown,
                msg("failed to create input tensor"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;

        let mut guard = self.detection_model.lock().unwrap();
        let result = guard.run(inputs![tensor]).map_err(|e| {
            err!(
                Unknown,
                msg("failed to run inference"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;

        let (shape, data) = result[0].try_extract_tensor::<f32>().map_err(|e| {
            err!(
                Unknown,
                msg("failed to get output array"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;

        let shape_usize: Vec<usize> = shape.iter().map(|&x| x as usize).collect();
        let output = ArrayViewD::from_shape(shape_usize, data).map_err(|e| {
            err!(
                Unknown,
                msg("invalid shape"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;

        let mut detections = Vec::new();
        let num_anchors = output.shape()[2];

        for i in 0..num_anchors {
            let mut max_score = 0.0;
            let mut class_id = 0;
            for c in 4..84 {
                let score = output[[0, c, i]];
                if score > max_score {
                    max_score = score;
                    class_id = c - 4;
                }
            }

            if max_score > 0.45 {
                let cx = output[[0, 0, i]];
                let cy = output[[0, 1, i]];
                let w = output[[0, 2, i]];
                let h = output[[0, 3, i]];

                detections.push(Detection {
                    class_id,
                    confidence: max_score,
                    x: (cx - w / 2.0) * orig_w as f32 / 640.0,
                    y: (cy - h / 2.0) * orig_h as f32 / 640.0,
                    w: w * orig_w as f32 / 640.0,
                    h: h * orig_h as f32 / 640.0,
                });
            }
        }

        detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        let mut final_detections = Vec::new();
        while !detections.is_empty() {
            let best = detections.remove(0);
            final_detections.push(best.clone());
            detections.retain(|d| {
                if d.class_id != best.class_id {
                    return true;
                }
                let iou = calculate_iou(&best, d);
                iou < 0.45
            });
        }

        Ok(final_detections)
    }

    pub fn reid(&self, person_crop: &DynamicImage) -> Result<Vec<f32>, Error> {
        let model = self
            .reid_model
            .as_ref()
            .ok_or_else(|| err!(Unknown, msg("ReID model not loaded")))?;
        let resized = person_crop.resize_exact(256, 128, image::imageops::FilterType::Triangle);
        let mut input = Array4::<f32>::zeros((1, 3, 128, 256));

        for (x, y, rgb) in resized.pixels() {
            input[[0, 0, y as usize, x as usize]] = rgb[0] as f32 / 255.0;
            input[[0, 1, y as usize, x as usize]] = rgb[1] as f32 / 255.0;
            input[[0, 2, y as usize, x as usize]] = rgb[2] as f32 / 255.0;
        }

        let tensor = Tensor::from_array(input).map_err(|e| {
            err!(
                Unknown,
                msg("failed to create input tensor"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;
        let mut guard = model.lock().unwrap();
        let result = guard.run(inputs![tensor]).map_err(|e| {
            err!(
                Unknown,
                msg("failed to run ReID inference"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;

        let (shape, data) = result[0].try_extract_tensor::<f32>().map_err(|e| {
            err!(
                Unknown,
                msg("failed to get ReID output array"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;

        let shape_usize: Vec<usize> = shape.iter().map(|&x| x as usize).collect();
        let output = ArrayViewD::from_shape(shape_usize, data).map_err(|e| {
            err!(
                Unknown,
                msg("invalid shape"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;
        let embedding: Vec<f32> = output.iter().cloned().collect();
        Ok(embedding)
    }

    pub fn read_plate(&self, vehicle_crop: &DynamicImage) -> Result<String, Error> {
        let model = self
            .lpr_model
            .as_ref()
            .ok_or_else(|| err!(Unknown, msg("LPR model not loaded")))?;
        let resized = vehicle_crop.resize_exact(94, 24, image::imageops::FilterType::Triangle);
        let mut input = Array4::<f32>::zeros((1, 3, 24, 94));

        for (x, y, rgb) in resized.pixels() {
            input[[0, 0, y as usize, x as usize]] = (rgb[2] as f32 - 127.5) * 0.0078125; // B
            input[[0, 1, y as usize, x as usize]] = (rgb[1] as f32 - 127.5) * 0.0078125; // G
            input[[0, 2, y as usize, x as usize]] = (rgb[0] as f32 - 127.5) * 0.0078125;
            // R
        }

        let tensor = Tensor::from_array(input).map_err(|e| {
            err!(
                Unknown,
                msg("failed to create input tensor"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;
        let mut guard = model.lock().unwrap();
        let result = guard.run(inputs![tensor]).map_err(|e| {
            err!(
                Unknown,
                msg("failed to run LPR inference"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;

        let (shape, data) = result[0].try_extract_tensor::<f32>().map_err(|e| {
            err!(
                Unknown,
                msg("failed to get LPR output array"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;

        let shape_usize: Vec<usize> = shape.iter().map(|&x| x as usize).collect();
        let output = ArrayViewD::from_shape(shape_usize, data).map_err(|e| {
            err!(
                Unknown,
                msg("invalid shape"),
                source(std::io::Error::other(e.to_string()))
            )
        })?;

        info!("LPR output shape: {:?}", output.shape());
        let shape = output.shape();
        let (seq_len, num_classes, transposed) = if shape.len() == 3 {
            if shape[1] == 68 && shape[2] == 18 {
                (shape[2], shape[1], true)
            } else if shape[1] <= shape[2] {
                (shape[1], shape[2], false)
            } else {
                (shape[2], shape[1], true)
            }
        } else if shape.len() == 2 {
            (shape[0], shape[1], false)
        } else {
            bail!(
                Unknown,
                msg("unexpected LPR output shape"),
                source(std::io::Error::other(format!("{:?}", shape)))
            );
        };

        // Skip batch dimension for flat indexing
        let total_elements: usize = shape.iter().skip(1).cloned().product();
        let flat_data: Vec<f32> = output.iter().cloned().collect();

        let plate = decode_lpr_output(&flat_data, seq_len, num_classes, transposed, total_elements);
        info!("Decoded plate: {}", plate);
        Ok(plate)
    }
}
fn calculate_iou(a: &Detection, b: &Detection) -> f32 {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = (a.x + a.w).min(b.x + b.w);
    let y2 = (a.y + a.h).min(b.y + b.h);

    let inter_area = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let a_area = a.w * a.h;
    let b_area = b.w * b.h;

    inter_area / (a_area + b_area - inter_area)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm_a * norm_b)
}

pub struct DetectionWorker<C: Clocks + Clone> {
    detector: Arc<Detector>,
    receiver: mpsc::Receiver<(Vec<u8>, i32, i64, Arc<db::Stream>)>,

    person_gallery: HashMap<String, Vec<f32>>,
    last_processed_time_90k: Option<i64>,
    sample_interval_90k: i64,
    _clocks: C,
}

impl<C: Clocks + Clone> DetectionWorker<C> {
    pub fn new(
        detector: Arc<Detector>,
        receiver: mpsc::Receiver<(Vec<u8>, i32, i64, Arc<db::Stream>)>,
        clocks: C,
    ) -> Self {
        let sample_interval_90k = detector.sample_interval_90k();
        Self {
            detector,
            receiver,

            person_gallery: HashMap::new(),
            last_processed_time_90k: None,
            sample_interval_90k,
            _clocks: clocks,
        }
    }

    pub async fn run(mut self, db: Arc<db::Database<C>>) {
        info!("Detection worker started");
        let mut frame_count = 0;
        let mut processed_count = 0;

        while let Some((data, _camera_id, time_90k, stream)) = self.receiver.recv().await {
            frame_count += 1;

            // Sampling logic based on AI mode
            if self.sample_interval_90k == i64::MAX {
                continue; // AI mode Off
            }
            if let Some(last) = self.last_processed_time_90k {
                if time_90k - last < self.sample_interval_90k {
                    continue; // skip frame
                }
            }
            self.last_processed_time_90k = Some(time_90k);
            processed_count += 1;

            let process_start = std::time::Instant::now();
            info!(
                "Processing frame {} (#{}) of size {} at time {}",
                frame_count,
                processed_count,
                data.len(),
                time_90k
            );

            match self.decode_h264_to_image(&data) {
                Ok(image) => {
                    let decode_time = process_start.elapsed();
                    info!(
                        "Frame {} decoded in {:.2}ms, size: {}x{}",
                        frame_count,
                        decode_time.as_secs_f64() * 1000.0,
                        image.width(),
                        image.height()
                    );

                    match self.detector.detect(&image) {
                        Ok(detections) => {
                            let detect_time = process_start.elapsed();
                            info!(
                                "Frame {}: {} detections in {:.2}ms",
                                frame_count,
                                detections.len(),
                                detect_time.as_secs_f64() * 1000.0
                            );
                            for (i, det) in detections.iter().enumerate() {
                                let class_name = match det.class_id {
                                    0 => "persona",
                                    1 => "vehículo",
                                    _ => "otro",
                                };
                                info!(
                                    "  Detección {}: {} (conf {:.2}) en [{:.0},{:.0},{:.0},{:.0}]",
                                    i, class_name, det.confidence, det.x, det.y, det.w, det.h
                                );
                            }

                            // Save annotated image every 20 frames
                            if processed_count % 20 == 1 {
                                self.draw_and_save_detections(
                                    &image,
                                    &detections,
                                    processed_count,
                                    &stream,
                                )
                                .await;
                            }

                            // Process detections for ReID and LPR
                            self.process_detections(&image, &detections, time_90k, &stream, &db)
                                .await;

                            let total_time = process_start.elapsed();
                            info!(
                                "Frame {} total processing time: {:.2}ms",
                                frame_count,
                                total_time.as_secs_f64() * 1000.0
                            );
                        }
                        Err(e) => {
                            info!("Error en detección: {}", e);
                        }
                    }
                }
                Err(e) => {
                    info!("Error decodificando frame {}: {}", frame_count, e);
                }
            }
        }
    }

    // fn match_person and fn update_db_signal commented out

    /// Converts H.264 data in AVCC (length-prefixed) format to Annex B (start code) format.
    fn convert_avcc_to_annexb(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        let mut reader = Cursor::new(data);
        let mut out = Vec::with_capacity(data.len() + 64); // extra space for start codes
        let start_code = [0x00, 0x00, 0x00, 0x01];

        while reader.position() < data.len() as u64 {
            let len = reader
                .read_u32::<BigEndian>()
                .map_err(|e| err!(Unknown, msg("failed to read length prefix"), source(e)))?;
            if len == 0 {
                // zero-length NAL unit? shouldn't happen, but skip
                continue;
            }
            let start = reader.position() as usize;
            let end = start + len as usize;
            if end > data.len() {
                bail!(
                    Unknown,
                    msg("invalid length prefix {} exceeds remaining data", len)
                );
            }
            out.extend_from_slice(&start_code);
            out.extend_from_slice(&data[start..end]);
            reader.set_position(end as u64);
        }
        Ok(out)
    }

    fn decode_h264_to_image(&self, data: &[u8]) -> Result<DynamicImage, Error> {
        use std::fs::File;
        use std::io::Write;
        use std::process::Command;

        let data_to_write = if data.starts_with(&[0x00, 0x00, 0x00, 0x01])
            || data.starts_with(&[0x00, 0x00, 0x01])
        {
            data.to_vec()
        } else {
            match self.convert_avcc_to_annexb(data) {
                Ok(annexb) => annexb,
                Err(_) => {
                    let mut new_data = Vec::with_capacity(data.len() + 4);
                    new_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
                    new_data.extend_from_slice(data);
                    new_data
                }
            }
        };

        let h264_path = format!("/tmp/frame_{}.h264", std::process::id());
        let png_path = format!("/tmp/frame_{}.png", std::process::id());

        let mut file = File::create(&h264_path)
            .map_err(|e| err!(Unknown, msg("failed to create temp file"), source(e)))?;
        file.write_all(&data_to_write)
            .map_err(|e| err!(Unknown, msg("failed to write H.264 data"), source(e)))?;

        let output = Command::new("ffmpeg")
            .args([
                "-i",
                &h264_path,
                "-frames:v",
                "1",
                "-y",
                "-loglevel",
                "error",
                &png_path,
            ])
            .output()
            .map_err(|e| err!(Unknown, msg("failed to execute ffmpeg"), source(e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(Unknown, msg("ffmpeg conversion failed: {}", stderr));
        }

        let img = image::open(&png_path)
            .map_err(|e| err!(Unknown, msg("failed to open decoded image"), source(e)))?;

        let _ = std::fs::remove_file(&h264_path);
        let _ = std::fs::remove_file(&png_path);

        Ok(img)
    }

    async fn draw_and_save_detections(
        &self,
        image: &DynamicImage,
        detections: &[Detection],
        frame_count: i32,
        _stream: &Arc<db::Stream>,
    ) {
        if detections.is_empty() {
            return;
        }

        // Crear una copia de la imagen para dibujar
        let mut img_copy = image.clone().to_rgb8();

        // Colores para diferentes clases
        let colors = [
            image::Rgb([255, 0, 0]), // Rojo: persona
            image::Rgb([0, 255, 0]), // Verde: vehículo
            image::Rgb([0, 0, 255]), // Azul: otros
        ];

        for det in detections {
            let color = if det.class_id == 0 {
                colors[0]
            } else if det.class_id == 1 {
                colors[1]
            } else {
                colors[2]
            };

            // Dibujar bounding box
            let x1 = det.x as i32;
            let y1 = det.y as i32;
            let x2 = (det.x + det.w) as i32;
            let y2 = (det.y + det.h) as i32;

            // Dibujar rectángulo (líneas simples)
            for x in x1..=x2 {
                if x >= 0 && x < img_copy.width() as i32 && y1 >= 0 && y1 < img_copy.height() as i32
                {
                    img_copy.put_pixel(x as u32, y1 as u32, color);
                }
                if x >= 0 && x < img_copy.width() as i32 && y2 >= 0 && y2 < img_copy.height() as i32
                {
                    img_copy.put_pixel(x as u32, y2 as u32, color);
                }
            }
            for y in y1..=y2 {
                if x1 >= 0 && x1 < img_copy.width() as i32 && y >= 0 && y < img_copy.height() as i32
                {
                    img_copy.put_pixel(x1 as u32, y as u32, color);
                }
                if x2 >= 0 && x2 < img_copy.width() as i32 && y >= 0 && y < img_copy.height() as i32
                {
                    img_copy.put_pixel(x2 as u32, y as u32, color);
                }
            }

            // Añadir texto con confianza (placeholder)
            // En una implementación real usaríamos imageproc o similar
        }

        // Guardar imagen cada 20 frames anotados, y siempre los primeros 5 frames
        if frame_count % 20 == 0 || frame_count <= 5 {
            let filename = format!("/tmp/detection_frame_laptop_{}.png", frame_count);
            if let Err(e) = img_copy.save(&filename) {
                info!("Error guardando imagen {}: {}", filename, e);
            } else {
                info!("Imagen guardada: {}", filename);
            }
        }
    }

    async fn process_detections(
        &mut self,
        image: &DynamicImage,
        detections: &[Detection],
        time_90k: i64,
        stream: &Arc<db::Stream>,
        db: &Arc<db::Database<C>>,
    ) {
        // Procesar cada detección para ReID o LPR
        for det in detections {
            if det.class_id == 0 {
                // Persona
                // Recortar región de persona
                let crop = image.crop_imm(det.x as u32, det.y as u32, det.w as u32, det.h as u32);

                // Extraer embedding ReID si modelo cargado
                if let Ok(embedding) = self.detector.reid(&crop) {
                    info!("Embedding ReID extraído ({} dimensiones)", embedding.len());

                    // Matching con galería de personas
                    let person_id = self.match_person(&embedding);

                    // Convertir embedding a bytes para almacenamiento
                    let embedding_bytes: &[u8] = unsafe {
                        std::slice::from_raw_parts(
                            embedding.as_ptr() as *const u8,
                            embedding.len() * std::mem::size_of::<f32>(),
                        )
                    };

                    // Obtener camera_id desde el stream
                    let camera_id = stream.inner.lock().camera_id;

                    // Insertar metadatos en base de datos
                    let guard = db.lock();
                    if let Err(e) = guard.insert_ai_metadata(
                        time_90k,
                        camera_id,
                        "person_reid",
                        &person_id,
                        Some(embedding_bytes),
                    ) {
                        info!("Error insertando metadatos de persona: {}", e);
                    } else {
                        info!("Metadatos de persona insertados: {}", person_id);
                    }
                }
            } else if det.class_id == 1 {
                // Vehículo
                // Recortar región de vehículo
                let crop = image.crop_imm(det.x as u32, det.y as u32, det.w as u32, det.h as u32);

                // Leer placa si modelo cargado
                if let Ok(plate) = self.detector.read_plate(&crop) {
                    info!("Placa detectada: {}", plate);

                    // Obtener camera_id desde el stream
                    let camera_id = stream.inner.lock().camera_id;

                    // Insertar metadatos en base de datos
                    let guard = db.lock();
                    if let Err(e) =
                        guard.insert_ai_metadata(time_90k, camera_id, "plate", &plate, None)
                    {
                        info!("Error insertando metadatos de placa: {}", e);
                    } else {
                        info!("Metadatos de placa insertados: {}", plate);
                    }
                }
            }
        }
    }

    fn match_person(&mut self, embedding: &[f32]) -> String {
        let mut best_similarity = 0.8; // umbral
        let mut best_id = None;
        for (id, stored_embedding) in &self.person_gallery {
            let similarity = cosine_similarity(embedding, stored_embedding);
            if similarity > best_similarity {
                best_similarity = similarity;
                best_id = Some(id.clone());
            }
        }
        match best_id {
            Some(id) => id,
            None => {
                let new_id = format!("person_{}", self.person_gallery.len() + 1);
                self.person_gallery
                    .insert(new_id.clone(), embedding.to_vec());
                new_id
            }
        }
    }

    #[allow(dead_code)]
    async fn save_frame_as_image(&self, data: &[u8], frame_count: i32) {
        use std::fs::File;
        use std::io::Write;
        use std::process::Command;

        info!("Guardando frame {} para demostración visual", frame_count);

        // Guardar raw H.264
        let h264_path = format!("/tmp/frame_raw_{}.h264", frame_count);
        let png_path = format!("/tmp/frame_decoded_{}.png", frame_count);

        // Convertir AVCC a Annex B si es necesario
        let converted_data = match self.convert_avcc_to_annexb(data) {
            Ok(annexb) => {
                info!("Frame convertido AVCC -> Annex B ({} bytes)", annexb.len());
                annexb
            }
            Err(e) => {
                info!("AVCC conversion failed, assuming Annex B: {}", e);
                data.to_vec()
            }
        };

        if let Ok(mut file) = File::create(&h264_path) {
            if file.write_all(&converted_data).is_ok() {
                info!("Frame H.264 guardado en {}", h264_path);

                // Convertir a PNG
                let output = Command::new("ffmpeg")
                    .args(["-i", &h264_path, "-frames:v", "1", "-y", &png_path])
                    .output();

                match output {
                    Ok(output) if output.status.success() => {
                        info!("Frame decodificado guardado en {}", png_path);
                        // Crear una versión anotada con bounding boxes de ejemplo
                        self.create_sample_annotation(&png_path, frame_count).await;
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        info!("Error convirtiendo frame: {}", stderr);
                    }
                    Err(e) => {
                        info!("Error ejecutando ffmpeg: {}", e);
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    async fn create_sample_annotation(&self, png_path: &str, frame_count: i32) {
        use std::process::Command;

        // Crear una versión anotada usando ImageMagick
        let annotated_path = format!("/tmp/frame_annotated_{}.png", frame_count);

        // Ejemplo: agregar bounding box y texto
        let status = Command::new("convert")
            .args([
                png_path,
                "-stroke",
                "red",
                "-strokewidth",
                "3",
                "-fill",
                "none",
                "-draw",
                "rectangle 100,100 300,300",
                "-stroke",
                "white",
                "-draw",
                "text 110,90 'Persona detectada (ejemplo)'",
                &annotated_path,
            ])
            .status();

        match status {
            Ok(status) if status.success() => {
                info!("Ejemplo visual creado en {}", annotated_path);
            }
            _ => {
                info!("No se pudo crear anotación de ejemplo");
            }
        }
    }
}

/// Decodes LPR model output using CTC decoding.
/// Extracted from `read_plate` for testability.
fn decode_lpr_output(
    output: &[f32],
    seq_len: usize,
    num_classes: usize,
    transposed: bool,
    total_elements: usize,
) -> String {
    let blank_class = 67;
    let mut plate_chars = Vec::new();

    for i in 0..seq_len {
        let mut max_prob = f32::NEG_INFINITY;
        let mut max_idx = 0;
        for c in 0..num_classes {
            let idx = if transposed {
                // shape: [batch, num_classes, seq_len] -> [0, c, i]
                c * seq_len + i
            } else {
                // shape: [batch, seq_len, num_classes] -> [0, i, c]
                i * num_classes + c
            };
            let prob = if idx < total_elements {
                output[idx]
            } else {
                f32::NEG_INFINITY
            };
            if prob > max_prob {
                max_prob = prob;
                max_idx = c;
            }
        }
        if max_idx != blank_class && max_idx < 31 {
            continue;
        }
        if max_idx != blank_class {
            let ch = match max_idx {
                31..=40 => (b'0' + (max_idx - 31) as u8) as char,
                41..=66 => (b'A' + (max_idx - 41) as u8) as char,
                _ => continue,
            };
            plate_chars.push(ch);
        }
    }

    let mut plate = String::new();
    let mut prev_char = None;
    for &ch in &plate_chars {
        if Some(ch) != prev_char {
            plate.push(ch);
            prev_char = Some(ch);
        }
    }
    if plate.is_empty() {
        plate = "UNKNOWN".to_string();
    }
    plate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iou() {
        let a = Detection {
            class_id: 0,
            confidence: 0.9,
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        };
        let b = Detection {
            class_id: 0,
            confidence: 0.8,
            x: 5.0,
            y: 5.0,
            w: 10.0,
            h: 10.0,
        };
        let iou = calculate_iou(&a, &b);
        assert!(iou > 0.14 && iou < 0.15);
    }

    #[test]
    fn test_lpr_decode_simple_plate() {
        // Test decoding a simple plate "ABC123"
        // seq_len = 18, num_classes = 68, transposed = true
        // Shape: [1, 68, 18] -> flat: 68 * 18 = 1224 elements
        let seq_len = 18;
        let num_classes = 68;
        let total = seq_len * num_classes;
        let mut output = vec![f32::NEG_INFINITY; total];

        // Set high probabilities for characters at specific timesteps
        // Class 41 = 'A', 42 = 'B', 43 = 'C', 31 = '0', 32 = '1', 33 = '2'
        let chars = [(0, 41), (1, 42), (2, 43), (3, 31), (4, 32), (5, 33)]; // ABC012
        for (timestep, class_idx) in chars.iter() {
            // transposed = true: index = c * seq_len + i
            let idx = *class_idx * seq_len + *timestep;
            output[idx] = 10.0; // high probability
        }

        let result = decode_lpr_output(&output, seq_len, num_classes, true, total);
        assert_eq!(result, "ABC012");
    }

    #[test]
    fn test_lpr_decode_with_blanks() {
        // Test that blank class (67) is skipped
        let seq_len = 10;
        let num_classes = 68;
        let total = seq_len * num_classes;
        let mut output = vec![f32::NEG_INFINITY; total];

        // Set blanks at even positions, 'A' (41) at odd positions
        for i in 0..seq_len {
            let idx = 67 * seq_len + i; // blank class
            output[idx] = if i % 2 == 0 { 10.0 } else { f32::NEG_INFINITY };
            if i % 2 == 1 {
                let idx = 41 * seq_len + i; // 'A'
                output[idx] = 10.0;
            }
        }

        let result = decode_lpr_output(&output, seq_len, num_classes, true, total);
        // Should decode 'A' at positions 1, 3, 5, 7, 9 -> collapsed to single 'A'
        assert_eq!(result, "A");
    }

    #[test]
    fn test_lpr_decode_duplicate_removal() {
        // Test CTC duplicate character removal
        let seq_len = 6;
        let num_classes = 68;
        let total = seq_len * num_classes;
        let mut output = vec![f32::NEG_INFINITY; total];

        // Set 'A' (41) at all timesteps
        for i in 0..seq_len {
            let idx = 41 * seq_len + i;
            output[idx] = 10.0;
        }

        let result = decode_lpr_output(&output, seq_len, num_classes, true, total);
        // All 'A's collapsed to single 'A'
        assert_eq!(result, "A");
    }

    #[test]
    fn test_lpr_decode_unknown_when_empty() {
        // When all outputs are blank or low-confidence, should return "UNKNOWN"
        let seq_len = 5;
        let num_classes = 68;
        let total = seq_len * num_classes;
        let output = vec![0.0f32; total]; // all zeros, blank class will be max

        let result = decode_lpr_output(&output, seq_len, num_classes, true, total);
        assert_eq!(result, "UNKNOWN");
    }

    #[test]
    fn test_lpr_decode_full_alphanumeric() {
        // Test decoding a plate with all character types: "AB9HYZ"
        let seq_len = 18;
        let num_classes = 68;
        let total = seq_len * num_classes;
        let mut output = vec![f32::NEG_INFINITY; total];

        // Class mapping: 31-40 = '0'-'9', 41-66 = 'A'-'Z'
        // A=41, B=42, 9=40, H=41+7=48, Y=41+24=65, Z=41+25=66
        let chars = [
            (0, 41), // 'A'
            (1, 42), // 'B'
            (2, 40), // '9'
            (3, 48), // 'H'
            (4, 65), // 'Y'
            (5, 66), // 'Z'
        ];

        for (timestep, class_idx) in chars.iter() {
            let idx = *class_idx * seq_len + *timestep;
            output[idx] = 10.0;
        }

        let result = decode_lpr_output(&output, seq_len, num_classes, true, total);
        assert_eq!(result, "AB9HYZ");
    }

    #[test]
    fn test_lpr_decode_non_transposed() {
        // Test non-transposed output format
        let seq_len = 8;
        let num_classes = 68;
        let total = seq_len * num_classes;
        let mut output = vec![f32::NEG_INFINITY; total];

        // Non-transposed: index = i * num_classes + c
        let chars = [(0, 41), (1, 42), (2, 43)]; // ABC
        for (timestep, class_idx) in chars.iter() {
            let idx = *timestep * num_classes + *class_idx;
            output[idx] = 10.0;
        }

        let result = decode_lpr_output(&output, seq_len, num_classes, false, total);
        assert_eq!(result, "ABC");
    }

    #[test]
    fn test_lpr_decode_ignores_low_confidence() {
        // Characters below threshold (< 31) should be ignored
        let seq_len = 5;
        let num_classes = 68;
        let total = seq_len * num_classes;
        let mut output = vec![f32::NEG_INFINITY; total];

        // Set class 10 (below 31) at position 0
        output[10 * seq_len + 0] = 10.0;
        // Set 'A' (41) at position 1
        output[41 * seq_len + 1] = 10.0;

        let result = decode_lpr_output(&output, seq_len, num_classes, true, total);
        assert_eq!(result, "A");
    }
}
