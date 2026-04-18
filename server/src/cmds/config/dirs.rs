// This file is part of Moonshadow NVR, a security camera network video recorder.
// Copyright (C) 2017-2025 Moonshadow NVR Contributors.
// SPDX-License-Identifier: GPL-v3.0-or-later WITH GPL-3.0-linking-exception.

//! Interactive CLI directory and retention configuration with dialoguer.

use base::err;
use base::strutil::{decode_size, encode_size};
use base::Error;
use bpaf::Bpaf;
use console::{pad_str, Alignment};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use std::path::PathBuf;
use std::sync::Arc;

use crate::cmds::open_conn;
use crate::cmds::OpenMode;
use base::clock;

const PC_PURPLE: &str = "\x1b[38;2;191;219;216m";
const PC_GREEN: &str = "\x1b[38;2;166;227;161m";
const PC_BLUE: &str = "\x1b[38;2;137;180;250m";
const PC_PINK: &str = "\x1b[38;2;205;127;151m";
const PC_RESET: &str = "\x1b[0m";

fn pc(s: &str) -> String {
    format!("{}{}", PC_PURPLE, s)
}

fn gr(s: &str) -> String {
    format!("{}{}", PC_GREEN, s)
}

fn pk(s: &str) -> String {
    format!("{}{}", PC_PINK, s)
}

fn bl(s: &str) -> String {
    format!("{}{}", PC_BLUE, s)
}

#[derive(Clone, Debug)]
pub struct DirCard {
    pub id: i32,
    pub path: String,
    pub stream_count: usize,
}

#[derive(Bpaf, Debug)]
#[bpaf(command("dirs"))]
#[allow(dead_code)]
pub struct Args {
    #[bpaf(external(crate::parse_db_dir))]
    db_dir: PathBuf,
}

