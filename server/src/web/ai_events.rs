// This file is part of Moonshadow NVR, an intelligent surveillance system with AI capabilities.
// Fork of Moonshadow NVR. Copyright (C) 2025 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

use crate::json;
use crate::web::{serve_json, Caller, ResponseResult};
use base::err;
use core::borrow::Borrow;
use http::Request;
use std::str::FromStr;
use std::sync::Arc;
use url::form_urlencoded;

pub async fn ai_events(
    req: Request<hyper::body::Incoming>,
    _caller: Caller,
    db: Arc<db::Database>,
) -> ResponseResult {
    let mut type_filter: Option<String> = None;
    let mut camera_id_filter: Option<i32> = None;
    let mut start_time_90k: Option<i64> = None;
    let mut end_time_90k: Option<i64> = None;
    let mut limit: i64 = 100;

    if let Some(q) = req.uri().query() {
        for (key, value) in form_urlencoded::parse(q.as_bytes()) {
            let (key, value): (_, &str) = (key.borrow(), value.borrow());
            match key {
                "type" => type_filter = Some(value.to_string()),
                "cameraId" => {
                    camera_id_filter = i32::from_str(value)
                        .map_err(|_| err!(InvalidArgument, msg("unparseable cameraId")))?
                        .into()
                }
                "startTime90k" => {
                    start_time_90k = i64::from_str(value)
                        .map_err(|_| err!(InvalidArgument, msg("unparseable startTime90k")))?
                        .into()
                }
                "endTime90k" => {
                    end_time_90k = i64::from_str(value)
                        .map_err(|_| err!(InvalidArgument, msg("unparseable endTime90k")))?
                        .into()
                }
                "limit" => {
                    limit = i64::from_str(value)
                        .map_err(|_| err!(InvalidArgument, msg("unparseable limit")))?
                }
                _ => {}
            }
        }
    }

    let db_guard = db.lock();
    let rows = db_guard
        .query_ai_metadata(
            type_filter.as_deref(),
            camera_id_filter,
            start_time_90k,
            end_time_90k,
            limit,
        )
        .map_err(|e| err!(e, msg("failed to query AI metadata")))?;
    drop(db_guard);

    let events: Vec<json::AiEvent> = rows
        .into_iter()
        .map(|row| json::AiEvent {
            time_90k: row.time_90k,
            camera_id: row.camera_id,
            type_: row.type_,
            value: row.value,
        })
        .collect();

    serve_json(&req, &json::AiEventsResponse { events })
}
