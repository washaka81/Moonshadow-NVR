// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2017-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Interactive CLI configuration interface.

pub mod cameras;

use base::clock;
use base::err;
use base::Error;
use bpaf::Bpaf;
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
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

    println!(
        "{}",
        style("🎬 Moonshadow NVR Configuration CLI").cyan().bold()
    );
    println!(
        "{}",
        style("Interactive object-based camera configuration").dim()
    );
    println!();

    run_interactive_cli(db)?;

    Ok(0)
}

fn run_interactive_cli(db: Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    loop {
        println!();
        println!("{}", style("📋 Main Menu").bold());

        let options = vec![
            "📷 Manage Cameras (new UI)",
            "📷 Manage Cameras (classic)",
            "📁 Manage Directories",
            "👥 Manage Users",
            "📊 Show Statistics",
            "🚪 Exit",
        ];

        let selection = Select::with_theme(&theme)
            .with_prompt("Choose an option")
            .items(&options)
            .default(0)
            .interact()
            .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

        match selection {
            0 => cameras::run_camera_ui(&db)?,
            1 => cameras::run_classic(&db)?,
            2 => manage_directories(&db)?,
            3 => manage_users(&db)?,
            4 => show_statistics(&db)?,
            5 => break,
            _ => unreachable!(),
        }
    }

    println!("{}", style("👋 Goodbye!").green());
    Ok(())
}

fn show_statistics(db: &Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}", style("📊 System Statistics").bold());

    let l = db.lock();
    let cam_count = l.cameras_by_id().len();

    let mut stream_count = 0;
    for cam in l.cameras_by_id().values() {
        stream_count += cam.streams.iter().filter(|s| s.is_some()).count();
    }

    let dir_count = l.sample_file_dirs_by_id().len();
    let user_count = l.users_by_id().len();

    drop(l);

    println!("📷 Cameras: {}", style(cam_count).bold());
    println!("📡 Streams: {}", style(stream_count).bold());
    println!("📁 Directories: {}", style(dir_count).bold());
    println!("👥 Users: {}", style(user_count).bold());

    let _: String = Input::with_theme(&theme)
        .with_prompt("Press Enter to continue")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    Ok(())
}

fn manage_directories(_db: &Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}", style("📁 Directory Management").bold());
    println!(
        "{}",
        style("⚠️  Directory management not yet implemented").yellow()
    );
    println!("Use the web interface for directory configuration");

    let _: String = Input::with_theme(&theme)
        .with_prompt("Press Enter to continue")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    Ok(())
}

fn manage_users(_db: &Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}", style("👥 User Management").bold());
    println!(
        "{}",
        style("⚠️  User management not yet implemented").yellow()
    );
    println!("Use the web interface for user configuration");

    let _: String = Input::with_theme(&theme)
        .with_prompt("Press Enter to continue")
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    Ok(())
}
