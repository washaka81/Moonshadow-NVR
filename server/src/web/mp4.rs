// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use crate::web::{ResponseResult};
use base::{err, Error};
use http::{header, Request, Response};
use http_body_util::Full;
use hyper::body::Bytes;
use std::sync::Arc;

pub async fn serve_mp4_download(
    req: Request<hyper::body::Incoming>,
    camera_id: i32,
    time_sec: i64,
    db: Arc<db::Database>,
) -> ResponseResult {
    // Buscamos la grabación que contiene este segundo
    let time_90k = time_sec * 90000;
    
    // Lógica simplificada: obtenemos el path del fragmento de vídeo crudo
    // En Moonshadow, esto requiere buscar en la tabla 'recording'
    let file_data = {
        let l = db.lock();
        // Buscamos el archivo físico... (simplificado para operatividad)
        vec![0u8; 1024] // Placeholder del contenido
    };

    let response = Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "video/mp4")
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"camera_{}_event_{}.mp4\"", camera_id, time_sec))
        .body(Full::new(Bytes::from(file_data)))
        .map_err(|e| err!(Unknown, msg("fail build resp"), source(e)))?;

    Ok(response)
}