#[allow(dead_code)]
pub fn run(args: Args) -> Result<i32, Error> {
    let (_db_dir, mut conn) = open_conn(&args.db_dir, OpenMode::ReadWrite)?;

    let cur_ver = db::get_schema_version(&conn)?;
    if cur_ver.is_none() {
        println!("{}Initializing database...", pc("🗄️ "));
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

    println!("{}Moonshadow NVR - Directory Management", pc("📁 "));
    run_dir_ui(&db)?;

    Ok(0)
}

pub fn run_dir_ui(db: &Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    loop {
        print!("\x1B[2J\x1B[1;1H");

        let dirs = load_dirs(db);

        print_header();

        if dirs.is_empty() {
            print_empty_state();
        } else {
            print_table(&dirs);
        }

        println!();
        println!("{}Controls: {}q{}=quit | {}a{}=add | {}r{}=refresh | {}e{}=edit retention | {}d{}=delete | [num]=view",
            bl("► "), pk("q"), PC_RESET,
            pk("a"), PC_RESET,
            pk("r"), PC_RESET,
            pk("e"), PC_RESET,
            pk("d"), PC_RESET
        );
        println!();

        let input: String = Input::with_theme(&theme)
            .allow_empty(true)
            .with_prompt("Enter command")
            .interact_text()
            .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

        let input = input.trim().to_lowercase();

        match input.as_str() {
            "q" | "quit" => break,
            "a" | "add" => {
                add_dir_interactive(db)?;
            }
            "r" | "refresh" => {}
            "e" | "edit" => {
                if !dirs.is_empty() {
                    edit_retention_interactive(db, &dirs)?;
                }
            }
            "d" | "delete" => {
                if !dirs.is_empty() {
                    delete_dir_interactive(db, &dirs)?;
                }
            }
            _ => {
                if let Ok(num) = input.parse::<usize>() {
                    let idx = num.saturating_sub(1);
                    if idx < dirs.len() {
                        show_dir_detail(db, dirs[idx].id)?;
                    }
                }
            }
        }
    }

    println!("{}Goodbye!", gr("👋 "));
    Ok(())
}

fn load_dirs(db: &Arc<db::Database>) -> Vec<DirCard> {
    let l = db.lock();
    let dirs = l.sample_file_dirs_by_id();
    let mut result: Vec<DirCard> = Vec::new();

    for (id, dir) in dirs {
        let path = dir.pool().path().to_string_lossy().to_string();
        let stream_count = l
            .streams_by_id()
            .values()
            .filter(|stream| {
                let s_lock = stream.inner.lock();
                s_lock.sample_file_dir.as_ref().map(|d| d.id) == Some(*id)
            })
            .count();

        result.push(DirCard {
            id: *id,
            path,
            stream_count,
        });
    }

    result.sort_by(|a, b| a.id.cmp(&b.id));
    result
}

fn print_header() {
    println!(
        "{}╔══════════════════════════════════════════════════════════════════════════════╗",
        PC_PURPLE
    );
    println!(
        "║{} 📁 Moonshadow NVR - Directory Manager {} ║",
        PC_BLUE, PC_RESET
    );
    println!(
        "{}╚══════════════════════════════════════════════════════════════════════════════╝",
        PC_PURPLE
    );
    println!();
}

fn print_empty_state() {
    print!("{}", PC_PINK);
    println!(" ┌─────────────────────────────────────────────────────────────────────────────┐");
    println!(" │ 📁 No directories configured                                                │");
    print!("{}", PC_RESET);
    println!(
        " │ Press {}a{} to add your first directory                                         │",
        pk("a"),
        PC_RESET
    );
    println!(
        " │ or press {}q{} to return to the main menu                                       │",
        pk("q"),
        PC_RESET
    );
    println!(" └─────────────────────────────────────────────────────────────────────────────┘");
    print!("{}", PC_RESET);
    println!();
}

fn print_table(dirs: &[DirCard]) {
    print!("{}", PC_BLUE);
    println!(" ┌─────┬──────────────────────────────────────────────────────────┬─────────────┐");
    println!(
        " │ {:^3} │ {:^55} │ {:^8} │",
        pc("#"),
        pc("Path"),
        pc("Streams")
    );
    println!(" ├─────┼──────────────────────────────────────────────────────────┼─────────────┤");

    for (idx, dir) in dirs.iter().enumerate() {
        let path = if dir.path.len() > 53 {
            format!("...{}", &dir.path[dir.path.len() - 50..])
        } else {
            dir.path.clone()
        };
        let id_str = format!("{}", idx + 1);

        println!(
            " │ {:^3} │ {:<55} │ {:^8} │",
            bl(&id_str),
            bl(&path),
            gr(&format!("{}", dir.stream_count))
        );
    }

    println!(" └─────┴──────────────────────────────────────────────────────────┴─────────────┘");
    println!("{}", PC_RESET);
    println!();
}

/// Builds the directory list view.
fn build_dir_list_view(db: Arc<db::Database>) -> impl cursive::view::View {
    let mut layout = LinearLayout::new(Orientation::Vertical);

    // Directory list
    layout.add_child(
        SelectView::<i32>::new()
            .on_submit(move |s, dir_id: &i32| {
                show_dir_detail(s, *dir_id, db.clone());
            })
            .with_name("dir_list")
            .scrollable()
            .full_screen(),
    );

    // Action buttons
    let buttons = LinearLayout::new(Orientation::Horizontal)
        .child(cursive::views::Button::new(
            "✏️ Edit Retention",
            move |s| {
                edit_selected_dir_retention(s, db.clone());
            },
        ))
        .child(cursive::views::Button::new("🔄 Refresh", move |s| {
            refresh_dir_list(s, db.clone());
        }));

    layout.add_child(buttons);

    // Load initial data
    refresh_dir_list_impl(&mut layout, db);

    LinearLayout::new(Orientation::Vertical)
        .child(layout)
        .full_screen()
}

/// Refreshes the directory list from database.
fn refresh_dir_list(s: &mut Cursive, db: Arc<db::Database>) {
    // This function is not implemented correctly yet
    // For now, do nothing
}

fn refresh_dir_list_impl(layout: &mut LinearLayout, db: Arc<db::Database>) {
    let l = db.lock();
    let dirs = l.sample_file_dirs_by_id();

    let mut select = SelectView::<i32>::new().on_submit(move |s, dir_id: &i32| {
        show_dir_detail(s, *dir_id, db.clone());
    });

    for (id, dir) in dirs {
        let path = dir.pool().path().to_string_lossy();
        let label = format!("📁 ID {}: {}", id, path);
        select.add_item(label, *id);
    }

    if dirs.is_empty() {
        select.add_item("(No directories configured)", -1);
    }

    if let Some(mut select_view) = layout.find_name::<SelectView<i32>>("dir_list") {
        *select_view = select;
    }
    drop(l);
}

/// Gets the currently selected directory ID.
fn get_selected_dir_id(s: &mut Cursive) -> Option<i32> {
    s.find_name::<SelectView<i32>>("dir_list")
        .and_then(|select| select.selection().map(|arc| *arc))
        .filter(|id| *id >= 0)
}

/// Shows the directory detail dialog.
fn show_dir_detail(s: &mut Cursive, dir_id: i32, db: Arc<db::Database>) {
    if dir_id < 0 {
        return;
    }

    let l = db.lock();
    let dir = match l.sample_file_dirs_by_id().get(&dir_id) {
        Some(d) => d,
        None => return,
    };

    let path = dir.pool().path().to_string_lossy().into_owned();

    // Get streams using this directory
    let mut streams_info = Vec::new();
    for stream in l.streams_by_id().values() {
        let s_lock = stream.inner.lock();
        if s_lock.sample_file_dir.as_ref().map(|d| d.id) == Some(dir_id) {
            let camera_name = l
                .cameras_by_id()
                .get(&s_lock.camera_id)
                .map(|c| c.short_name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            streams_info.push((
                s_lock.id,
                format!("{} - {}", camera_name, s_lock.type_.as_str()),
                s_lock.config.retain_bytes,
            ));
        }
    }
    drop(l);

    let mut layout = LinearLayout::new(Orientation::Vertical);

    layout.add_child(TextView::new(format!("📁 Directory ID: {}", dir_id)));
    layout.add_child(TextView::new(format!("Path: {}", path)));
    layout.add_child(TextView::new(""));
    layout.add_child(TextView::new("=== Associated Streams & Retention ==="));

    for (stream_id, name, limit) in &streams_info {
        let limit_str = encode_size(*limit);
        let entry = format!("🎥 {} (ID: {})\n   Limit: {}", name, stream_id, limit_str);
        layout.add_child(TextView::new(entry));
        layout.add_child(cursive::views::Button::new("Edit", move |s| {
            show_retention_edit_dialog(s.clone(), *stream_id, &db);
        }));
    }

    if streams_info.is_empty() {
        layout.add_child(TextView::new(
            "\n(No streams associated with this directory)",
        ));
    }

    let dialog = Dialog::new()
        .title("Directory Details")
        .content(layout)
        .button("Close", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

/// Edits retention for the selected directory.
fn edit_selected_dir_retention(s: &mut Cursive, db: Arc<db::Database>) {
    if let Some(dir_id) = get_selected_dir_id(s) {
        show_dir_retention_for_dir(s, dir_id, db);
    }
}

/// Shows retention editing for a specific directory.
fn show_dir_retention_for_dir(s: &mut Cursive, dir_id: i32, db: Arc<db::Database>) {
    let l = db.lock();
    let mut streams_info = Vec::new();

    for stream in l.streams_by_id().values() {
        let s_lock = stream.inner.lock();
        if s_lock.sample_file_dir.as_ref().map(|d| d.id) == Some(dir_id) {
            let camera_name = l
                .cameras_by_id()
                .get(&s_lock.camera_id)
                .map(|c| c.short_name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            streams_info.push((
                s_lock.id,
                format!("{} - {}", camera_name, s_lock.type_.as_str()),
                s_lock.config.retain_bytes,
            ));
        }
    }
    drop(l);

    if streams_info.is_empty() {
        let dialog = Dialog::new()
            .title("ℹ️ Information")
            .content(TextView::new("No streams associated with this directory."))
            .button("OK", |s| {
                s.pop_layer();
            });
        s.add_layer(dialog);
        return;
    }

    let mut layout = LinearLayout::new(Orientation::Vertical);
    layout.add_child(TextView::new("Select a stream to edit retention:\n"));

    let mut select = SelectView::<(i32, usize)>::new();
    for (i, (stream_id, name, limit)) in streams_info.iter().enumerate() {
        let label = format!("{}: {}", name, encode_size(*limit));
        select.add_item(label, (*stream_id, i));
    }

    let db_clone = db.clone();
    select.set_on_submit(move |s, item: &(i32, usize)| {
        let (stream_id, _) = *item;
        show_retention_edit_dialog(s, stream_id, &db_clone);
    });

    layout.add_child(select);
    layout.add_child(cursive::views::Button::new("Back", |s| {
        s.pop_layer();
    }));

    let dialog = Dialog::new()
        .title("Edit Retention Limits")
        .content(layout)
        .button("Close", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

/// Shows the retention edit dialog for a specific stream.
fn show_retention_edit_dialog(s: &mut Cursive, stream_id: i32, db: &Arc<db::Database>) {
    let l = db.lock();
    let stream = match l.streams_by_id().get(&stream_id) {
        Some(s) => s,
        None => return,
    };

    let s_lock = stream.inner.lock();
    let current_limit = s_lock.config.retain_bytes;
    let camera_name = l
        .cameras_by_id()
        .get(&s_lock.camera_id)
        .map(|c| c.short_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    drop(l);

    let current_limit_str = encode_size(current_limit);

    let mut layout = LinearLayout::new(Orientation::Vertical);
    layout.add_child(TextView::new(format!(
        "Stream: {} - {}",
        camera_name, stream_id
    )));
    layout.add_child(TextView::new(format!(
        "Current Limit: {}",
        current_limit_str
    )));
    layout.add_child(TextView::new(""));
    layout.add_child(TextView::new("Examples: 100GB, 1TB, 500GB"));

    let limit_edit = EditView::new()
        .content(&current_limit_str)
        .with_name("retention_limit");

    layout.add_child(limit_edit);

    let db_clone = db.clone();
    let dialog = Dialog::new()
        .title("Edit Retention Limit")
        .content(layout)
        .button("💾 Save", move |s| {
            if let Some(edit_view) = s.find_name::<EditView>("retention_limit") {
                let new_limit_str = edit_view.get_content().to_string();

                match decode_size(&new_limit_str) {
                    Ok(new_limit) => {
                        let change = db::RetentionChange {
                            stream_id,
                            new_record: true,
                            new_limit,
                        };

                        match db_clone.lock().update_retention(&[change]) {
                            Ok(_) => {
                                println!("{}", style("Retention limit updated.").green());
                                s.pop_layer();
                            }
                            Err(e) => {
                                let error_dialog = Dialog::new()
                                    .title("❌ Error")
                                    .content(TextView::new(format!("Failed to update: {}", e)))
                                    .button("OK", |s| {
                                        s.pop_layer();
                                    });
                                s.add_layer(error_dialog);
                            }
                        }
                    }
                    Err(e) => {
                        let error_dialog = Dialog::new()
                            .title("❌ Validation Error")
                            .content(TextView::new("Invalid size format"))
                            .button("OK", |s| {
                                s.pop_layer();
                            });
                        s.add_layer(error_dialog);
                    }
                }
            }
        })
        .button("Cancel", |s| {
            s.pop_layer();
        });

    s.add_layer(dialog);
}

fn show_dir_detail(db: &Arc<db::Database>, dir_id: i32) -> Result<(), Error> {
    let l = db.lock();
    let dir = match l.sample_file_dirs_by_id().get(&dir_id) {
        Some(d) => d,
        None => {
            println!("{}Directory not found!", pk("⚠️ "));
            return Ok(());
        }
    };

    let path = dir.pool().path().to_string_lossy().into_owned();

    let mut streams_info = Vec::new();
    for stream in l.streams_by_id().values() {
        let s_lock = stream.inner.lock();
        if s_lock.sample_file_dir.as_ref().map(|d| d.id) == Some(dir_id) {
            let camera_name = l
                .cameras_by_id()
                .get(&s_lock.camera_id)
                .map(|c| c.short_name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            streams_info.push((
                s_lock.id,
                format!("{} - {}", camera_name, s_lock.type_.as_str()),
                s_lock.config.retain_bytes,
            ));
        }
    }
    drop(l);

    println!();
    println!("{}Directory Details: {}", pc("📁 "), bl(&path));
    println!("{}ID: {}", pc("🆔 "), bl(&dir_id.to_string()));
    println!(
        "{}Streams: {}",
        pc("📡 "),
        gr(&format!("{}", streams_info.len()))
    );
    println!();

    if !streams_info.is_empty() {
        println!("{}Associated Streams:", pc("🎥 "));
        for (stream_id, name, limit) in &streams_info {
            let limit_str = encode_size(*limit);
            println!(
                "  {} (ID: {}) - Limit: {}",
                bl(name),
                stream_id,
                gr(&limit_str)
            );
        }
    } else {
        println!("{}(No streams associated with this directory)", pk("⚠️ "));
    }

    println!();
    println!(
        "{}Controls: {}e{}=edit retention | {}d{}=delete | {}q{}=back",
        bl("► "),
        pk("e"),
        PC_RESET,
        pk("d"),
        PC_RESET,
        pk("q"),
        PC_RESET
    );
    println!();

    let theme = ColorfulTheme::default();
    loop {
        let input: String = Input::with_theme(&theme)
            .allow_empty(true)
            .with_prompt("Enter command")
            .interact_text()
            .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

        match input.trim() {
            "e" | "edit" => {
                if !streams_info.is_empty() {
                    edit_retention_for_stream(db, streams_info[0].0)?;
                }
                break;
            }
            "d" | "delete" => {
                delete_dir(db, dir_id)?;
                break;
            }
            "q" | "quit" | "" => break,
            _ => {}
        }
    }

    Ok(())
}

fn add_dir_interactive(db: &Arc<db::Database>) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}Add New Directory", pc("➕ "));

    let path: String = Input::with_theme(&theme)
        .with_prompt("Directory path (required)")
        .allow_empty(false)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    let confirm = Confirm::with_theme(&theme)
        .with_prompt("Save directory?")
        .default(true)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if !confirm {
        println!("{}Cancelled.", pk("⚠️ "));
        return Ok(());
    }

    let mut l = db.lock();
    match l.add_sample_file_dir(&path) {
        Ok(id) => {
            println!(
                "{}Directory added successfully! (ID: {})",
                gr("✅ "),
                bl(&id.to_string())
            );
        }
        Err(e) => {
            println!("{}Failed to add directory: {}", pk("❌ "), e);
        }
    }
    drop(l);

    Ok(())
}

fn edit_retention_interactive(db: &Arc<db::Database>, dirs: &[DirCard]) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}Select directory to edit retention:", pc("✏️ "));

    let mut options: Vec<String> = dirs
        .iter()
        .map(|d| format!("📁 {} ({} streams)", d.path, d.stream_count))
        .collect();
    options.push("↩️ Back".to_string());

    let selection = Select::with_theme(&theme)
        .with_prompt("Select directory")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if selection == options.len() - 1 {
        return Ok(());
    }

    let dir_id = dirs[selection].id;
    show_dir_detail(db, dir_id)?;

    Ok(())
}

fn delete_dir_interactive(db: &Arc<db::Database>, dirs: &[DirCard]) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    println!();
    println!("{}Select directory to delete:", pk("🗑️ "));

    let mut options: Vec<String> = dirs
        .iter()
        .map(|d| format!("📁 {} ({} streams)", d.path, d.stream_count))
        .collect();
    options.push("↩️ Back".to_string());

    let selection = Select::with_theme(&theme)
        .with_prompt("Select directory")
        .items(&options)
        .default(0)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if selection == options.len() - 1 {
        return Ok(());
    }

    let dir_id = dirs[selection].id;
    delete_dir(db, dir_id)?;

    Ok(())
}

fn delete_dir(db: &Arc<db::Database>, dir_id: i32) -> Result<(), Error> {
    let theme = ColorfulTheme::default();

    let confirm = Confirm::with_theme(&theme)
        .with_prompt(format!(
            "Delete directory ID {}? This cannot be undone!",
            dir_id
        ))
        .default(false)
        .interact()
        .map_err(|e| err!(InvalidArgument, msg("Dialog error: {}", e)))?;

    if !confirm {
        println!("{}Cancelled.", pk("⚠️ "));
        return Ok(());
    }

    let mut l = db.lock();
    match l.delete_sample_file_dir(dir_id) {
        Ok(_) => {
            println!("{}Directory deleted successfully!", gr("✅ "));
        }
        Err(e) => {
            println!("{}Failed to delete directory: {}", pk("❌ "), e);
        }
    }
    drop(l);

    Ok(())
}

fn edit_retention_for_stream(db: &Arc<db::Database>, stream_id: i32) -> Result<(), Error> {
    let theme = ColorfulTheme::default();
    let l = db.lock();
    let stream = match l.streams_by_id().get(&stream_id) {
        Some(s) => s,
        None => return Ok(()),
    };

    let s_lock = stream.inner.lock();
    let current_limit = s_lock.config.retain_bytes;
    let camera_name = l
        .cameras_by_id()
        .get(&s_lock.camera_id)
        .map(|c| c.short_name.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    drop(l);

    let current_limit_str = encode_size(current_limit);

    println!();
    println!("{}Edit Retention Limit", pc("✏️ "));
    println!("Stream: {} - {}", bl(&camera_name), stream_id);
    println!("Current Limit: {}", gr(&current_limit_str));
    println!("Examples: 100GB, 1TB, 500GB");
    println!();

    let new_limit: String = Input::with_theme(&theme)
        .with_prompt("New limit")
        .default(current_limit_str.clone())
        .allow_empty(true)
        .interact_text()
        .map_err(|e| err!(InvalidArgument, msg("Input error: {}", e)))?;

    match decode_size(&new_limit) {
        Ok(new_limit_val) => {
            let change = db::RetentionChange {
                stream_id,
                new_record: true,
                new_limit: new_limit_val,
            };

            match db.lock().update_retention(&[change]) {
                Ok(_) => {
                    println!("{}Retention limit updated.", gr("✅ "));
                }
                Err(e) => {
                    println!("{}Failed to update: {}", pk("❌ "), e);
                }
            }
        }
        Err(e) => {
            println!("{}Invalid size format: {}", pk("❌ "), e);
        }
    }

    Ok(())
}

pub fn run_wizard(db: &Arc<db::Database>) -> Result<(), Error> {
    run_dir_ui(db)
}
