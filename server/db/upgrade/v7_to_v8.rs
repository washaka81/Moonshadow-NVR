// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2020 The Moonshadow NVR Authors; see AUTHORS and LICENSE.txt.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception

//! Upgrades a version 7 schema to a version 8 schema.

use base::Error;

/// Upgrades a version 7 schema to a version 8 schema by adding the ai_metadata table.
pub fn run(_args: &super::Args, _tx: &rusqlite::Transaction) -> Result<(), Error> {
    _tx.execute_batch(
        r#"
        -- AI metadata for advanced detections (ReID and LPR).
        create table ai_metadata (
          time_90k integer not null,
          camera_id integer not null references camera (id),
          type text not null, -- 'plate' or 'person_reid'
          value text not null, -- License plate text or person ID
          embedding blob, -- Vector for ReID
          primary key (time_90k, camera_id, type)
        );
        "#,
    )?;
    Ok(())
}
