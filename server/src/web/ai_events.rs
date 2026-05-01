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

    let mut sql =
        "SELECT camera_id, timestamp_90k, event_type, payload, video_link FROM ai_event WHERE 1=1"
            .to_string();
    if type_filter.is_some() {
        sql.push_str(" AND event_type = ?");
    }
    if camera_id_filter.is_some() {
        sql.push_str(" AND camera_id = ?");
    }
    if start_time_90k.is_some() {
        sql.push_str(" AND timestamp_90k >= ?");
    }
    if end_time_90k.is_some() {
        sql.push_str(" AND timestamp_90k < ?");
    }
    sql.push_str(" ORDER BY timestamp_90k DESC LIMIT ?");

    let events = {
        let l = db.lock();
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        if let Some(t) = &type_filter {
            params.push(t);
        }
        if let Some(c) = &camera_id_filter {
            params.push(c);
        }
        if let Some(st) = &start_time_90k {
            params.push(st);
        }
        if let Some(et) = &end_time_90k {
            params.push(et);
        }
        params.push(&limit);

        l.execute_raw_query(&sql, &params, |row| {
            Ok(json::AiEvent {
                camera_id: row.get(0)?,
                time_90k: row.get(1)?,
                type_: row.get(2)?,
                value: row.get(3)?,
                video_link: Some(row.get(4)?),
            })
        })?
    };

    serve_json(&req, &json::AiEventsResponse { events })
}
