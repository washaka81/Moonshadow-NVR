// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

//! Upgrades a version 8 schema to a version 9 schema.

use base::Error;

/// Upgrades a version 8 schema to a version 9 schema by adding the ai_event table.
pub fn run(_args: &super::Args, _tx: &rusqlite::Transaction) -> Result<(), Error> {
    _tx.execute_batch(
        r#"
        -- AI events with links to recordings.
        create table if not exists ai_event (
          camera_id integer not null references camera (id),
          timestamp_90k integer not null,
          event_type text not null,
          payload text not null,
          video_link text,
          primary key (camera_id, timestamp_90k, event_type)
        );
        "#,
    )?;
    Ok(())
}
