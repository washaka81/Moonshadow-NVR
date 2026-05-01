// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2017-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Interactive CLI configuration interface.

pub mod cameras;
pub mod dirs_tui;
pub mod tui;
pub mod users_tui;

use base::clock;
use base::Error;
use bpaf::Bpaf;
use console::style;
use std::path::PathBuf;
use std::sync::Arc;

/// Interactively edits configuration.
#[derive(Bpaf, Debug)]
#[bpaf(command("config"))]
pub struct Args {
    #[bpaf(external(crate::parse_db_dir))]
    db_dir: PathBuf,
}

pub fn run(args: Args) -> Result<i32, Error> {
    let (_db_dir, mut conn) = super::open_conn(&args.db_dir, super::OpenMode::Create)?;
    let cur_ver = db::get_schema_version(&conn)?;
    if cur_ver.is_none() {
        println!("{}", style("🗄️  Initializing database...").cyan());
        conn.execute_batch(
            r#"
            pragma journal_mode = delete;
            pragma page_size = 16384;
            vacuum;
            pragma journal_mode = wal;
            "#,
        )?;
        db::init(&mut conn)?;
    }

    let db = Arc::new(db::Database::new(clock::RealClocks {}, conn, true)?);

    tui::run_main_menu(&db, &args.db_dir)?;

    Ok(0)
}
