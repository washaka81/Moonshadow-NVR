// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2021 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Live video websocket handling.

use std::sync::Arc;

use base::{bail, err, Error};
use futures::SinkExt;
use http::header;
use tokio_tungstenite::tungstenite;
use tracing::debug;
use uuid::Uuid;

use crate::mp4;

use super::{websocket::WebSocketStream, Caller, Service};

impl Service {
    pub(super) async fn stream_live_m4s(
        self: Arc<Self>,
        ws: &mut WebSocketStream,
        caller: Result<Caller, Error>,
        uuid: Uuid,
        stream_type: db::StreamType,
    ) {
        if let Err(err) = self
            .stream_live_m4s_inner(ws, caller, uuid, stream_type)
            .await
        {
            tracing::error!(err = %err.chain(), "closing with error");
            let _ = ws
                .send(tungstenite::Message::Text(
                    serde_json::to_string(&crate::json::LiveM4sMessage::Error {
                        message: err.to_string(),
                    })
                    .expect("should serialize")
                    .into(),
                ))
                .await;
        } else {
            tracing::info!("closing");
        };
    }

    async fn stream_live_m4s_inner(
        &self,
        ws: &mut WebSocketStream,
        caller: Result<Caller, Error>,
        uuid: Uuid,
        stream_type: db::StreamType,
    ) -> Result<(), Error> {
        let caller = caller?;
        if !caller.permissions.view_video {
            bail!(PermissionDenied, msg("view_video required"));
        }

        let (open_id, stream_id, stream);
        {
            let db = self.db.lock();
            open_id = match db.open {
                None => {
                    bail!(
                        FailedPrecondition,
                        msg("database is read-only; there are no live streams"),
                    );
                }
                Some(o) => o.id,
            };
            let camera = db
                .get_camera(uuid)
                .ok_or_else(|| err!(NotFound, msg("no such camera {uuid}")))?;
            stream_id = camera.streams[stream_type.index()]
                .ok_or_else(|| err!(NotFound, msg("no such stream {uuid}/{stream_type}")))?;
            stream = db
                .streams_by_id()
                .get(&stream_id)
                .expect("stream referenced by camera should exist")
                .clone();
        };
        let mut sub_rx = stream.frames();

        // Timeout for initial frame - if no frames arrive within 30 seconds, close connection
        let initial_timeout = tokio::time::Duration::from_secs(30);
        let mut first_frame = true;

        loop {
            // Apply timeout for first frame only
            let next = if first_frame {
                match tokio::time::timeout(initial_timeout, sub_rx.next()).await {
                    Ok(result) => result,
                    Err(_) => {
                        bail!(
                            DeadlineExceeded,
                            msg("timeout waiting for initial frame on {}/{}", uuid, stream_type)
                        );
                    }
                }
            } else {
                sub_rx.next().await
            };

            match next {
                Ok((_n, frame)) => {
                    if first_frame {
                        first_frame = false;
                    }
                    if !self
                        .stream_live_m4s_chunk(open_id, stream_id, ws, frame)
                        .await?
                    {
                        return Ok(());
                    }
                }
                Err(db::stream::DroppedFramesError { last, next }) => {
                    let next_key_frame = sub_rx.reset().expect("should have key frame after drop");
                    let behind = next.get() - last.get();
                    let key_frame_gap = next_key_frame.get() - next.get();
                    debug!("subscriber fell {behind} frames behind `recent_frames`; will jump {key_frame_gap} frames further to next key frame");
                    if ws
                        .send(tungstenite::Message::Text(
                            serde_json::to_string(&crate::json::LiveM4sMessage::Dropped {
                                frames: next_key_frame.get() - last.get(),
                            })
                            .expect("should serialize")
                            .into(),
                        ))
                        .await
                        .is_err()
                    {
                        return Ok(());
                    }
                }
            }
        }
    }

    /// Sends a single live segment chunk of a `live.m4s` stream, returning `Ok(false)` when
    /// the connection is lost.
    async fn stream_live_m4s_chunk(
        &self,
        open_id: u32,
        stream_id: i32,
        ws: &mut WebSocketStream,
        live: db::RecentFrame,
    ) -> Result<bool, Error> {
        let mut builder = mp4::FileBuilder::new(mp4::Type::MediaSegment);

        /// Selected fields from the `ListRecordingsRow` that are used after dropping the lock.
        struct AbridgedRow {
            prev_media_duration_and_runs: Option<(base::time::Duration, i32)>,
            run_offset: i32,
            start: base::time::Time,
            video_sample_entry_id: i32,
        }
        let mut row = None;
        tracing::debug!("looking for row to match frame");
        {
            let db = self.db.lock();
            let mut rows = 0;
            db.list_recordings_by_id(
                stream_id,
                live.recording_id..live.recording_id + 1,
                &mut |r| {
                    rows += 1;
                    builder.append(&db, &r, live.media_off_90k.clone(), false)?;
                    row = Some(AbridgedRow {
                        prev_media_duration_and_runs: r.prev_media_duration_and_runs,
                        run_offset: r.run_offset,
                        start: r.start,
                        video_sample_entry_id: r.video_sample_entry_id,
                    });
                    Ok(())
                },
            )?;
        }
        let row = row.ok_or_else(|| err!(Internal, msg("unable to find recording for {live:?}")))?;
        tracing::debug!("have row to match frame");
        use http_serve::Entity;
        let mp4 = builder.build(self.db.clone())?;
        let mut hdrs = header::HeaderMap::new();
        mp4.add_headers(&mut hdrs);
        let mime_type = hdrs.get(header::CONTENT_TYPE).unwrap();
        let (prev_media_duration, prev_runs) = row.prev_media_duration_and_runs.unwrap();
        let hdr = format!(
            "Content-Type: {}\r\n\
             X-Recording-Start: {}\r\n\
             X-Recording-Id: {}.{}\r\n\
             X-Media-Time-Range: {}-{}\r\n\
             X-Prev-Media-Duration: {}\r\n\
             X-Runs: {}\r\n\
             X-Video-Sample-Entry-Id: {}\r\n\r\n",
            mime_type.to_str().unwrap(),
            row.start.0,
            open_id,
            live.recording_id,
            live.media_off_90k.start,
            live.media_off_90k.end,
            prev_media_duration.0,
            prev_runs + if row.run_offset == 0 { 1 } else { 0 },
            &row.video_sample_entry_id
        );
        let mut v = hdr.into_bytes();
        mp4.append_into_vec(&mut v).await?;
        tracing::debug!("sending frame msg");
        Ok(ws.send(tungstenite::Message::Binary(v.into())).await.is_ok())
    }
}
